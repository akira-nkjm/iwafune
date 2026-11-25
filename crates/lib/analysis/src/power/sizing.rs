//! 電源の逆算サイジング。季節の蝕・モード負荷・発電モデル・電力収支の正本を共有する。
//!
//! - SMAD の式（[`calculate::smad_requirements`]）: 手計算と比べられる参考値（`smad`・`modes` の SMAD の欄）。
//! - 時系列の逆算（[`series`]）: 電力収支と同じ運用で 1 周の収支 ≥ 0 の最小面積と、DoD が上限に収まる最小容量。
//!   KPI と判定（`sap_margin`・`battery_margin`・`mode_checks`）はこちらで行う（`basis`）。判定できなければ SMAD で行う。
use super::battery::EnergyBalanceBattery;
use super::energy_balance::{EnergyBalanceSegment, ModeTiming, SequenceTiming};
use super::mode_load::{ModeLoad, ModeLoadInput, ModeLoadOutput};
use super::solar::{SolarArray, SolarPanel};
use crate::orbit::{
    EclipseSeason, EclipseSeasonInput, EclipseSeasonPoint, sun_distance_extremes_over,
};
use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Check, System, ensure_fraction,
    ensure_non_negative, ensure_positive, quantity_text, ratio_margin_verdict,
};
use sat_astro::attitude::AttitudeLaw;
use sat_astro::orbit::Trajectory;
use sat_astro::time::Epoch;
use sat_astro::timeline::TimelineSpec;
use sat_units::typed::{
    Area, ElectricCurrent, Energy, Fraction, Length, Power, Qty, Time, Voltage,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 直並列数を出すためのセルの電圧・電流（パネルごと）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SizingCell {
    /// 対象のパネル（`array.panels` の ID）
    pub panel: String,
    pub voltage: Qty<Voltage>,
    pub current: Qty<ElectricCurrent>,
}

/// 日照経路の効率 Xd の既定値（SAP からバスまでの経路。SMAD 型の必要発電の式 `(Pe·Te/Xe + Pd·Td/Xd)/Td` の係数）。
/// 教科書的な概算値。経路の方式（直接エネルギー転送か最大電力点追従か）は仮定せず、
/// 設計の値（`analysis.power.conversion_efficiency` など）があればそれを使う。
pub const DEFAULT_SUNLIT_EFFICIENCY: f64 = 0.8;

/// 蝕経路の効率 Xe の既定値（SAP から電池を通って蝕中の負荷に届くまでの経路全体。同じ式の係数）。
/// 教科書的な概算値。日照経路の効率と同様に、設計の値があればそれを使う。
pub const DEFAULT_ECLIPSE_EFFICIENCY: f64 = 0.6;

/// 任意のモデル上書き。省略値の正本は `used`。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SizingOptions {
    pub sunlit_efficiency: Option<f64>,
    pub eclipse_efficiency: Option<f64>,
    pub battery_efficiency: Option<f64>,
    pub annual_degradation: Option<f64>,
    pub inherent_degradation: Option<f64>,
    pub battery_strings: Option<u32>,
    pub bus_voltage: Option<Qty<Voltage>>,
    /// 省略時は中心天体の太陽距離の慣例値（地球・月は 1 AU。`CentralBody::nominal_sun_distance_m`）。
    /// それ以外の天体では書く。発電は太陽光の強さ SOLAR_CONSTANT·(1 AU/d)² で比例する
    pub sun_distance: Option<Qty<Length>>,
}

impl SizingOptions {
    /// 日照経路の効率 Xd（省略時 [`DEFAULT_SUNLIT_EFFICIENCY`]）。
    pub fn sunlit_eff(&self) -> f64 {
        self.sunlit_efficiency.unwrap_or(DEFAULT_SUNLIT_EFFICIENCY)
    }

    /// 蝕経路の効率 Xe（省略時 [`DEFAULT_ECLIPSE_EFFICIENCY`]）。
    pub fn eclipse_eff(&self) -> f64 {
        self.eclipse_efficiency
            .unwrap_or(DEFAULT_ECLIPSE_EFFICIENCY)
    }
}

/// 軌道と期間・発電を評価する時系列と姿勢・全モードまたは指定モード・設計の部品と上書き。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SizingInput {
    pub season: EclipseSeasonInput,
    /// 入射を評価する時系列（`power.orbit_generation` と同じ）。姿勢則はモードの上書きが無いときに使う
    pub timeline: TimelineSpec,
    /// モードごとの姿勢則（`timeline.attitude` を替える。`power.energy_balance` と同じ）
    #[serde(default)]
    pub mode_laws: BTreeMap<String, AttitudeLaw>,
    pub loads: ModeLoadInput,
    pub mode: Option<String>,
    /// 設計寿命（経年劣化 `(1 − 年あたりの劣化率)^寿命` の寿命）。省略時は `season.duration`（季節の走査の期間）を寿命とみなす
    #[serde(default)]
    pub design_life: Option<Qty<Time>>,
    /// 設計の SAP（法線つき）。変換効率は日照経路の効率 Xd で数えるのでここでは使わない
    pub array: SolarArray,
    #[serde(default)]
    pub cells: Vec<SizingCell>,
    pub batteries: Vec<EnergyBalanceBattery>,
    pub options: SizingOptions,
    /// モードの最長継続時間（`power.energy_balance` と同じ。時系列の逆算で基本のモードの上に 1 回入れる）
    #[serde(default)]
    pub max_durations: BTreeMap<String, Qty<Time>>,
    /// モードごとの時刻の指定（`power.energy_balance` と同じ。最長継続時間の開始の選び方）
    #[serde(default)]
    pub mode_timings: BTreeMap<String, ModeTiming>,
    /// モードの並び（`power.energy_balance` と同じ。全モードを逆算するときだけ時系列の逆算に使う）
    #[serde(default)]
    pub sequence: Vec<EnergyBalanceSegment>,
    /// 並びの名前と先頭の位置（`power.energy_balance` と同じ）
    #[serde(default)]
    pub sequence_timing: SequenceTiming,
    /// 最長継続時間の評価で下に敷く基本のモード（`power.energy_balance` と同じ）
    #[serde(default)]
    pub base_mode: Option<String>,
}

/// 計算に用いた仮定。寿命はミッション期間の Julian year 換算。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SizingUsed {
    pub sunlit_efficiency: f64,
    pub eclipse_efficiency: f64,
    pub battery_efficiency: f64,
    pub annual_degradation: f64,
    pub inherent_degradation: f64,
    pub lifetime_years: f64,
    /// `lifetime_years` をどこから取ったか
    #[serde(default)]
    pub lifetime_source: SizingLifetimeSource,
    pub life_factor: f64,
    pub dod_max: Fraction,
    pub battery_strings: u32,
    pub sun_distance: Qty<Length>,
    /// `sun_distance` の出どころ。設定と中心天体の慣例値（従来の出どころ）は書き出さず、期間の最大のときだけ書き出す
    #[serde(
        default,
        skip_serializing_if = "SizingSunDistanceSource::is_conventional"
    )]
    pub sun_distance_source: SizingSunDistanceSource,
    /// `sun_distance_source = span_max` のとき、期間の中で太陽距離が最大になった日付
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sun_distance_epoch: Option<Epoch>,
    /// 部品別のセル効率・実装率・EOL 係数（省略値を解決済み）。
    pub panels: Vec<SolarPanel>,
}

/// モードごとの逆算。`required_*` は SMAD の式、`series_*` は時系列の逆算（[`SizingSeriesTarget`]）。
/// 入射係数は姿勢則の時系列での日照中の平均発電 / 真上から当たるときの発電。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SizingMode {
    pub id: String,
    pub sunlit_load: Qty<Power>,
    pub eclipse_load: Qty<Power>,
    pub incidence_factor: Fraction,
    pub required_generation: Qty<Power>,
    /// SMAD の式の必要面積
    pub required_area: Qty<Area>,
    /// SMAD の式の必要容量（合計）
    pub required_capacity: Qty<Energy>,
    /// 時系列の逆算の必要面積（時系列の逆算ができなければ None）
    #[serde(default)]
    pub series_area: Option<Qty<Area>>,
    /// 時系列の逆算の必要容量（全体の必要面積で求めたもの）
    #[serde(default)]
    pub series_capacity: Option<Qty<Energy>>,
    /// 時系列の面積 / SMAD の面積（SMAD がどれだけ少なく見ていたか。SMAD が 0 なら None）
    #[serde(default)]
    pub area_ratio: Option<Fraction>,
    /// 時系列の容量 / SMAD の容量
    #[serde(default)]
    pub capacity_ratio: Option<Fraction>,
}

/// 経年劣化の寿命の出どころ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SizingLifetimeSource {
    /// 季節の走査の期間（`season.duration`）。設計寿命の入力が無いとき
    #[default]
    SeasonSpan,
    /// 設計寿命の入力（`design_life`）
    DesignLife,
}

/// 発電の強さを決める太陽距離の出どころ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SizingSunDistanceSource {
    /// 設定 `[sizing] sun_distance`
    Setting,
    /// 中心天体の慣例値（地球・月は 1 AU）
    #[default]
    Nominal,
    /// 慣例値の無い中心天体で、設定も無いとき: 季節の走査の期間（ミッション期間）の中の太陽距離の最大（最悪の日。発電が最小）
    SpanMax,
}

impl SizingSunDistanceSource {
    /// 設定か中心天体の慣例値か（従来の出どころ。出力に書かない）。
    pub fn is_conventional(&self) -> bool {
        !matches!(self, Self::SpanMax)
    }
}

/// KPI と判定に使った必要量の求め方。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SizingBasis {
    /// 時系列の逆算（電力収支と同じ運用）
    #[default]
    TimeSeries,
    /// SMAD の式（時系列の逆算ができなかったとき）
    Smad,
}

/// SMAD の式による全体の必要量（参考値）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SizingSmad {
    pub required_generation: Qty<Power>,
    pub required_area: Qty<Area>,
    pub required_capacity: Qty<Energy>,
    pub sap_margin: Option<Fraction>,
    pub battery_margin: Option<Fraction>,
    /// 必要面積が最大のモード
    pub area_mode: String,
    /// 必要容量が最大のモード
    pub capacity_mode: String,
    /// `area_mode` の日照中の負荷
    pub sunlit_load: Qty<Power>,
    /// `capacity_mode` の蝕中の負荷
    pub eclipse_load: Qty<Power>,
}

/// 同じセルを使う場合の太陽電池全体の直並列数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SizingStrings {
    pub series_count: u32,
    pub parallel_count: u32,
    pub required_current: Qty<ElectricCurrent>,
}

/// 必要量と現在値の比較。必要量・余裕・決めた運用は `basis` の求め方の値（容量は N 組の合計）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SizingOutput {
    /// KPI と判定に使った必要量の求め方
    pub basis: SizingBasis,
    /// モードごとの面積・容量の判定（`basis` の値で）。
    pub mode_checks: BTreeMap<String, Vec<Check>>,
    /// 必要面積（`basis` の求め方）。
    pub required_area: Qty<Area>,
    pub required_capacity: Qty<Energy>,
    pub required_capacity_per_string: Qty<Energy>,
    pub design_area: Qty<Area>,
    pub design_capacity: Qty<Energy>,
    pub sap_margin: Option<Fraction>,
    pub battery_margin: Option<Fraction>,
    /// 必要面積を決めた運用（モードの ID か `sequence`）
    pub area_mode: String,
    /// 必要容量を決めた運用
    pub capacity_mode: String,
    /// SMAD の式による全体の必要量（参考値）
    pub smad: SizingSmad,
    /// 時系列の逆算（できなければ None。理由は `checks` の `time_series`）
    pub time_series: Option<SizingTimeSeries>,
    /// 直並列数（SMAD の必要発電と、面積を決めたモードの入射係数から）
    pub strings: Option<SizingStrings>,
    pub modes: Vec<SizingMode>,
    /// 必要量に、部品どうしの影（部品の箱から出すパネルの面の日照割合）が入っているか。入っていれば、必要面積と容量は
    /// 現在の部品の配置の影を含んだ値（影を減らす配置の変更で変わる。パネルの面積を替えても影の割合は変わらない）。
    /// 影の割合は `power.orbit_generation` / `power.energy_balance` の `shadowing`
    #[serde(default)]
    pub shadowed: bool,
    pub sunlit_duration: Qty<Time>,
    pub eclipse_duration: Qty<Time>,
    pub longest: EclipseSeasonPoint,
    pub used: SizingUsed,
    pub checks: Vec<Check>,
}

/// 電源の逆算（部品へ書き戻さない）。
pub struct Sizing;
impl Analysis for Sizing {
    const ID: &'static str = "power.sizing";
    const USES: &'static [&'static str] = &[
        EclipseSeason::ID,
        ModeLoad::ID,
        super::OrbitGeneration::ID,
        super::EnergyBalance::ID,
    ];
    const TITLE: &'static str = "電源の逆算サイジング";
    const TITLE_EN: &'static str = "Power sizing";
    const AREA: AnalysisArea = AnalysisArea::System(System::Power);
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str = "最長蝕とモード負荷から太陽電池の面積と電池容量を逆算する";
    const DESCRIPTION_EN: &'static str =
        "Required solar array area and battery capacity from worst eclipse and mode loads";
    type Input = SizingInput;
    type Output = SizingOutput;

    fn run(input: &SizingInput) -> Result<SizingOutput, AnalysisError> {
        let combined = EnergyBalanceBattery::combine(&input.batteries)?;
        let mismatch = combined.mismatch_check();
        let battery = combined.battery;
        let used = used(input, &battery)?;
        // モードの負荷は 1 回の実行で 1 度だけ求める（SMAD の逆算も時系列の逆算も同じ出力を使う）
        let (loads, mode_loads) = loads(input)?;
        let mut checks: Vec<Check> = mismatch.into_iter().collect();
        let season = EclipseSeason::run(&input.season)?;
        let (period, te) = (
            input.season.orbit.period_s(),
            season.max_eclipse_duration.value(),
        );
        let td = period - te;
        let (design_area, array) = design(input, &used)?;
        let mut modes = modes::evaluate(input, &array, &used, &loads, td, te)?;
        let smad = smad_totals(&modes, design_area, battery.capacity);
        let series = series::evaluate(
            input,
            &series::SeriesBasis {
                battery: &battery,
                used: &used,
                design_area: design_area.value(),
                longest: &season.longest,
                loads: &loads,
                mode_loads: &mode_loads,
                smad: &modes,
            },
        )?;
        let (chosen, time_series) = choose(series, &mut modes, &smad, &mut checks);
        let (sap_margin, battery_margin) =
            margin_checks(&chosen, design_area, battery.capacity, &mut checks);
        Ok(SizingOutput {
            basis: chosen.basis,
            mode_checks: mode_checks(&modes, chosen.basis, design_area, battery.capacity),
            required_area: chosen.area,
            required_capacity: chosen.capacity,
            required_capacity_per_string: chosen.capacity / f64::from(used.battery_strings),
            design_area,
            design_capacity: battery.capacity,
            sap_margin,
            battery_margin,
            strings: smad_strings(input, &used, &modes, &smad)?,
            area_mode: chosen.area_mode,
            capacity_mode: chosen.capacity_mode,
            smad,
            time_series,
            sunlit_duration: Qty::from_raw(td),
            eclipse_duration: Qty::from_raw(te),
            longest: season.longest,
            modes,
            shadowed: input.array.shading().is_some(),
            used,
            checks,
        })
    }
}

mod calculate;
mod judge;
mod modes;
mod series;
use calculate::{design, loads, used};
use judge::{choose, margin_checks, mode_checks, smad_strings, smad_totals};
pub use series::{AREA_TOLERANCE, SizingSeriesTarget, SizingTimeSeries, worst_season};

#[cfg(test)]
mod tests;
