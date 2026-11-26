//! 解析ケース・出典・ハーネス・マヌーバの Op の解釈（ID の検査 → store の書き込み）。

use super::subdir::{move_file, placed_file};
use crate::error::OpError;
use crate::op::Op;
use fune_model::Design;
use fune_model::design::{Maneuver, RefSite, Referent, Source};
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
        Op::RemoveCase { id } => {
            refuse_if_case_referenced(design, id)?;
            remove(
                edits,
                design.cases.contains_key(id.as_str()),
                &layout::case_file(id),
                "Analysis case",
                id,
            )
        }
        Op::SetSource { source, dir } => set_source(edits, design, source, dir.as_deref()),
        Op::RemoveSource { id } => writable_source(design, id).and_then(|()| {
            remove(
                edits,
                design.sources.contains_key(id.as_str()),
                &layout::source_file(design, id),
                "Source",
                id,
            )
        }),
        Op::MoveSource { id, dir } => move_source(edits, design, id, dir),
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

/// 解析ケースを指しているもの（既定のケース・要求の `quantify.cases`）があれば、それを挙げて消すのを拒む。
/// 参照元の索引は model の `Design::references`。
fn refuse_if_case_referenced(design: &Design, id: &str) -> Result<(), OpError> {
    if !design.cases.contains_key(id) {
        return Ok(());
    }
    let mut parts = Vec::new();
    let mut requirements = Vec::new();
    for site in design.references(Referent::Case(id)) {
        match site {
            RefSite::CommonDefaultCase => parts.push(
                "it is the default case (`default_case` in analysis/common.toml)".to_string(),
            ),
            RefSite::RequirementCase { requirement } => requirements.push(requirement),
            _ => {}
        }
    }
    if !requirements.is_empty() {
        parts.push(format!(
            "`quantify.cases` of requirement(s) {} refers to it",
            requirements.join(", ")
        ));
    }
    if parts.is_empty() {
        return Ok(());
    }
    Err(OpError::invalid(format!(
        "Analysis case `{id}` cannot be removed: {}. Change those first",
        parts.join("; ")
    )))
}

/// 共有カタログの出典（`<カタログ名>/<ID>`）は読み取り専用（共有カタログのリポジトリで直す）。
pub(super) fn writable_source(design: &Design, id: &str) -> Result<(), OpError> {
    match design.shared_catalog_of(id) {
        Some(c) => Err(OpError::invalid(format!(
            "Source `{id}` belongs to shared catalog `{}`, so it cannot be edited from this project. Open a PR on the shared catalog repository",
            c.name
        ))),
        None => Ok(()),
    }
}

/// 出典を書く。既にあれば今のファイルに、無ければ `dir`（`sources/` からの相対。省けば直下）の下に作る。
fn set_source(
    edits: &mut Edits,
    design: &Design,
    source: &Source,
    dir: Option<&str>,
) -> Result<(), OpError> {
    let id = source.id.as_str();
    writable_source(design, id)?;
    set_id(design.sources.keys().map(|k| k.as_str()), id, "Source")?;
    let current = design
        .sources
        .contains_key(id)
        .then(|| layout::source_file(design, id));
    let hint = format!("fune source mv {id} <DIR>");
    let file = placed_file(
        layout::SOURCES_DIR,
        current.as_deref(),
        dir,
        id,
        (&format!("Source `{id}`"), &hint),
    )?;
    Ok(write_record(edits, &file, source)?)
}

/// 出典を別のディレクトリへ移す（ID は変わらないので、参照は書き換えない）。
fn move_source(edits: &mut Edits, design: &Design, id: &str, dir: &str) -> Result<(), OpError> {
    writable_source(design, id)?;
    if !design.sources.contains_key(id) {
        return Err(OpError::not_found(format!("Source `{id}`")));
    }
    let file = layout::source_file(design, id);
    move_file(
        edits,
        &format!("Source `{id}`"),
        &file,
        layout::SOURCES_DIR,
        id,
        dir,
    )
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
pub(super) fn set_id<'a>(
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

pub(super) fn remove(
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
