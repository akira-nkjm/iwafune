//! 電源の解析（`power.mode_load`・`power.orbit_generation`・`power.energy_balance`・`power.sizing`）の KPI の値
//! （`kpi_table!` から引く）。電力のモードの選び方はバジェットと同じ `budget::budget_mode`。

use super::{Ctx, Measured, largest, one, smallest};
use sat_analysis::power;
use sat_units::typed::{Fraction, Qty};

pub(super) fn mode_load(o: &power::ModeLoadOutput, ctx: Ctx<'_>) -> Option<Measured> {
    one(crate::budget::budget_mode(o, ctx.case_mode)?.total_with_growth)
}
pub(super) fn orbit_generation(o: &power::OrbitGenerationOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.orbit_average)
}
pub(super) fn sunlit_fraction(o: &power::OrbitGenerationOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.sunlit_fraction)
}
/// 判定に使った運用での電力収支の結果: 最長継続時間で評価したケースはその値、ほかは連続の値。並びがあれば並びも入れる。
fn judged_balances(
    o: &power::EnergyBalanceOutput,
) -> impl Iterator<Item = (Qty<sat_units::typed::Power>, Fraction)> + '_ {
    let cases = o.cases.iter().map(|c| match &c.by_max_duration {
        Some(d) => (d.orbit_average_balance, d.max_dod_from_start),
        None => (c.orbit_average_balance, c.max_dod),
    });
    let sequence = o
        .sequence
        .iter()
        .map(|s| (s.balance.orbit_average_balance, s.balance.max_dod));
    cases.chain(sequence)
}
pub(super) fn orbit_balance(o: &power::EnergyBalanceOutput, _: Ctx<'_>) -> Option<Measured> {
    smallest(judged_balances(o).map(|(b, _)| b))
}
pub(super) fn max_dod(o: &power::EnergyBalanceOutput, _: Ctx<'_>) -> Option<Measured> {
    largest(judged_balances(o).map(|(_, d)| d))
}
pub(super) fn sap_margin(o: &power::SizingOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.sap_margin?)
}
pub(super) fn battery_margin(o: &power::SizingOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.battery_margin?)
}
