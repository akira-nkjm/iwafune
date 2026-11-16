//! プロジェクトのファイル（`fune.toml`・`budget/allocation.toml`・`budget/policy.toml`・`catalog.lock`）の新しい本文の正規形。
//! `fune init` の雛形もここで書き出す（手で TOML を組み立てない）。

use super::entities::{quantity, serde_text};
use crate::codec::float;
use crate::error::StoreError;
use crate::layout::{LOCK_FILE, POLICY_FILE, PROJECT_FILE};
use crate::profile as format;
use fune_model::design::{Allocations, BudgetPolicy, CatalogLock, CatalogSource, Project};
use toml_edit::{ArrayOfTables, DocumentMut, Item, Table};

/// 新しい `fune.toml`（serde の形のまま）。
pub fn new_project_text(project: &Project) -> Result<String, StoreError> {
    serde_text(PROJECT_FILE, project)
}

/// 新しい `budget/allocation.toml`（種類ごとに `[<種類>]` の表。上限と配分は量のインライン表、`[<種類>.alloc]` は空でも書く）。
pub fn new_allocation_text(allocations: &Allocations) -> String {
    let mut doc = DocumentMut::new();
    for (kind, a) in allocations.iter() {
        let mut table = Table::new();
        if let Some(limit) = &a.limit {
            table.insert("limit", quantity(limit));
        }
        table.insert("system_margin", Item::Value(float(a.system_margin)));
        let mut alloc = Table::new();
        for (node, q) in &a.alloc {
            alloc.insert(node, quantity(q));
        }
        table.insert("alloc", Item::Table(alloc));
        doc.insert(kind.as_str(), Item::Table(table));
    }
    format::finish(doc)
}

/// lock の先頭の注記。
const LOCK_HEADER: &str = "# Written by fune catalog fetch and fune catalog update (do not edit by hand). Keep it under git.\n";

/// `catalog.lock` の本文（先頭の注記と、serde の形のままの `[[catalog]]`）。記録が無ければ注記だけ。
pub fn catalog_lock_text(lock: &CatalogLock) -> Result<String, StoreError> {
    if lock.catalogs.is_empty() {
        // serde の形のままだと `catalog = []` になる（共有カタログを全部外したあとの fetch）
        return Ok(LOCK_HEADER.to_owned());
    }
    let body = serde_text(LOCK_FILE, lock)?;
    let body = body.trim_start_matches('\n');
    Ok(if body.trim().is_empty() {
        LOCK_HEADER.to_owned()
    } else {
        format!("{LOCK_HEADER}\n{body}")
    })
}

/// 新しい `budget/policy.toml`（serde の形のまま）。
pub fn new_policy_text(policy: &BudgetPolicy) -> Result<String, StoreError> {
    serde_text(POLICY_FILE, policy)
}

/// `fune.toml` に `[[catalog]]` を 1 つ足す（今の `[[catalog]]` の後ろ。キーは serde の形の順）。ほかの行とコメントは残し、
/// `=` の揃えは正規形（`fune fmt`）に合わせる。名前の重複などの検査は呼ぶ側（fune-ops）。
pub fn add_catalog_source(
    edits: &mut crate::Edits,
    source: &CatalogSource,
) -> Result<(), StoreError> {
    let text =
        toml::to_string(source).map_err(|e| StoreError::edit(PROJECT_FILE, e.to_string()))?;
    let parsed = super::doc::parse(PROJECT_FILE, &text)?;
    let mut table = Table::new();
    for (key, item) in parsed.iter() {
        table.insert(key, item.clone());
    }
    edits.edit(PROJECT_FILE, |doc| {
        let entry = doc
            .as_table_mut()
            .entry("catalog")
            .or_insert_with(|| Item::ArrayOfTables(ArrayOfTables::new()));
        let array = entry.as_array_of_tables_mut().ok_or_else(|| {
            StoreError::edit(
                PROJECT_FILE,
                "`catalog` is not an array of [[catalog]] tables",
            )
        })?;
        array.push(table);
        refinish(doc)
    })
}

/// `fune.toml` の `[[catalog]]` のうち `name` のものを外す（最後の 1 つなら `catalog` のキーごと）。無ければ false。
/// `catalog.lock` の記録は書き換えない（次の `fune catalog fetch` が外す。docs/model.md §12.3）。
pub fn remove_catalog_source(edits: &mut crate::Edits, name: &str) -> Result<bool, StoreError> {
    edits.edit(PROJECT_FILE, |doc| {
        let Some(array) = doc
            .get_mut("catalog")
            .and_then(Item::as_array_of_tables_mut)
        else {
            return Ok(false);
        };
        let Some(index) = array
            .iter()
            .position(|t| t.get("name").and_then(Item::as_str) == Some(name))
        else {
            return Ok(false);
        };
        array.remove(index);
        if array.is_empty() {
            doc.remove("catalog");
        }
        refinish(doc)?;
        Ok(true)
    })
}

/// 編集した `fune.toml` を正規形の揃えで読み直す。
fn refinish(doc: &mut DocumentMut) -> Result<(), StoreError> {
    *doc = crate::profile::parse(PROJECT_FILE, &format::finish(doc.clone()))?;
    Ok(())
}

/// 表示順を保存し、実際の順序が ID 順なら表を省く（欠けた ID の警告は残す）。
pub fn set_mode_order(edits: &mut crate::Edits, order: &[String]) -> Result<(), StoreError> {
    let mut design = crate::read::design_of(edits.files());
    design.project.modes = Some(fune_model::design::ModeOrder {
        order: order.to_vec(),
    });
    edits.edit(PROJECT_FILE, |doc| {
        super::doc::set(
            super::doc::table_mut(doc, &["modes"])?,
            "order",
            super::doc::strings(order.iter().map(String::as_str)),
        );
        normalize_mode_order(doc, &design);
        *doc = crate::profile::parse(PROJECT_FILE, &format::finish(doc.clone()))?;
        Ok(())
    })
}

/// モードの一覧が ID 順と同じ意味なら省く。存在しない ID は警告のため保持する。
pub fn normalize_mode_order(doc: &mut DocumentMut, design: &fune_model::Design) {
    let Some(modes) = &design.project.modes else {
        return;
    };
    let missing = modes
        .order
        .iter()
        .any(|id| !design.modes.contains_key(id.as_str()));
    if !missing
        && design
            .ordered_modes()
            .iter()
            .map(|m| &m.id)
            .eq(design.modes.keys())
    {
        doc.remove("modes");
    }
}
