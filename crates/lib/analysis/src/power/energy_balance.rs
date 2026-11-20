//! 電力収支と DoD（`power.energy_balance`）。
//!
//! 軌道と姿勢則の時系列に沿った発電（[`super::solar`]）と、ケースごとの負荷（モードの平均消費電力
//! [`super::mode_load::evaluate_mode`] か一定値）から、電池の充放電（[`super::battery`]）を積分する。
//! 日照中（[`crate::spec::is_sunlit`]）と日陰中で別のモードを使える。
//!
//! - ケースは既定で解析期間ずっと続けたとして評価する（[`case`]）。
//! - 最長継続時間を持つモードのケースは、基本のモードの上でその時間だけ続けたときの最悪で判定し直す（[`duration`]）。
//! - 最長継続時間の評価の開始は、モードの使う時間帯（`when`）・開始の基準（`start`）に従って選ぶ（[`timing`]。指定が無ければ周回内の最悪）。
//! - モードの並びがあれば、並びに沿って負荷と姿勢則を切り替えた収支を別に出す（[`sequence`]。並びの先頭は食の出入りに合わせられる）。
//! - 発電に倍率をかけて同じ運用を繰り返し評価する入口（[`ScaledBalance`]）を `power.sizing` の時系列の逆算が使う。
//! - duty（状態の時間割合）はどれも平均の消費電力として一定にかける（周回内の時間分布は扱わない。結果の `duty` に書く）。

pub use super::battery::EnergyBalanceBattery;
use super::mode_load::ModeLoadInput;
use super::solar::{SolarArray, SolarLife, SolarProfile};
use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Channel, ChannelKind, Check, Series,
    SpanFallback, System, orbit_mean, span_fallback, time_fraction,
};
use sat_astro::attitude::AttitudeLaw;
use sat_astro::timeline::TimelineSpec;
use sat_units::typed::{ElectricCurrent, Energy, Fraction, Power, Qty, Time};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 電力収支と DoD の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EnergyBalanceInput {
    pub timeline: TimelineSpec,
    pub array: SolarArray,
    /// 発電を評価する寿命の時点（省略時 EOL。[`SolarLife::default`]）
    #[serde(default)]
    pub life: Option<SolarLife>,
    /// 個別電池（空はエラー）。容量合計・先頭電圧/効率の規則は combine。
    pub batteries: Vec<EnergyBalanceBattery>,
    /// 部品とモード（`power.mode_load` と同じ入力）。モードで負荷を指定するケースが使う
    #[serde(default)]
    pub loads: ModeLoadInput,
    /// モードの消費電力に成長余裕を含めるか（省略時は含める。[`LoadGrowth::default`]）
    #[serde(default)]
    pub load_growth: Option<LoadGrowth>,
    pub cases: Vec<EnergyBalanceCase>,
    /// モードの並び（1 周期分。繰り返して解析期間を覆う）。空なら並びの収支を出さない
    #[serde(default)]
    pub sequence: Vec<EnergyBalanceSegment>,
    /// 並びの名前と、日照・食に対する先頭の位置（[`timing`]）。省略時は無名で、解析の始めから数える
    #[serde(default)]
    pub sequence_timing: SequenceTiming,
    /// 最長継続時間の評価で下に敷く基本のモード。省略時は並びの最初の区間のモード（並びも無ければ連続で評価する）
    #[serde(default)]
    pub base_mode: Option<String>,
    /// モードごとの姿勢則（並びと基本のモードの発電に使う）。無いモードは `timeline.attitude`
    #[serde(default)]
    pub mode_attitudes: BTreeMap<String, AttitudeLaw>,
}

impl EnergyBalanceInput {
    /// 発電を評価する寿命の時点（省略時は [`SolarLife::default`]）。出力の `life` に載る。
    pub fn life(&self) -> SolarLife {
        self.life.unwrap_or_default()
    }

    /// 成長余裕の扱い（省略時は [`LoadGrowth::default`] = 含める）。
    pub fn load_growth(&self) -> LoadGrowth {
        self.load_growth.unwrap_or_default()
    }
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
    /// モードの最長継続時間（負荷が 1 つのモードのときだけ使う）。与えると判定は基本のモードの上で
    /// この時間だけ続けたときの最悪で行う（[`EnergyBalanceCaseResult::by_max_duration`]）。省略時は連続で評価
    #[serde(default)]
    pub max_duration: Option<Qty<Time>>,
    /// 最長継続時間の評価での時刻の指定（使う時間帯・開始の基準）。無ければ開始を 1 周の中でずらして最悪を取る
    #[serde(default)]
    pub timing: ModeTiming,
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
    /// モード別の負荷内訳（電源のモードの見方でも共用）。
    pub mode_loads: super::mode_load::ModeLoadOutput,
    /// 集中電池モデルで使った DoD 上限。
    pub dod_limit: Fraction,
    /// 許容する SOC の下限。
    pub min_allowed_soc: Fraction,
    pub period: Qty<Time>,
    /// 回り切った周回の数（0 なら平均は全期間）
    pub complete_orbits: usize,
    /// 1 周に満たないときの代わりの集計（回り切った周回があれば `None`）
    pub span_fallback: Option<SpanFallback>,
    /// 軌道平均の発電（`timeline.attitude` の姿勢則。ケースで姿勢則を替えたときはケースの値を見る）
    pub orbit_average_generation: Qty<Power>,
    /// 日照の割合（0〜1）
    pub sunlit_fraction: Fraction,
    /// 共通の影の判定による蝕区間。
    pub eclipses: Vec<sat_astro::timeline::Window>,
    /// 時系列（`timeline.attitude` の姿勢則）: `generation`・`sunlit`
    pub series: Series,
    /// モードごとのケース。`max_dod` などの欄は解析期間ずっと続けたときの値、判定は `evaluation` の運用で
    pub cases: Vec<EnergyBalanceCaseResult>,
    /// モードの並びで回した収支（並びが無ければ None）
    #[serde(default)]
    pub sequence: Option<EnergyBalanceSequence>,
    /// 結果全体にかかる判定（電池の値の不一致。ケースと並びの判定はそれぞれの `checks`）
    #[serde(default)]
    pub checks: Vec<Check>,
}

/// ケースの判定（`checks` の DoD・周回の収支・電池切れ）をどの運用で評価したか。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum EnergyBalanceEvaluation {
    /// 解析期間ずっと続けたとき（最長継続時間が無い・一定の負荷・日照と日陰で別のモード）
    #[default]
    Continuous,
    /// 最長継続時間はあるが基本のモード（設定・並び）が無いので、解析期間ずっと続けたとき
    NoBaseMode,
    /// このモードが基本のモードなので、解析期間ずっと続けたとき
    BaseMode,
    /// 基本のモードの上で最長継続時間だけ続けたときの最悪（`by_max_duration`）
    MaxDuration,
    /// モードの並びに沿って回したとき
    Sequence,
}

/// duty（状態の時間割合）の扱い。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum EnergyBalanceDuty {
    /// 使うモードに duty が無い
    #[default]
    NotUsed,
    /// duty を平均の消費電力にして一定にかけた（周回内の時間分布は扱わない）
    Averaged,
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
    /// このケースの姿勢則での日照中の平均発電。
    pub sunlit_average_generation: Qty<Power>,
    /// 日照中のバス側平均充電電力。
    pub sunlit_average_charge: Qty<Power>,
    /// 日照中のバス側平均放電電力。
    pub sunlit_average_discharge: Qty<Power>,
    /// 日照中のバス側平均シャント電力。
    pub sunlit_average_shunt: Qty<Power>,
    /// 軌道平均の負荷。
    pub orbit_average_load: Qty<Power>,
    /// 軌道平均の発電 − 負荷（電池の効率を含まない）
    pub orbit_average_balance: Qty<Power>,
    /// 1 周あたりのエネルギー余裕（電池の効率込み、容量の上下限で切る前）。周回のうち最小のもの。回り切った周回が無ければ None
    pub energy_margin_per_orbit: Option<Qty<Energy>>,
    /// 解析期間全体の収支（電池の効率込み、容量の上下限で切る前、周期への換算なし）。回り切った周回が無いときだけ Some
    pub energy_margin_over_span: Option<Qty<Energy>>,
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
    /// DoD の上限・周回の収支・電池切れ・負荷の不足の判定（DoD・収支・電池切れは `evaluation` の運用で）
    pub checks: Vec<Check>,
    /// 判定をどの運用で評価したか
    #[serde(default)]
    pub evaluation: EnergyBalanceEvaluation,
    /// 最長継続時間で評価した結果（`evaluation` が `max_duration` のときだけ）。上の欄は連続で回した値のまま
    #[serde(default)]
    pub by_max_duration: Option<EnergyBalanceDuration>,
    /// duty の扱い
    #[serde(default)]
    pub duty: EnergyBalanceDuty,
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
    const USES: &'static [&'static str] = &[super::mode_load::ModeLoad::ID];
    const TITLE: &'static str = "電力収支と DoD";
    const AREA: AnalysisArea = AnalysisArea::System(System::Power);
    const KIND: AnalysisKind = AnalysisKind::TimeSeries;
    const DESCRIPTION: &'static str = "モードごとの軌道平均収支・充放電・日陰での DoD・SOC の推移";
    const TITLE_EN: &'static str = "Energy balance and DoD";
    const DESCRIPTION_EN: &'static str = "Orbit-average balance per mode, charge/discharge, eclipse depth of discharge and SOC history";
    type Input = EnergyBalanceInput;
    type Output = EnergyBalanceOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        let combined = EnergyBalanceBattery::combine(&input.batteries)?;
        let mismatch = combined.mismatch_check();
        let battery = combined.battery;
        input.array.validate()?;
        super::mode_load::validate(&input.loads)?;
        sequence::validate(input)?;
        let period_s = input.timeline.orbit().period_s();
        let main = profile(input, &input.timeline)?;
        let ctx = Context::new(input, &battery, &main, period_s);
        let base = duration::BaseMode::new(&ctx)?;
        let mut cases = Vec::with_capacity(input.cases.len());
        for c in &input.cases {
            let own = match &c.attitude {
                None => None,
                Some(law) => Some(profile(input, &with_law(input, law))?),
            };
            let own_ctx = own
                .as_ref()
                .map(|p| Context::new(input, &battery, p, period_s));
            let case_ctx = own_ctx.as_ref().unwrap_or(&ctx);
            let (mut result, _) = evaluate_case(case_ctx, c)?;
            duration::apply(case_ctx, c, base.as_ref(), &mut result)?;
            cases.push(result);
        }
        let sequence = sequence::evaluate(&ctx)?.map(|(s, _)| s);
        let mut out = ctx.output(
            cases,
            sequence,
            super::mode_load::ModeLoad::run(&input.loads)?,
        );
        // 電池の値の不一致は結果のどの欄にも効くので、ケースごとではなく出力の最上位に 1 つだけ載せる
        out.checks.extend(mismatch);
        Ok(out)
    }
}

impl Context<'_> {
    pub(super) fn output(
        self,
        cases: Vec<EnergyBalanceCaseResult>,
        sequence: Option<EnergyBalanceSequence>,
        mode_loads: crate::power::mode_load::ModeLoadOutput,
    ) -> EnergyBalanceOutput {
        let p = self.profile;
        EnergyBalanceOutput {
            life: self.input.life(),
            mode_loads,
            dod_limit: Fraction::new(self.battery.dod_limit),
            min_allowed_soc: Fraction::new(1.0 - self.battery.dod_limit),
            period: Qty::from_raw(self.period_s),
            complete_orbits: self.ranges.len(),
            span_fallback: span_fallback(self.ranges.len(), SpanFallback::OrbitMeanOverSpan),
            orbit_average_generation: Qty::from_raw(orbit_mean(
                &p.t,
                &self.generation,
                &self.ranges,
            )),
            eclipses: p.eclipses.clone(),
            sunlit_fraction: time_fraction(&p.t, &p.lit, &self.ranges),
            series: Series {
                channels: vec![
                    Channel::of::<Power>("generation", ChannelKind::Generation, self.generation),
                    Channel::of::<Fraction>("sunlit", ChannelKind::Sunlit, p.sunlit.clone()),
                ],
                t: p.t.clone(),
            },
            cases,
            sequence,
            checks: Vec::new(),
        }
    }
}

mod case;
mod context;
mod duration;
mod scaled;
mod sequence;
mod timing;
mod verdict;
use case::evaluate_case;
use context::{Context, profile, with_law};
pub use duration::EnergyBalanceDuration;
pub use scaled::{ScaledBalance, ScaledJudgement};
pub use sequence::{EnergyBalanceSegment, EnergyBalanceSequence};
pub use timing::{
    DurationTiming, EclipseEvent, EventStart, ModeTiming, SequenceTiming, TimeWindow,
};

#[cfg(test)]
mod tests;
