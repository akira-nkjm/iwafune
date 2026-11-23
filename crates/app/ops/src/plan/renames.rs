//! ネット・ハーネス・解析ケース・出典・マヌーバの名前の変更の Op の解釈（前提の確認 → store の書き換え）。
//! ID の規則と大文字小文字を無視した一意性は `others::new_flat_id`（モード・ネットと同じ）。参照の所在は model の索引
//! （`Design::references`）で、書き換えは `fune_store::write::refs`。名前の変更で参照が残るとすれば、それは索引の漏れ
//! （`tests/rename_records.rs` が、変更後に旧い ID の参照が無いことを 5 つのエンティティで確かめる）。

use super::defs::refuse_shared_users;
use super::others::new_flat_id;
use super::records::writable_source;
use crate::error::OpError;
use crate::op::Op;
use fune_model::Design;
use fune_model::design::Referent;
use fune_store::Edits;
use fune_store::write::refs;

pub(super) fn interpret(edits: &mut Edits, design: &Design, op: &Op) -> Result<(), OpError> {
    match op {
        Op::RenameNet { id, to } => {
            exists(design.nets.contains_key(id.as_str()), "Net", id)?;
            new_flat_id(ids(design.nets.keys()), to, "Net")?;
            Ok(refs::rename_net(edits, design, id, to)?)
        }
        Op::RenameHarness { id, to } => {
            exists(design.harnesses.contains_key(id.as_str()), "Harness", id)?;
            new_flat_id(ids(design.harnesses.keys()), to, "Harness")?;
            Ok(refs::rename_harness(edits, design, id, to)?)
        }
        Op::RenameCase { id, to } => {
            exists(design.cases.contains_key(id.as_str()), "Analysis case", id)?;
            new_flat_id(ids(design.cases.keys()), to, "Analysis case")?;
            Ok(refs::rename_case(edits, design, id, to)?)
        }
        Op::RenameManeuver { id, to } => {
            exists(design.maneuvers.contains_key(id.as_str()), "Maneuver", id)?;
            new_flat_id(ids(design.maneuvers.keys()), to, "Maneuver")?;
            Ok(refs::rename_maneuver(edits, design, id, to)?)
        }
        Op::RenameSource { id, to } => rename_source(edits, design, id, to),
        _ => Err(OpError::invalid("Not a rename op")),
    }
}

fn rename_source(edits: &mut Edits, design: &Design, id: &str, to: &str) -> Result<(), OpError> {
    writable_source(design, id)?;
    exists(design.sources.contains_key(id), "Source", id)?;
    new_flat_id(ids(design.sources.keys()), to, "Source")?;
    refuse_shared_users(
        "source",
        id,
        design.references(Referent::Source(id)),
        design,
    )?;
    Ok(refs::rename_source(edits, design, id, to)?)
}

/// ID の列（`Id<T>` → `&str`）。
fn ids<'a, T: 'a>(
    keys: impl Iterator<Item = &'a fune_model::Id<T>>,
) -> impl Iterator<Item = &'a str> {
    keys.map(fune_model::Id::as_str)
}

fn exists(found: bool, what: &str, id: &str) -> Result<(), OpError> {
    if found {
        Ok(())
    } else {
        Err(OpError::not_found(format!("{what} `{id}`")))
    }
}
