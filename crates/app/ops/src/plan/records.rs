//! 解析ケース・出典・ハーネス・マヌーバの Op の解釈（ID の検査 → store の書き込み）。

use crate::error::OpError;
use crate::op::Op;
use fune_model::Design;
use fune_model::design::Maneuver;
use fune_model::id::{clash, validate_entity_id};
use fune_store::Edits;
use fune_store::layout;
use fune_store::write::records::{MANEUVER_SECTIONS, write_record, write_record_with_sections};

pub(super) fn interpret(edits: &mut Edits, design: &Design, op: &Op) -> Result<(), OpError> {
    match op {
        Op::SetCase { case } => {
            let id = case.id.as_str();
            set_id(design.cases.keys().map(|k| k.as_str()), id, "Analysis case")?;
            Ok(write_record(edits, &layout::case_file(id), case)?)
        }
        Op::RemoveCase { id } => remove(
            edits,
            design.cases.contains_key(id.as_str()),
            &layout::case_file(id),
            "Analysis case",
            id,
        ),
        Op::SetSource { source } => {
            let id = source.id.as_str();
            writable_source(design, id)?;
            set_id(design.sources.keys().map(|k| k.as_str()), id, "Source")?;
            Ok(write_record(edits, &layout::source_file(id), source)?)
        }
        Op::RemoveSource { id } => writable_source(design, id).and_then(|()| {
            remove(
                edits,
                design.sources.contains_key(id.as_str()),
                &layout::source_file(id),
                "Source",
                id,
            )
        }),
        Op::SetHarness { harness } => {
            let id = harness.id.as_str();
            set_id(design.harnesses.keys().map(|k| k.as_str()), id, "Harness")?;
            Ok(write_record(edits, &layout::harness_file(id), harness)?)
        }
        Op::RemoveHarness { id } => remove(
            edits,
            design.harnesses.contains_key(id.as_str()),
            &layout::harness_file(id),
            "Harness",
            id,
        ),
        Op::CreateManeuver { maneuver } => {
            let id = maneuver.id.as_str();
            if design.maneuvers.contains_key(id) {
                return Err(OpError::exists(format!("Maneuver `{id}`")));
            }
            set_maneuver(edits, design, maneuver)
        }
        Op::SetManeuver { maneuver } => set_maneuver(edits, design, maneuver),
        Op::RemoveManeuver { id } => remove_maneuver(edits, design, id),
        _ => Err(OpError::invalid("This op is not handled here")),
    }
}

/// 共有カタログの出典（`<カタログ名>/<ID>`）は読み取り専用（共有カタログのリポジトリで直す）。
fn writable_source(design: &Design, id: &str) -> Result<(), OpError> {
    match design.shared_catalog_of(id) {
        Some(c) => Err(OpError::invalid(format!(
            "Source `{id}` belongs to shared catalog `{}`, so it cannot be edited from this project. Open a PR on the shared catalog repository",
            c.name
        ))),
        None => Ok(()),
    }
}

fn set_maneuver(edits: &mut Edits, design: &Design, m: &Maneuver) -> Result<(), OpError> {
    let id = m.id.as_str();
    set_id(design.maneuvers.keys().map(|k| k.as_str()), id, "Maneuver")?;
    let file = layout::maneuver_file(id);
    Ok(write_record_with_sections(
        edits,
        &file,
        m,
        MANEUVER_SECTIONS,
    )?)
}

/// マヌーバを消し、それを `after` にしていたマヌーバを、消したマヌーバの `after` につなぎ直す。
fn remove_maneuver(edits: &mut Edits, design: &Design, id: &str) -> Result<(), OpError> {
    let Some(gone) = design.maneuvers.get(id) else {
        return Err(OpError::not_found(format!("Maneuver `{id}`")));
    };
    edits.remove(&layout::maneuver_file(id))?;
    for m in design
        .maneuvers
        .values()
        .filter(|m| m.after.as_deref() == Some(id))
    {
        let relinked = Maneuver {
            after: gone.after.clone(),
            ..m.clone()
        };
        let file = layout::maneuver_file(m.id.as_str());
        write_record_with_sections(edits, &file, &relinked, MANEUVER_SECTIONS)?;
    }
    Ok(())
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
            "{what} `{id}` (differs from `{other}` only in letter case)"
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
