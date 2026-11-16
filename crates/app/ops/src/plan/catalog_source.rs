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
        _ => Err(OpError::invalid("カタログの出所の Op ではありません")),
    }
}

fn add(
    edits: &mut Edits,
    design: &Design,
    name: &str,
    git: &str,
    rev: Option<&str>,
) -> Result<(), OpError> {
    validate_entity_id(name)
        .map_err(|e| OpError::invalid(format!("カタログの名前が不正です: {e}")))?;
    let names = design.project.catalogs.iter().map(|c| c.name.as_str());
    if let Some(existing) = clash(names, name) {
        return Err(OpError::exists(format!("カタログ `{existing}`")));
    }
    if git.trim().is_empty() {
        return Err(OpError::invalid(
            "共有カタログの git（取得元の URL）が空です",
        ));
    }
    if rev.is_some_and(|r| r.trim().is_empty()) {
        return Err(OpError::invalid(
            "rev が空です（既定のブランチを使うなら rev を省いてください）",
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
        return Err(OpError::not_found(format!("カタログ `{name}`")));
    };
    if let Some(dir) = source.local_dir() {
        return Err(OpError::invalid(format!(
            "カタログ `{name}` はこのプロジェクトのカタログ（path = \"{dir}\"）なので外せません（外せるのは共有カタログ git = … だけです）"
        )));
    }
    let users = users(design.references(Referent::CatalogSource(name)));
    if !users.is_empty() {
        return Err(OpError::invalid(format!(
            "カタログ `{name}` の品は {} から使われているので外せません。先に使っている側を付け替えるか消してください",
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
                format!("部品 {part}")
            }
            RefSite::HarnessWire { harness } => format!("ハーネス {harness}"),
            _ => continue,
        };
        if !out.contains(&text) {
            out.push(text);
        }
    }
    out
}
