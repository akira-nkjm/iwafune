//! 軌道の解析（`orbit.eclipse_season`・`orbit.decay`・`orbit.geometry`）の KPI の値（`kpi_table!` から引く）。

use super::{Ctx, Measured, largest, one, smallest};
use sat_analysis::orbit::{EclipseSeasonOutput, GeometryOutput, OrbitDecayOutput, TotalDoseOutput};

pub(super) fn longest_eclipse(o: &EclipseSeasonOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.max_eclipse_duration)
}
pub(super) fn beta_min(o: &EclipseSeasonOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.beta_min.beta)
}
pub(super) fn beta_max(o: &EclipseSeasonOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.beta_max.beta)
}

pub(super) fn orbit_lifetime(o: &OrbitDecayOutput, _: Ctx<'_>) -> Option<Measured> {
    // 再突入しない条件があっても、再突入する条件のうち最短を出す（どれも再突入しなければ値なし）
    smallest(o.cases.iter().filter_map(|c| c.lifetime))
}
pub(super) fn decay_per_day(o: &OrbitDecayOutput, _: Ctx<'_>) -> Option<Measured> {
    largest(o.cases.iter().map(|c| c.decay_per_day))
}
pub(super) fn station_keeping_delta_v(o: &OrbitDecayOutput, _: Ctx<'_>) -> Option<Measured> {
    largest(o.cases.iter().map(|c| c.station_keeping.delta_v_per_year))
}

/// 地球までの距離の最大（区間ごとの最大のうち最大。通信の距離・光の遅れの上限を決める側）。
pub(super) fn greatest_earth_distance(o: &GeometryOutput, _: Ctx<'_>) -> Option<Measured> {
    largest(o.legs.iter().map(|l| l.earth_distance.max.value))
}

/// 放射線の総線量の余裕の最小（許容線量を持ち、評価できた部品のうち最小。無ければ値なし）。
pub(super) fn total_dose_margin(o: &TotalDoseOutput, _: Ctx<'_>) -> Option<Measured> {
    smallest(o.parts.iter().filter_map(|p| p.margin))
}
