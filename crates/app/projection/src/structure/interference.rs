//! `structure.interference` の射影。寸法を持つ部品の箱（[`super::boxes`]。姿勢は取付の連鎖から）と、許容差の設定。
//! 寸法を持たない部品は一覧にするだけで、不足入力にはしない（箱が 2 つに満たなくても、重なり無しの成功になる）。

use super::acknowledged::acknowledgements;
use super::body_boxes::boxes;
use crate::context::Context;
use crate::missing::MissingInputs;
use crate::quantity::setting_opt;
use fune_model::design::FindingKind;
use fune_model::design::setting_dims as dims;
use sat_analysis::structure::interference::{InterferenceBox, InterferenceInput};

/// `structure.interference`: 全部品の箱と、許容差（`analysis/structure.toml` の `interference_tolerance`。無ければ解析の既定）。
pub fn interference(ctx: &Context<'_>) -> Result<InterferenceInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let body = boxes(ctx);
    let tolerance = ctx
        .design
        .analysis
        .structure
        .as_ref()
        .and_then(|s| s.interference_tolerance.as_ref());
    let tolerance = setting_opt(
        &dims::STRUCTURE_INTERFERENCE_TOLERANCE,
        &[],
        tolerance,
        &mut m,
    );
    let acknowledged = acknowledgements(ctx, FindingKind::Interference);
    m.finish(|| {
        Some(InterferenceInput {
            boxes: body.boxes.into_iter().map(InterferenceBox::from).collect(),
            unsized_parts: body.unsized_parts,
            tolerance,
            acknowledged,
        })
    })
}
