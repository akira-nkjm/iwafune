//! Op の列 → ファイルの変更計画（適用後の検証込み）。**書き込みはしない。**
//!
//! Op を 1 つずつ解釈して store の編集（[`fune_store::Edits`]）に変え、そのたびに Design を読み直す。
//! 最後に適用前と適用後の `check` を比べ、新しく増えた error があれば [`OpError::Rejected`] で拒む
//! （元からある error は編集を妨げない）。
//!
//! | モジュール | 中身 |
//! |---|---|
//! | `parts` | 部品の Op（追加・削除・名前・移動・値・フィールド・取付・ポート） |
//! | `others` | 定義・モード・ネット・図・設定の Op |
//! | `records` | 解析ケース・出典・ハーネスの Op |
//! | `catalog` | カタログ品の Op（このプロジェクトの `catalog/` だけ） |
//! | `attr` | 属性の値の書き込み（部品とカタログ品で共有） |

mod attr;
mod catalog;
mod others;
mod parts;
mod records;

use crate::error::OpError;
use crate::op::Op;
use fune_model::design::Revision;
use fune_model::{Design, Diagnostic, check};
use fune_store::read::design_of;
use fune_store::{Edits, FileChange, Snapshot, revision};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use ts_rs::TS;

/// 変更計画。
#[derive(Debug, Clone)]
pub struct Plan {
    /// 計画の元にした revision
    pub base_revision: Revision,
    /// 適用後の revision
    pub revision: Revision,
    pub changes: Vec<FileChange>,
    /// 適用後の設計の診断（全件）
    pub diagnostics: Vec<Diagnostic>,
    /// 適用後の設計
    pub design: Design,
}

/// 計画の要約（API・CLI の `--dry-run` 向け）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct PlanSummary {
    pub base_revision: Revision,
    pub revision: Revision,
    pub changes: Vec<FileChange>,
    pub diagnostics: Vec<Diagnostic>,
}

impl From<&Plan> for PlanSummary {
    fn from(plan: &Plan) -> Self {
        Self {
            base_revision: plan.base_revision.clone(),
            revision: plan.revision.clone(),
            changes: plan.changes.clone(),
            diagnostics: plan.diagnostics.clone(),
        }
    }
}

/// Op の列を計画する。
pub fn plan(snapshot: &Snapshot, ops: &[Op]) -> Result<Plan, OpError> {
    let mut edits = Edits::new(&snapshot.files);
    let mut design = snapshot.design.clone();
    for op in ops {
        interpret(&mut edits, &design, op)?;
        design = design_of(edits.files());
    }
    let before = check(&snapshot.design);
    let after = check(&design);
    let new_errors = new_errors(&before, &after);
    if !new_errors.is_empty() {
        return Err(OpError::Rejected {
            diagnostics: new_errors,
        });
    }
    Ok(Plan {
        base_revision: snapshot.design.revision.clone(),
        revision: revision::of(edits.files()),
        changes: edits.changes(),
        diagnostics: after,
        design,
    })
}

/// 適用後に新しく増えた error（位置は比べない。編集で行がずれ、名前の変更でファイルも変わるため）。
fn new_errors(before: &[Diagnostic], after: &[Diagnostic]) -> Vec<Diagnostic> {
    let key = |d: &Diagnostic| (d.code.clone(), d.target.clone(), d.message.clone());
    let old: BTreeSet<_> = before.iter().filter(|d| d.is_error()).map(key).collect();
    after
        .iter()
        .filter(|d| d.is_error() && !old.contains(&key(d)))
        .cloned()
        .collect()
}

/// Op 1 つを編集に変える。
fn interpret(edits: &mut Edits, design: &Design, op: &Op) -> Result<(), OpError> {
    match op {
        Op::CreatePart { .. }
        | Op::RemovePart { .. }
        | Op::RenamePart { .. }
        | Op::RenameGroup { .. }
        | Op::MovePart { .. }
        | Op::SetValue { .. }
        | Op::UnsetValue { .. }
        | Op::SetField { .. }
        | Op::Place { .. }
        | Op::SetPort { .. }
        | Op::SetMountPoint { .. } => parts::interpret(edits, design, op),
        Op::CreateCatalogItem { .. }
        | Op::SetCatalogValue { .. }
        | Op::UnsetCatalogValue { .. }
        | Op::SetCatalogField { .. }
        | Op::RemoveCatalogItem { .. }
        | Op::RenameCatalogItem { .. } => catalog::interpret(edits, design, op),
        Op::SetCase { .. }
        | Op::RemoveCase { .. }
        | Op::SetSource { .. }
        | Op::RemoveSource { .. }
        | Op::SetHarness { .. }
        | Op::RemoveHarness { .. }
        | Op::CreateManeuver { .. }
        | Op::SetManeuver { .. }
        | Op::RemoveManeuver { .. } => records::interpret(edits, design, op),
        _ => others::interpret(edits, design, op),
    }
}
