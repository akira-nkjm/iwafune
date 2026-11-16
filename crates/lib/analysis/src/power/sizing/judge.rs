//! 判定に使う必要量の選び方（時系列の逆算か SMAD の式か）と、余裕の判定。
use super::calculate::{margin_check, strings};
use super::{
    SizingBasis, SizingInput, SizingMode, SizingSmad, SizingStrings, SizingTimeSeries, SizingUsed,
    modes,
};
use crate::spec::{AnalysisError, Check, ratio_margin};
use sat_units::typed::{Area, Energy, Fraction, Qty};
use std::collections::BTreeMap;

/// 面積と容量の余裕の判定のキーと表示名。
const SAP: (&str, &str) = ("sap_margin", "Solar cell margin");
const BATTERY: (&str, &str) = ("battery_margin", "Battery margin");

impl SizingMode {
    /// `basis` の求め方での必要面積と必要容量（時系列の値が無ければ SMAD の値）。
    pub fn required(&self, basis: SizingBasis) -> (Qty<Area>, Qty<Energy>) {
        match basis {
            SizingBasis::TimeSeries => (
                self.series_area.unwrap_or(self.required_area),
                self.series_capacity.unwrap_or(self.required_capacity),
            ),
            SizingBasis::Smad => (self.required_area, self.required_capacity),
        }
    }
}

impl SizingBasis {
    /// 判定の文の括弧書き。
    pub(super) fn note(self) -> &'static str {
        match self {
            Self::TimeSeries => "time-series sizing",
            Self::Smad => "SMAD formula",
        }
    }
}

/// KPI と判定に使う必要量と、決めた運用。
pub(super) struct Chosen {
    pub basis: SizingBasis,
    pub area: Qty<Area>,
    pub capacity: Qty<Energy>,
    pub area_mode: String,
    pub capacity_mode: String,
}

impl Chosen {
    pub(super) fn series(ts: &SizingTimeSeries) -> Self {
        Self {
            basis: SizingBasis::TimeSeries,
            area: ts.required_area,
            capacity: ts.required_capacity,
            area_mode: ts.area_target.clone(),
            capacity_mode: ts.capacity_target.clone(),
        }
    }

    pub(super) fn smad(s: &SizingSmad) -> Self {
        Self {
            basis: SizingBasis::Smad,
            area: s.required_area,
            capacity: s.required_capacity,
            area_mode: s.area_mode.clone(),
            capacity_mode: s.capacity_mode.clone(),
        }
    }
}

/// 判定に使う必要量を選ぶ。時系列の逆算ができればその値をモードに入れて使い、できなければ SMAD の値と注意。
pub(super) fn choose(
    series: Result<SizingTimeSeries, String>,
    modes: &mut [SizingMode],
    smad: &SizingSmad,
    checks: &mut Vec<Check>,
) -> (Chosen, Option<SizingTimeSeries>) {
    match series {
        Ok(ts) => {
            modes::attach_series(modes, &ts);
            (Chosen::series(&ts), Some(ts))
        }
        Err(reason) => {
            checks.push(series_unavailable(&reason));
            (Chosen::smad(smad), None)
        }
    }
}

/// 設計の面積・容量と判定に使う必要量の余裕を求め、その判定を `checks` に足す。
pub(super) fn margin_checks(
    chosen: &Chosen,
    area: Qty<Area>,
    capacity: Qty<Energy>,
    checks: &mut Vec<Check>,
) -> (Option<Fraction>, Option<Fraction>) {
    let note = chosen.basis.note();
    let (need_area, need_capacity) = (chosen.area.value(), chosen.capacity.value());
    let sap = ratio_margin(area.value(), need_area);
    let battery = ratio_margin(capacity.value(), need_capacity);
    checks.push(margin_check::<Area>(
        SAP,
        sap,
        area.value(),
        need_area,
        note,
    ));
    checks.push(margin_check::<Energy>(
        BATTERY,
        battery,
        capacity.value(),
        need_capacity,
        note,
    ));
    (sap, battery)
}

/// 時系列の逆算ができなかったことの注意。
fn series_unavailable(reason: &str) -> Check {
    Check::new(
        "time_series",
        "time-series sizing",
        crate::spec::Verdict::Warning,
        format!("Time-series sizing is not possible, so the SMAD formula is used ({reason})"),
    )
}

/// SMAD の式による全体の必要量（面積・容量それぞれ最大のモード）。`modes` は空でない。
pub(super) fn smad_totals(
    modes: &[SizingMode],
    area: Qty<Area>,
    capacity: Qty<Energy>,
) -> SizingSmad {
    let by_area = modes::governing(modes, |m| m.required_area.value());
    let by_capacity = modes::governing(modes, |m| m.required_capacity.value());
    SizingSmad {
        required_generation: by_area.required_generation,
        required_area: by_area.required_area,
        required_capacity: by_capacity.required_capacity,
        sap_margin: ratio_margin(area.value(), by_area.required_area.value()),
        battery_margin: ratio_margin(capacity.value(), by_capacity.required_capacity.value()),
        area_mode: by_area.id.clone(),
        capacity_mode: by_capacity.id.clone(),
        sunlit_load: by_area.sunlit_load,
        eclipse_load: by_capacity.eclipse_load,
    }
}

/// 直並列数（SMAD の必要発電と、SMAD で面積を決めたモードの入射係数）。
pub(super) fn smad_strings(
    input: &SizingInput,
    used: &SizingUsed,
    modes: &[SizingMode],
    smad: &SizingSmad,
) -> Result<Option<SizingStrings>, AnalysisError> {
    let incidence = modes
        .iter()
        .find(|m| m.id == smad.area_mode)
        .map_or(1.0, |m| m.incidence_factor.value());
    strings(input, used, smad.required_generation.value(), incidence)
}

/// 各モードの必要量（`basis` の値）を同じ面積・容量の基準で比較する。
pub(super) fn mode_checks(
    modes: &[SizingMode],
    basis: SizingBasis,
    area: Qty<Area>,
    capacity: Qty<Energy>,
) -> BTreeMap<String, Vec<Check>> {
    let note = basis.note();
    modes
        .iter()
        .map(|m| {
            let (need_area, need_capacity) = m.required(basis);
            let margin = |d: f64, r: f64| ratio_margin(d, r);
            (
                m.id.clone(),
                vec![
                    margin_check::<Area>(
                        SAP,
                        margin(area.value(), need_area.value()),
                        area.value(),
                        need_area.value(),
                        note,
                    ),
                    margin_check::<Energy>(
                        ("battery_margin", "Battery margin"),
                        margin(capacity.value(), need_capacity.value()),
                        capacity.value(),
                        need_capacity.value(),
                        note,
                    ),
                ],
            )
        })
        .collect()
}
