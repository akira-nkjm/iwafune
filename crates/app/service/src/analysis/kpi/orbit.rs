//! 軌道の条件の解析（`orbit.eclipse_season`・`orbit.decay`）の KPI の値（`kpi_table!` から引く）。

use super::{Ctx, Measured, largest, one, smallest};
use sat_analysis::orbit::{EclipseSeasonOutput, OrbitDecayOutput};

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
