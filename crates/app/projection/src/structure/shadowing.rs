//! 影を数えるための入力。部品の箱（[`super::boxes`]。いま有効な形態の配置）と、設計の設定（`analysis.<系>.shadowing`）を、
//! 電源と熱の解析の入力（`ShadowingInput`）に写すだけ。影の規則と既定（箱があれば数える）は sat-analysis が持つ。

use super::body_boxes::boxes;
use crate::context::Context;
use fune_model::design::AnalysisSystem;
use sat_analysis::structure::external_geometry::ShadowingInput;
use sat_analysis::structure::interference::InterferenceBox;

/// 影を数えるための入力。寸法を持つ部品が 1 つも無ければ `None`。`enabled` は設計の設定（`analysis.<系>.shadowing`。読んだことを記録する）。
pub fn shadowing(
    ctx: &Context<'_>,
    system: AnalysisSystem,
    enabled: Option<bool>,
) -> Option<ShadowingInput> {
    let enabled = ctx.setting(system.setting_path().child(&["shadowing"]), enabled);
    let body = boxes(ctx);
    (!body.boxes.is_empty()).then(|| ShadowingInput {
        boxes: body.boxes.into_iter().map(InterferenceBox::from).collect(),
        enabled,
    })
}
