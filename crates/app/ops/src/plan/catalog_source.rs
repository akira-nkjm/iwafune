//! カタログの出所の Op の解釈（fune.toml の `[[catalog]]`。docs/model.md §12）。足せるのも外せるのも共有カタログ
//! （`git = …`）だけで、このプロジェクトのカタログ（`path = …`）は外さない。取得と catalog.lock は
//! `fune catalog fetch`（service の `shared_catalog`）の仕事で、ここでは触らない。

use crate::error::OpError;
use crate::op::Op;
use fune_model::Design;
use fune_model::design::{CatalogSource, RefSite, Referent};
use fune_model::id::{clash, validate_entity_id};
use fune_store::Edits;
use fune_store::write::project::{add_catalog_source, remove_catalog_source};

pub(super) fn interpret(edits: &mut Edits, design: &Design, op: &Op) -> Result<(), OpError> {
    match op {
        Op::AddCatalogSource { name, git, rev } => add(edits, design, name, git, rev.as_deref()),
        Op::RemoveCatalogSource { name } => remove(edits, design, name),
        _ => Err(OpError::invalid("Not a catalog source op")),
    }
}

fn add(
    edits: &mut Edits,
    design: &Design,
    name: &str,
    git: &str,
    rev: Option<&str>,
) -> Result<(), OpError> {
    validate_entity_id(name).map_err(|e| OpError::invalid(format!("Invalid catalog name: {e}")))?;
    let names = design.project.catalogs.iter().map(|c| c.name.as_str());
    if let Some(existing) = clash(names, name) {
        return Err(OpError::exists(format!("Catalog `{existing}`")));
    }
    if git.trim().is_empty() {
        return Err(OpError::invalid(
            "The git URL of the shared catalog is empty",
        ));
    }
    if rev.is_some_and(|r| r.trim().is_empty()) {
        return Err(OpError::invalid(
            "rev is empty (omit rev to use the default branch)",
        ));
    }
    let source = CatalogSource {
        name: name.to_owned(),
        path: None,
        git: Some(git.to_owned()),
        rev: rev.map(str::to_owned),
    };
    Ok(add_catalog_source(edits, &source)?)
}

fn remove(edits: &mut Edits, design: &Design, name: &str) -> Result<(), OpError> {
    let Some(source) = design.project.catalogs.iter().find(|c| c.name == name) else {
        return Err(OpError::not_found(format!("Catalog `{name}`")));
    };
    if let Some(dir) = source.local_dir() {
        return Err(OpError::invalid(format!(
            "Catalog `{name}` is this project's own catalog (path = \"{dir}\") and cannot be removed (only shared catalogs, git = …, can be removed)"
        )));
    }
    let users = users(design.references(Referent::CatalogSource(name)));
    if !users.is_empty() {
        return Err(OpError::invalid(format!(
            "Items of catalog `{name}` are used by {} so it cannot be removed. Re-point or delete the users first",
            users.join(", ")
        )));
    }
    remove_catalog_source(edits, name)?;
    Ok(())
}

/// 品を使っているものの呼び名（重ねない。部品は `catalog` と `candidates` の両方に書いていても 1 つ）。
fn users(sites: Vec<RefSite>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for site in sites {
        let text = match site {
            RefSite::PartCatalog { part } | RefSite::PartCandidate { part } => {
                format!("part {part}")
            }
            RefSite::HarnessWire { harness } => format!("harness {harness}"),
            _ => continue,
        };
        if !out.contains(&text) {
            out.push(text);
        }
    }
    out
}
