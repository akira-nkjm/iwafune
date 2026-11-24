//! 構造の新しい解析（打ち上げ適合・干渉・円錐の遮り・外形の面積）の KPI の値（`kpi_table!` から引く）。

use super::{Ctx, Measured, one, smallest};
use crate::dto::KpiTally;
use sat_analysis::structure::{
    ConeClearanceOutput, ExternalGeometryOutput, InterferenceOutput, LaunchFitOutput,
};

/// 打ち上げ適合の最小の余裕（質量と包絡の軸のうち、余裕を持つもの。どれも持たなければ None）。
/// 固有振動数の余裕は `structure.modal_estimate` の推定が要り、無いことが多いので入れない（その判定は行の判定に出る）。
pub(super) fn launch_fit_margin(o: &LaunchFitOutput, _: Ctx<'_>) -> Option<Measured> {
    smallest(
        o.mass
            .iter()
            .filter_map(|m| m.margin)
            .chain(o.envelope.iter().filter_map(|a| a.margin)),
    )
}

/// 干渉: 判定した組のうち重なっている組の数（判定した組が無ければ None）。
pub(super) fn interference_overlaps(o: &InterferenceOutput, _: Ctx<'_>) -> Option<KpiTally> {
    counted(o.pairs_judged, o.overlaps.len())
}

/// 円錐の遮り: 判定した円錐のうち部品が入っている担い手の数（判定した円錐が無ければ None）。
pub(super) fn cone_obstructions(o: &ConeClearanceOutput, _: Ctx<'_>) -> Option<KpiTally> {
    counted(o.cones_judged, o.cones_obstructed as usize)
}

/// 外形の投影面積の平均（全方向の平均）。
pub(super) fn projected_area_mean(o: &ExternalGeometryOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.mean_projected_area)
}

fn counted(checked: u32, flagged: usize) -> Option<KpiTally> {
    (checked > 0).then(|| KpiTally {
        checked,
        flagged: u32::try_from(flagged).unwrap_or(u32::MAX),
    })
}
