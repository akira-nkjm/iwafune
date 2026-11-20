//! 要求の Op の解釈（ID の検査 → store の書き込み）。参照（出典・ケース・解析）の実在は適用後の `check` が見る。

use super::records::{remove, set_id};
use crate::error::OpError;
use crate::op::Op;
use fune_model::Design;
use fune_store::Edits;
use fune_store::layout;
use fune_store::write::records::{REQUIREMENT_SECTIONS, write_record_with_sections};

pub(super) fn interpret(edits: &mut Edits, design: &Design, op: &Op) -> Result<(), OpError> {
    match op {
        Op::CreateRequirement { requirement } => {
            let id = requirement.id.as_str();
            if design.requirements.contains_key(id) {
                return Err(OpError::exists(format!("Requirement `{id}`")));
            }
            set(edits, design, requirement)
        }
        Op::SetRequirement { requirement } => set(edits, design, requirement),
        Op::RemoveRequirement { id } => remove(
            edits,
            design.requirements.contains_key(id.as_str()),
            &layout::requirement_file(id),
            "Requirement",
            id,
        ),
        _ => Err(OpError::invalid("This op is not handled here")),
    }
}

fn set(
    edits: &mut Edits,
    design: &Design,
    r: &fune_model::design::Requirement,
) -> Result<(), OpError> {
    let id = r.id.as_str();
    set_id(
        design.requirements.keys().map(|k| k.as_str()),
        id,
        "Requirement",
    )?;
    let file = layout::requirement_file(id);
    Ok(write_record_with_sections(
        edits,
        &file,
        r,
        REQUIREMENT_SECTIONS,
    )?)
}
