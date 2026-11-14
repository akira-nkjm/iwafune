//! 電源の逆算サイジング。季節の蝕・モード負荷・発電モデル・電力収支の正本を共有する。
//!
//! - SMAD の式（[`calculate::smad_requirements`]）: 手計算と比べられる参考値（`smad`・`modes` の SMAD の欄）。
//! - 時系列の逆算（[`series`]）: 電力収支と同じ運用で 1 周の収支 ≥ 0 の最小面積と、DoD が上限に収まる最小容量。
//!   KPI と判定（`sap_margin`・`battery_margin`・`mode_checks`）はこちらで行う（`basis`）。判定できなければ SMAD で行う。
use super::battery::EnergyBalanceBattery;
use super::energy_balance::EnergyBalanceSegment;
use super::mode_load::{ModeLoad, ModeLoadInput};
use super::solar::{SolarArray, SolarPanel};
use crate::orbit::{EclipseSeason, EclipseSeasonInput, EclipseSeasonPoint};
use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Check, System, ensure_fraction,
    ensure_non_negative, ensure_positive, quantity_text, ratio_margin_verdict,
};
use sat_astro::attitude::AttitudeLaw;
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
    /// 設計の SAP（法線つき）。変換効率は日照経路の効率 Xd で数えるのでここでは使わない
    pub array: SolarArray,
    #[serde(default)]
    pub cells: Vec<SizingCell>,
    pub batteries: Vec<EnergyBalanceBattery>,
    pub options: SizingOptions,
    /// モードの最長継続時間（`power.energy_balance` と同じ。時系列の逆算で基本のモードの上に 1 回入れる）
    #[serde(default)]
    pub max_durations: BTreeMap<String, Qty<Time>>,
    /// モードの並び（`power.energy_balance` と同じ。全モードを逆算するときだけ時系列の逆算に使う）
    #[serde(default)]
    pub sequence: Vec<EnergyBalanceSegment>,
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
    pub life_factor: f64,
    pub dod_max: Fraction,
    pub battery_strings: u32,
    pub sun_distance: Qty<Length>,
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
        let battery = EnergyBalanceBattery::combine(&input.batteries)?;
        let used = used(input, &battery)?;
        let SizingLoads { loads, mut checks } = loads(input)?;
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
            used,
            checks,
        })
    }
}

mod calculate;
mod judge;
mod modes;
mod series;
use calculate::{SizingLoads, design, loads, used};
use judge::{choose, margin_checks, mode_checks, smad_strings, smad_totals};
pub use series::{AREA_TOLERANCE, SizingSeriesTarget, SizingTimeSeries, worst_season};

#[cfg(test)]
mod tests;
