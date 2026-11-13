//! 推進系の KPI の値（`kpi_table!` から引く）。

use super::{Ctx, Measured, largest, one, smallest};
use sat_analysis::propulsion::{
    DvBudgetOutput, ManeuversOutput, PropellantOutput, ThrusterUsageOutput,
};

pub(super) fn longest_transfer(o: &ManeuversOutput, _: Ctx<'_>) -> Option<Measured> {
    largest(o.maneuvers.iter().filter_map(|m| m.transfer_time))
}
pub(super) fn dv_with_margin(o: &DvBudgetOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.total_with_margin)
}
pub(super) fn loaded_propellant(o: &PropellantOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.loaded_propellant)
}
pub(super) fn tank_volume_margin(o: &PropellantOutput, _: Ctx<'_>) -> Option<Measured> {
    smallest(o.groups.iter().filter_map(|g| g.volume_margin))
}
pub(super) fn impulse_margin(o: &ThrusterUsageOutput, _: Ctx<'_>) -> Option<Measured> {
    smallest(o.thrusters.iter().filter_map(|t| t.impulse_margin))
}
