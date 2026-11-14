//! プロジェクトのファイル（`fune.toml`・`budget/allocation.toml`・`budget/policy.toml`・`catalog.lock`）の新しい本文の正規形。
//! `fune init` の雛形もここで書き出す（手で TOML を組み立てない）。

use super::entities::{quantity, serde_text};
use crate::codec::float;
use crate::error::StoreError;
use crate::layout::{LOCK_FILE, POLICY_FILE, PROJECT_FILE};
use crate::profile as format;
use fune_model::design::{Allocations, BudgetPolicy, CatalogLock, Project};
use toml_edit::{DocumentMut, Item, Table};

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
const LOCK_HEADER: &str = "# fune catalog fetch・fune catalog update が書きます（手で編集しない）。git で管理してください。\n";

/// `catalog.lock` の本文（先頭の注記と、serde の形のままの `[[catalog]]`）。記録が無ければ注記だけ。
pub fn catalog_lock_text(lock: &CatalogLock) -> Result<String, StoreError> {
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
