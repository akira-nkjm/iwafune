//! 解析ケース・出典・ハーネスの Op の解釈（ID の検査 → store の書き込み）。

use crate::error::OpError;
use crate::op::Op;
use fune_model::Design;
use fune_model::id::{clash, validate_entity_id};
use fune_store::Edits;
use fune_store::layout;
use fune_store::write::records::write_record;

pub(super) fn interpret(edits: &mut Edits, design: &Design, op: &Op) -> Result<(), OpError> {
    match op {
        Op::SetCase { case } => {
            let id = case.id.as_str();
            set_id(design.cases.keys().map(|k| k.as_str()), id, "解析ケース")?;
            Ok(write_record(edits, &layout::case_file(id), case)?)
        }
        Op::RemoveCase { id } => remove(
            edits,
            design.cases.contains_key(id.as_str()),
            &layout::case_file(id),
            "解析ケース",
            id,
        ),
        Op::SetSource { source } => {
            let id = source.id.as_str();
            writable_source(design, id)?;
            set_id(design.sources.keys().map(|k| k.as_str()), id, "出典")?;
            Ok(write_record(edits, &layout::source_file(id), source)?)
        }
        Op::RemoveSource { id } => writable_source(design, id).and_then(|()| {
            remove(
                edits,
                design.sources.contains_key(id.as_str()),
                &layout::source_file(id),
                "出典",
                id,
            )
        }),
        Op::SetHarness { harness } => {
            let id = harness.id.as_str();
            set_id(design.harnesses.keys().map(|k| k.as_str()), id, "ハーネス")?;
            Ok(write_record(edits, &layout::harness_file(id), harness)?)
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

/// 共有カタログの出典（`<カタログ名>/<ID>`）は読み取り専用（共有カタログのリポジトリで直す）。
fn writable_source(design: &Design, id: &str) -> Result<(), OpError> {
    match design.shared_catalog_of(id) {
        Some(c) => Err(OpError::invalid(format!(
            "出典 `{id}` は共有カタログ `{}` の出典なので、このプロジェクトからは書き換えられません。共有カタログのリポジトリに PR を出してください",
            c.name
        ))),
        None => Ok(()),
    }
}

/// 書き込む（無ければ作る）ID として使えるか。同じ綴りの既存のものは書き換え、大文字小文字だけが違う既存の ID とは
/// 重ねない（一意性の規則は `id::clash`。部品・モード・ネット・カタログ品と同じ）。
fn set_id<'a>(
    existing: impl Iterator<Item = &'a str>,
    id: &str,
    what: &str,
) -> Result<(), OpError> {
    validate_entity_id(id).map_err(|e| OpError::invalid(e.to_string()))?;
    let existing: Vec<&str> = existing.collect();
    if existing.contains(&id) {
        return Ok(());
    }
    match clash(existing, id) {
        Some(other) => Err(OpError::exists(format!(
            "{what} `{id}`（`{other}` と大文字小文字だけが違います）"
        ))),
        None => Ok(()),
    }
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
