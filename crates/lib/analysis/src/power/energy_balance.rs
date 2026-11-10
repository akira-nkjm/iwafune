//! 電力収支と DoD（`power.energy_balance`）。
//!
//! 軌道と姿勢則の時系列に沿った発電（[`super::solar`]）と、ケースごとの負荷（モードの平均消費電力
//! [`super::mode_load::evaluate_mode`] か一定値）から、電池の充放電（[`super::battery`]）を積分する。
//! 日照中（[`crate::spec::is_sunlit`]）と日陰中で別のモードを使える。

pub use super::battery::EnergyBalanceBattery;
use super::mode_load::ModeLoadInput;
use super::solar::{SolarArray, SolarLife, SolarProfile};
use crate::spec::{
    Analysis, AnalysisError, AnalysisKind, Channel, ChannelKind, Check, Series, SpanFallback,
    System, orbit_mean, orbit_ranges, span_fallback, time_fraction,
};
use sat_astro::attitude::AttitudeLaw;
use sat_astro::timeline::{self, TimelineSpec};
use sat_units::typed::{ElectricCurrent, Energy, Fraction, Power, Qty, Time};
use serde::{Deserialize, Serialize};
use std::ops::Range;

/// 電力収支と DoD の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EnergyBalanceInput {
    pub timeline: TimelineSpec,
    pub array: SolarArray,
    /// 発電を評価する寿命の時点（省略時 EOL）
    #[serde(default)]
    pub life: SolarLife,
    /// 個別電池（空はエラー）。容量合計・先頭電圧/効率の規則は combine。
    pub batteries: Vec<EnergyBalanceBattery>,
    /// 部品とモード（`power.mode_load` と同じ入力）。モードで負荷を指定するケースが使う
    #[serde(default)]
    pub loads: ModeLoadInput,
    /// モードの消費電力に成長余裕を含めるか（省略時は含める）
    #[serde(default)]
    pub load_growth: LoadGrowth,
    pub cases: Vec<EnergyBalanceCase>,
}

/// モードの消費電力に成長余裕を含めるか。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum LoadGrowth {
    /// 成長余裕込みの消費電力（`total_with_growth`）
    #[default]
    Included,
    /// 成長余裕を含めない消費電力（`total`）
    Excluded,
}

/// 評価するケース（負荷のかけ方）1 つ。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EnergyBalanceCase {
    pub id: String,
    #[serde(default)]
    pub label: String,
    pub load: EnergyBalanceLoad,
    /// このケースの姿勢則（モードごとに姿勢が違うとき）。省略時は `timeline.attitude`。
    /// 与えると、軌道・期間・刻み・β はそのままで姿勢則だけを替えた時系列で発電を求める
    #[serde(default)]
    pub attitude: Option<AttitudeLaw>,
}

/// 負荷のかけ方。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EnergyBalanceLoad {
    /// 1 つのモードの平均消費電力を常にかける
    Mode { mode: String },
    /// 日照中と日陰中で別のモード
    SunlitEclipse {
        sunlit_mode: String,
        eclipse_mode: String,
    },
    /// 一定の負荷（モード表を使わない検討用）
    Constant { power: Qty<Power> },
}

/// 電力収支と DoD の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EnergyBalanceOutput {
    pub life: SolarLife,
    pub period: Qty<Time>,
    /// 回り切った周回の数（0 なら平均は全期間）
    pub complete_orbits: usize,
    /// 1 周に満たないときの代わりの集計（回り切った周回があれば `None`）
    pub span_fallback: Option<SpanFallback>,
    /// 軌道平均の発電（`timeline.attitude` の姿勢則。ケースで姿勢則を替えたときはケースの値を見る）
    pub orbit_average_generation: Qty<Power>,
    /// 日照の割合（0〜1）
    pub sunlit_fraction: Fraction,
    /// 時系列（`timeline.attitude` の姿勢則）: `generation`・`sunlit`
    pub series: Series,
    pub cases: Vec<EnergyBalanceCaseResult>,
}

/// 1 ケースの結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EnergyBalanceCaseResult {
    pub id: String,
    pub label: String,
    /// 日照中の負荷
    pub load_sunlit: Qty<Power>,
    /// 日陰中の負荷
    pub load_eclipse: Qty<Power>,
    /// 軌道平均の発電（このケースの姿勢則で）
    pub orbit_average_generation: Qty<Power>,
    /// 軌道平均の負荷
    pub orbit_average_load: Qty<Power>,
    /// 軌道平均の発電 − 負荷（電池の効率を含まない）
    pub orbit_average_balance: Qty<Power>,
    /// 1 周あたりのエネルギー余裕（電池の効率込み、容量の上下限で切る前）。周回のうち最小のもの
    pub energy_margin_per_orbit: Qty<Energy>,
    pub min_soc: Fraction,
    /// 最大の DoD（1 − 最小 SOC）
    pub max_dod: Fraction,
    /// 1 回の日陰で減る SOC の最大（日陰に入る直前の SOC − 日陰中の最小 SOC）
    pub max_eclipse_dod: Fraction,
    pub final_soc: Fraction,
    pub peak_charge_power: Qty<Power>,
    pub peak_discharge_power: Qty<Power>,
    /// 最大の放電電流（最大放電電力 / 公称電圧）
    pub peak_discharge_current: Qty<ElectricCurrent>,
    /// 回り切った周回ごとの集計
    pub orbits: Vec<EnergyBalanceOrbit>,
    /// 時系列: `generation`（このケースの姿勢則）・`load`・`soc`・`charge_power`・`discharge_power`・`shunt_power`
    pub series: Series,
    /// DoD の上限・周回の収支・電池切れ・負荷の不足の判定
    pub checks: Vec<Check>,
}

/// 1 周の集計。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EnergyBalanceOrbit {
    /// 0 から数えた周回の番号
    pub index: usize,
    pub start: Qty<Time>,
    /// 発電量
    pub generation: Qty<Energy>,
    /// 消費電力量
    pub load: Qty<Energy>,
    /// エネルギー余裕（電池の効率込み、上下限で切る前）
    pub margin: Qty<Energy>,
    pub min_soc: Fraction,
    pub end_soc: Fraction,
}

/// 電力収支と DoD。
pub struct EnergyBalance;

impl Analysis for EnergyBalance {
    const ID: &'static str = "power.energy_balance";
    const TITLE: &'static str = "電力収支と DoD";
    const SYSTEM: System = System::Power;
    const KIND: AnalysisKind = AnalysisKind::TimeSeries;
    const DESCRIPTION: &'static str = "モードごとの軌道平均収支・充放電・日陰での DoD・SOC の推移";
    const TITLE_EN: &'static str = "Energy balance and DoD";
    const DESCRIPTION_EN: &'static str = "Orbit-average balance per mode, charge/discharge, eclipse depth of discharge and SOC history";
    type Input = EnergyBalanceInput;
    type Output = EnergyBalanceOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        let battery = EnergyBalanceBattery::combine(&input.batteries)?;
        input.array.validate()?;
        super::mode_load::validate(&input.loads)?;
        let period_s = input.timeline.orbit().period_s();
        let ctx = Context::new(input, &battery, profile(input, &input.timeline)?, period_s);
        let mut cases = Vec::with_capacity(input.cases.len());
        for c in &input.cases {
            let result = match &c.attitude {
                None => evaluate_case(&ctx, c)?,
                Some(law) => {
                    let mut spec = input.timeline.clone();
                    spec.attitude = law.clone();
                    evaluate_case(
                        &Context::new(input, &battery, profile(input, &spec)?, period_s),
                        c,
                    )?
                }
            };
            cases.push(result);
        }
        Ok(ctx.output(cases))
    }
}

/// 時系列に沿った発電。
fn profile(input: &EnergyBalanceInput, spec: &TimelineSpec) -> Result<SolarProfile, AnalysisError> {
    Ok(SolarProfile::build(&input.array, &timeline::build(spec)?))
}

/// 1 つの姿勢則での時系列と集計（ケースが姿勢則を替えなければ全ケースで共通）。
struct Context<'a> {
    input: &'a EnergyBalanceInput,
    battery: &'a EnergyBalanceBattery,
    profile: SolarProfile,
    generation: Vec<f64>,
    ranges: Vec<Range<usize>>,
    period_s: f64,
}

impl<'a> Context<'a> {
    fn new(
        input: &'a EnergyBalanceInput,
        battery: &'a EnergyBalanceBattery,
        profile: SolarProfile,
        period_s: f64,
    ) -> Self {
        Self {
            generation: profile.total(input.life),
            ranges: orbit_ranges(&profile.t, period_s),
            input,
            battery,
            profile,
            period_s,
        }
    }

    fn output(self, cases: Vec<EnergyBalanceCaseResult>) -> EnergyBalanceOutput {
        EnergyBalanceOutput {
            life: self.input.life,
            period: Qty::from_raw(self.period_s),
            complete_orbits: self.ranges.len(),
            span_fallback: span_fallback(self.ranges.len(), SpanFallback::OrbitMeanOverSpan),
            orbit_average_generation: Qty::from_raw(orbit_mean(
                &self.profile.t,
                &self.generation,
                &self.ranges,
            )),
            sunlit_fraction: time_fraction(&self.profile.t, &self.profile.lit, &self.ranges),
            series: Series {
                channels: vec![
                    Channel::of::<Power>("generation", ChannelKind::Generation, self.generation),
                    Channel::of::<Fraction>("sunlit", ChannelKind::Sunlit, self.profile.sunlit),
                ],
                t: self.profile.t,
            },
            cases,
        }
    }
}

mod case;
use case::evaluate_case;

#[cfg(test)]
mod tests;
