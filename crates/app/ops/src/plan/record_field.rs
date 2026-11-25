//! `SetRecordField` の解釈: 名前のついたレコード（解析ケース・マヌーバ・要求・ハーネス・出典・フェーズの見出し）の欄を 1 つだけ書く。
//! 欄の名前と型の検査・書き換えは `fune_model::design::record_field`（ここは集まりごとにレコードを取り、書き戻すだけ）。
//! 書けない値（参照の先が無い・次元が合わない）は書いたあとの `check` が拒む。

use super::records::writable_source;
use crate::error::OpError;
use crate::op::Op;
use fune_model::Design;
use fune_model::design::{RecordKind, RecordValue, patched};
use fune_store::Edits;
use fune_store::layout;
use fune_store::write::records::{
    MANEUVER_SECTIONS, PHASE_SECTIONS, REQUIREMENT_SECTIONS, write_record_with_sections,
};
use serde::Serialize;
use serde::de::DeserializeOwned;

pub(super) fn interpret(edits: &mut Edits, design: &Design, op: &Op) -> Result<(), OpError> {
    match op {
        Op::SetRecordField {
            record,
            id,
            field,
            value,
        } => set_record_field(edits, design, *record, id, field, value.as_ref()),
        _ => Err(OpError::invalid("This op is not handled here")),
    }
}

/// レコード `id`（`kind` の集まり）の欄 `field` だけを書き換えて書き戻す。
fn set_record_field(
    edits: &mut Edits,
    design: &Design,
    kind: RecordKind,
    id: &str,
    field: &str,
    value: Option<&RecordValue>,
) -> Result<(), OpError> {
    kind.field(field)
        .map_err(|e| OpError::invalid(e.to_string()))?;
    let missing = |what: &str| OpError::not_found(format!("{what} `{id}`"));
    match kind {
        RecordKind::Case => {
            let current = design
                .cases
                .get(id)
                .ok_or_else(|| missing("Analysis case"))?;
            rewrite(edits, &layout::case_file(id), current, field, value, &[])
        }
        RecordKind::Maneuver => {
            let current = design
                .maneuvers
                .get(id)
                .ok_or_else(|| missing("Maneuver"))?;
            let file = layout::maneuver_file(id);
            rewrite(edits, &file, current, field, value, MANEUVER_SECTIONS)
        }
        RecordKind::Requirement => {
            let current = design
                .requirements
                .get(id)
                .ok_or_else(|| missing("Requirement"))?;
            let file = layout::requirement_file(id);
            rewrite(edits, &file, current, field, value, REQUIREMENT_SECTIONS)
        }
        RecordKind::Harness => {
            let current = design.harnesses.get(id).ok_or_else(|| missing("Harness"))?;
            rewrite(edits, &layout::harness_file(id), current, field, value, &[])
        }
        RecordKind::Source => {
            writable_source(design, id)?;
            let current = design.sources.get(id).ok_or_else(|| missing("Source"))?;
            let file = layout::source_file(design, id);
            rewrite(edits, &file, current, field, value, &[])
        }
        RecordKind::Phase => {
            let current = design.phases.get(id).ok_or_else(|| missing("Phase"))?;
            let file = layout::phase_file(id);
            rewrite(edits, &file, current, field, value, PHASE_SECTIONS)
        }
    }
}

fn rewrite<T: Serialize + DeserializeOwned>(
    edits: &mut Edits,
    file: &str,
    current: &T,
    field: &str,
    value: Option<&RecordValue>,
    sections: &[&str],
) -> Result<(), OpError> {
    let updated = patched(current, field, value).map_err(|e| OpError::invalid(e.to_string()))?;
    Ok(write_record_with_sections(edits, file, &updated, sections)?)
}
