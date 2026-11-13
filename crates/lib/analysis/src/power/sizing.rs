//! SMAD の逆算サイジング。季節の蝕・モード負荷・発電モデルの正本を共有する。
use super::battery::EnergyBalanceBattery;
use super::mode_load::{ModeLoad, ModeLoadInput};
use super::solar::{SolarArray, SolarPanel};
use crate::orbit::{EclipseSeason, EclipseSeasonInput, EclipseSeasonPoint};
use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Check, System, ensure_fraction,
    ensure_non_negative, ensure_positive, quantity_text, ratio_margin, ratio_margin_verdict,
};
use sat_astro::attitude::AttitudeLaw;
use sat_astro::constants::AU;
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
    /// 省略時は 1 AU。1 AU 以外は未対応。
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

/// モードごとの逆算。入射係数は姿勢則の時系列での日照中の平均発電 / 真上から当たるときの発電。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SizingMode {
    pub id: String,
    pub sunlit_load: Qty<Power>,
    pub eclipse_load: Qty<Power>,
    pub incidence_factor: Fraction,
    pub required_generation: Qty<Power>,
    pub required_area: Qty<Area>,
    pub required_capacity: Qty<Energy>,
}

/// 同じセルを使う場合の太陽電池全体の直並列数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SizingStrings {
    pub series_count: u32,
    pub parallel_count: u32,
    pub required_current: Qty<ElectricCurrent>,
}

/// 必要量と現在値の比較。容量は N 組の合計を返す。面積は `area_mode`、容量は `capacity_mode` で決まる。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SizingOutput {
    /// モードごとの面積・容量の判定。
    pub mode_checks: BTreeMap<String, Vec<Check>>,
    pub required_generation: Qty<Power>,
    pub required_area: Qty<Area>,
    pub required_capacity: Qty<Energy>,
    pub required_capacity_per_string: Qty<Energy>,
    pub design_area: Qty<Area>,
    pub design_capacity: Qty<Energy>,
    pub sap_margin: Option<Fraction>,
    pub battery_margin: Option<Fraction>,
    pub strings: Option<SizingStrings>,
    /// 必要面積が最大のモード
    pub area_mode: String,
    /// 必要容量が最大のモード
    pub capacity_mode: String,
    /// `area_mode` の日照中の負荷
    pub sunlit_load: Qty<Power>,
    /// `capacity_mode` の蝕中の負荷
    pub eclipse_load: Qty<Power>,
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
        let te = season.max_eclipse_duration.value();
        let td = input.season.orbit.period_s() - te;
        let (design_area, array) = design(input)?;
        let modes = modes::evaluate(input, &array, &used, &loads, td, te)?;
        let mode_checks = mode_checks(&modes, design_area, battery.capacity);
        let area = modes::governing(&modes, |m| m.required_area.value());
        let capacity = modes::governing(&modes, |m| m.required_capacity.value());
        let sap_margin = ratio_margin(design_area.value(), area.required_area.value());
        let battery_margin =
            ratio_margin(battery.capacity.value(), capacity.required_capacity.value());
        checks.push(margin_check::<Area>(
            "sap_margin",
            "太陽電池の余裕",
            sap_margin,
            design_area.value(),
            area.required_area.value(),
        ));
        checks.push(margin_check::<Energy>(
            "battery_margin",
            "バッテリーの余裕",
            battery_margin,
            battery.capacity.value(),
            capacity.required_capacity.value(),
        ));
        let generation = area.required_generation;
        Ok(SizingOutput {
            mode_checks,
            required_generation: generation,
            required_area: area.required_area,
            required_capacity: capacity.required_capacity,
            required_capacity_per_string: capacity.required_capacity
                / f64::from(used.battery_strings),
            design_area,
            design_capacity: battery.capacity,
            sap_margin,
            battery_margin,
            strings: strings(
                input,
                &used,
                generation.value(),
                area.incidence_factor.value(),
            )?,
            area_mode: area.id.clone(),
            capacity_mode: capacity.id.clone(),
            sunlit_load: area.sunlit_load,
            eclipse_load: capacity.eclipse_load,
            sunlit_duration: Qty::from_raw(td),
            eclipse_duration: Qty::from_raw(te),
            longest: season.longest,
            modes: modes.clone(),
            used,
            checks,
        })
    }
}

/// 各モードの必要量を同じ面積・容量の基準で比較する。
fn mode_checks(
    modes: &[SizingMode],
    area: Qty<Area>,
    capacity: Qty<Energy>,
) -> BTreeMap<String, Vec<Check>> {
    modes
        .iter()
        .map(|m| {
            (
                m.id.clone(),
                vec![
                    margin_check::<Area>(
                        "sap_margin",
                        "太陽電池の余裕",
                        ratio_margin(area.value(), m.required_area.value()),
                        area.value(),
                        m.required_area.value(),
                    ),
                    margin_check::<Energy>(
                        "battery_margin",
                        "電池の余裕",
                        ratio_margin(capacity.value(), m.required_capacity.value()),
                        capacity.value(),
                        m.required_capacity.value(),
                    ),
                ],
            )
        })
        .collect()
}

mod calculate;
mod modes;
use calculate::{SizingLoads, design, loads, margin_check, strings, used};

#[cfg(test)]
mod tests;
