//! `structure.external_geometry` の射影。寸法を持つ部品の箱（[`super::boxes`]。姿勢は取付の連鎖から。形態を選べれば、その配置に追従する）。
//! 箱が 1 つも無ければ、外形は出せないので入力の不足にする（寸法を持たない部品は一覧にするだけで、不足入力にはしない）。

use super::body_boxes::boxes;
use crate::context::Context;
use crate::missing::MissingInputs;
use fune_model::Target;
use sat_analysis::structure::ExternalGeometryInput;
use sat_analysis::structure::interference::InterferenceBox;

/// `structure.external_geometry`: 全部品の箱。
pub fn external_geometry(ctx: &Context<'_>) -> Result<ExternalGeometryInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let body = boxes(ctx);
    if body.boxes.is_empty() {
        m.push(
            Target::any_part("size"),
            "A part with a size (its box makes the external geometry)",
        );
    }
    m.finish(|| {
        Some(ExternalGeometryInput {
            boxes: body.boxes.into_iter().map(InterferenceBox::from).collect(),
            unsized_parts: body.unsized_parts,
        })
    })
}
