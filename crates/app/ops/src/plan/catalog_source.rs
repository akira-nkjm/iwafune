//! カタログの出所の Op の解釈（fune.toml の `[[catalog]]`。docs/model.md §12）。共有カタログ（`git = …`。`AddCatalogSource`）と
//! このプロジェクトの中のカタログ（`path = …`。`AddLocalCatalog`）を足し、`RemoveCatalogSource` はどちらも外す
//! （ローカルのカタログは、品が 1 つも無いときだけ。ファイルを残して出所だけ外すと品が読めなくなるため）。
//! 取得と catalog.lock は `fune catalog fetch`（service の `shared_catalog`）の仕事で、ここでは触らない。

use crate::error::OpError;
use crate::op::Op;
use fune_model::Design;
use fune_model::design::{CatalogSource, RefSite, Referent};
use fune_model::id::{clash, validate_entity_id};
use fune_model::path::split_catalog_ref;
use fune_store::Edits;
use fune_store::layout::parse_local_catalog_path;
use fune_store::write::project::{add_catalog_source, remove_catalog_source};

pub(super) fn interpret(edits: &mut Edits, design: &Design, op: &Op) -> Result<(), OpError> {
    match op {
        Op::AddCatalogSource { name, git, rev } => add(edits, design, name, git, rev.as_deref()),
        Op::AddLocalCatalog { name, path } => add_local(edits, design, name, path),
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
    new_catalog_name(design, name)?;
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

/// 新しいカタログの名前として使えるか（ID の規則と、既にあるカタログ（`path` のものも）との大文字小文字を無視した一意性）。
/// **カタログの名前の検査はここだけ**（共有・ローカルの追加が使う）。
fn new_catalog_name(design: &Design, name: &str) -> Result<(), OpError> {
    validate_entity_id(name).map_err(|e| OpError::invalid(format!("Invalid catalog name: {e}")))?;
    let names = design.project.catalogs.iter().map(|c| c.name.as_str());
    if let Some(existing) = clash(names, name) {
        return Err(OpError::exists(format!("Catalog `{existing}`")));
    }
    Ok(())
}

/// このプロジェクトの中のカタログを足す。置き場所の読み方（規則・予約された場所・ほかのカタログとの入れ子）は
/// `fune_store::layout::parse_local_catalog_path`。ディレクトリは作らない（品を足すとき作られる）。
fn add_local(edits: &mut Edits, design: &Design, name: &str, path: &str) -> Result<(), OpError> {
    new_catalog_name(design, name)?;
    let path = parse_local_catalog_path(design, path).map_err(OpError::invalid)?;
    let source = CatalogSource {
        name: name.to_owned(),
        path: Some(path),
        git: None,
        rev: None,
    };
    Ok(add_catalog_source(edits, &source)?)
}

fn remove(edits: &mut Edits, design: &Design, name: &str) -> Result<(), OpError> {
    let Some(source) = design.project.catalogs.iter().find(|c| c.name == name) else {
        return Err(OpError::not_found(format!("Catalog `{name}`")));
    };
    if let Some(dir) = source.local_dir() {
        let items = design
            .catalog
            .keys()
            .filter(|r| split_catalog_ref(r).is_some_and(|(s, _)| s == name))
            .count();
        if items > 0 {
            return Err(OpError::invalid(format!(
                "Catalog `{name}` (path = \"{dir}\") still holds {items} item(s), so it cannot be removed. \
                 Remove the items first (fune catalog rm), because their files would otherwise stay unread"
            )));
        }
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
