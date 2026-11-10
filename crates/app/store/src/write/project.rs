//! プロジェクトのファイル（`fune.toml`・`budget/allocation.toml`・`budget/policy.toml`）の新しい本文の正規形。
//! `fune init` の雛形もここで書き出す（手で TOML を組み立てない）。

use super::entities::{quantity, serde_text};
use crate::codec::float;
use crate::error::StoreError;
use crate::layout::{POLICY_FILE, PROJECT_FILE};
use crate::profile as format;
use fune_model::design::{Allocations, BudgetPolicy, Project};
use toml_edit::{DocumentMut, Item, Table};

/// 新しい `fune.toml`（serde の形のまま。`[groups]` は空でも書く）。
pub fn new_project_text(project: &Project) -> Result<String, StoreError> {
    serde_text(PROJECT_FILE, project)
}

/// 新しい `budget/allocation.toml`（種類ごとに `[<種類>]` の表。上限と配分は量のインライン表、`[<種類>.alloc]` は空でも書く）。
pub fn new_allocation_text(allocations: &Allocations) -> String {
    let mut doc = DocumentMut::new();
    for (kind, a) in allocations.iter() {
        let mut table = Table::new();
        table.insert("limit", quantity(&a.limit));
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

/// 新しい `budget/policy.toml`（serde の形のまま）。
pub fn new_policy_text(policy: &BudgetPolicy) -> Result<String, StoreError> {
    serde_text(POLICY_FILE, policy)
}
