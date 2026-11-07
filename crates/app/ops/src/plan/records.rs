//! 解析ケース・出典・ハーネスの Op の解釈（ID の検査 → store の書き込み）。

use crate::error::OpError;
use crate::op::Op;
use fune_model::Design;
use fune_model::id::validate_entity_id;
use fune_store::Edits;
use fune_store::layout;
use fune_store::write::records::write_record;

pub(super) fn interpret(edits: &mut Edits, design: &Design, op: &Op) -> Result<(), OpError> {
    match op {
        Op::SetCase { case } => {
            valid_id(case.id.as_str())?;
            Ok(write_record(
                edits,
                &layout::case_file(case.id.as_str()),
                case,
            )?)
        }
        Op::RemoveCase { id } => remove(
            edits,
            design.cases.contains_key(id.as_str()),
            &layout::case_file(id),
            "解析ケース",
            id,
        ),
        Op::SetSource { source } => {
            valid_id(source.id.as_str())?;
            Ok(write_record(
                edits,
                &layout::source_file(source.id.as_str()),
                source,
            )?)
        }
        Op::RemoveSource { id } => remove(
            edits,
            design.sources.contains_key(id.as_str()),
            &layout::source_file(id),
            "出典",
            id,
        ),
        Op::SetHarness { harness } => {
            valid_id(harness.id.as_str())?;
            Ok(write_record(
                edits,
                &layout::harness_file(harness.id.as_str()),
                harness,
            )?)
        }
        Op::RemoveHarness { id } => remove(
            edits,
            design.harnesses.contains_key(id.as_str()),
            &layout::harness_file(id),
            "ハーネス",
            id,
        ),
        _ => Err(OpError::invalid("この Op はここでは扱いません")),
    }
}

fn valid_id(id: &str) -> Result<(), OpError> {
    validate_entity_id(id).map_err(|e| OpError::invalid(e.to_string()))
}

fn remove(
    edits: &mut Edits,
    exists: bool,
    file: &str,
    what: &str,
    id: &str,
) -> Result<(), OpError> {
    if !exists {
        return Err(OpError::not_found(format!("{what} `{id}`")));
    }
    edits.remove(file)?;
    Ok(())
}
