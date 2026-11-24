//! 構造の新しい解析（打ち上げ適合・干渉・円錐の遮り・外形の面積・有限要素の固有値）の KPI の値（`kpi_table!` から引く）。

use super::{Ctx, Measured, one, smallest};
use crate::dto::KpiTally;
use sat_analysis::structure::modal_fem::ModalFemModeClass;
use sat_analysis::structure::{
    ConeClearanceOutput, ExternalGeometryOutput, InterferenceOutput, LaunchFitOutput,
    ModalFemOutput,
};

/// 打ち上げ適合の最小の余裕（質量・包絡の軸・固有振動数のうち、余裕を持つもの。どれも持たなければ None）。
/// 固有振動数の余裕は、見積もり（有限要素か梁の式）が無ければ持たない（その判定は行の判定に出る）。
/// 探した範囲で適合とした向きの余裕は下界。
pub(super) fn launch_fit_margin(o: &LaunchFitOutput, _: Ctx<'_>) -> Option<Measured> {
    smallest(
        o.mass
            .iter()
            .filter_map(|m| m.margin)
            .chain(o.envelope.iter().filter_map(|a| a.margin))
            .chain(o.frequencies.iter().filter_map(|f| f.margin)),
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

/// 有限要素の固有値解析の最低の固有振動数（剛体のモードを除く最初のモード。局所のモードも含む）。要求の判定は無い。
pub(super) fn first_frequency(o: &ModalFemOutput, _: Ctx<'_>) -> Option<Measured> {
    smallest(
        o.modes
            .iter()
            .filter(|m| m.class != ModalFemModeClass::Rigid)
            .map(|m| m.frequency),
    )
}

fn counted(checked: u32, flagged: usize) -> Option<KpiTally> {
    (checked > 0).then(|| KpiTally {
        checked,
        flagged: u32::try_from(flagged).unwrap_or(u32::MAX),
    })
}
