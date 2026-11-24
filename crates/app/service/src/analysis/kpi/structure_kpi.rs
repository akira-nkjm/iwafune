//! 構造の新しい解析（打ち上げ適合・干渉・円錐の遮り・外形の面積・有限要素の固有値）の KPI の値（`kpi_table!` から引く）。

use super::{Ctx, Measured, one, smallest};
use crate::dto::KpiTally;
use sat_analysis::structure::cone_clearance::ConeReport;
use sat_analysis::structure::modal_fem::ModalFemModeClass;
use sat_analysis::structure::{
    ConeClearanceOutput, ExternalGeometryOutput, InterferenceOutput, LaunchFitOutput,
    ModalFemOutput,
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

/// 干渉: 判定した組のうち重なっている組の数（判定した組が無ければ None）。承知済みの組は `flagged` に数えず、`acknowledged` に数える。
pub(super) fn interference_overlaps(o: &InterferenceOutput, _: Ctx<'_>) -> Option<KpiTally> {
    let acknowledged = o.overlaps.iter().filter(|p| p.acknowledged).count();
    counted(
        o.pairs_judged,
        o.overlaps.len() - acknowledged,
        acknowledged,
    )
}

/// 円錐の遮り: 判定した円錐のうち部品が入っている担い手の数（判定した円錐が無ければ None）。承知でない部品が入っている担い手は
/// `flagged`、入っている部品がすべて承知済みの担い手は `acknowledged` に数える。
pub(super) fn cone_obstructions(o: &ConeClearanceOutput, _: Ctx<'_>) -> Option<KpiTally> {
    let open = |c: &&ConeReport| c.obstructions.iter().any(|x| !x.acknowledged);
    let flagged = o.cones.iter().filter(open).count();
    let acknowledged = o
        .cones
        .iter()
        .filter(|c| !c.obstructions.is_empty() && !open(c))
        .count();
    counted(o.cones_judged, flagged, acknowledged)
}

/// 外形の投影面積の平均（全方向の平均）。
pub(super) fn projected_area_mean(o: &ExternalGeometryOutput, _: Ctx<'_>) -> Option<Measured> {
    one(o.mean_projected_area)
}

/// 有限要素の固有値解析の最低の固有振動数（剛体のモードを除く最初のモード。局所のモードも含む）。要求の判定は無い。
pub(super) fn first_frequency(o: &ModalFemOutput, _: Ctx<'_>) -> Option<Measured> {
    smallest(
        o.modes
            .iter()
            .filter(|m| m.class != ModalFemModeClass::Rigid)
            .map(|m| m.frequency),
    )
}

fn counted(checked: u32, flagged: usize, acknowledged: usize) -> Option<KpiTally> {
    let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    (checked > 0).then(|| KpiTally {
        checked,
        flagged: count(flagged),
        acknowledged: Some(count(acknowledged)),
    })
}
