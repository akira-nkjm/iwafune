//! Op の列 → ファイルの変更計画（適用後の検証込み）。**書き込みはしない。**
//!
//! Op を 1 つずつ解釈して store の編集（[`fune_store::Edits`]）に変え、そのたびに Design を読み直す。
//! 最後に適用前と適用後の `check` を比べ、新しく増えた error があれば [`OpError::Rejected`] で拒む
//! （元からある error と、取得で直る共有カタログの状態 [`fune_model::check::FETCH_STATE_CODES`] は編集を妨げない）。
//!
//! | モジュール | 中身 |
//! |---|---|
//! | `parts` | 部品の Op（追加・削除・名前・移動・値・フィールド・取付・ポート） |
//! | `defs` | 定義（属性・種別）の Op（足す・置き換える・消す） |
//! | `others` | モード・ネット・図・設定の Op |
//! | `records` | 解析ケース・出典・ハーネスの Op |
//! | `requirements` | 要求の Op（足す・書く・消す） |
//! | `catalog` | カタログ品の Op（このプロジェクトの `catalog/` だけ） |
//! | `catalog_source` | カタログの出所の Op（fune.toml の共有カタログ `[[catalog]] git = …` を足す・外す） |
//! | `subdir` | ディレクトリの下に置けるエンティティ（カタログ品・出典）の Op に共通の計画（ディレクトリを選ぶ・移す） |
//! | `attr` | 属性の値の書き込み（部品とカタログ品で共有） |

mod attr;
mod catalog;
mod catalog_source;
mod defs;
mod others;
mod parts;
mod records;
mod requirements;
mod subdir;

use crate::error::OpError;
use crate::op::Op;
use fune_model::check::FETCH_STATE_CODES;
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
/// 取得で直る共有カタログの状態（共有カタログを足した直後の `catalog.not_locked` など）は数えない。
fn new_errors(before: &[Diagnostic], after: &[Diagnostic]) -> Vec<Diagnostic> {
    let key = |d: &Diagnostic| (d.code.clone(), d.target.clone(), d.message.clone());
    let old: BTreeSet<_> = before.iter().filter(|d| d.is_error()).map(key).collect();
    after
        .iter()
        .filter(|d| d.is_error() && !FETCH_STATE_CODES.contains(&d.code.as_str()))
        .filter(|d| !old.contains(&key(d)))
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
        Op::CreateAttribute { .. }
        | Op::UpdateAttribute { .. }
        | Op::DeleteAttribute { .. }
        | Op::CreateKind { .. }
        | Op::UpdateKind { .. }
        | Op::DeleteKind { .. } => defs::interpret(edits, design, op),
        Op::CreateCatalogItem { .. }
        | Op::SetCatalogValue { .. }
        | Op::UnsetCatalogValue { .. }
        | Op::SetCatalogField { .. }
        | Op::RemoveCatalogItem { .. }
        | Op::RenameCatalogItem { .. }
        | Op::MoveCatalogItem { .. } => catalog::interpret(edits, design, op),
        Op::AddCatalogSource { .. } | Op::RemoveCatalogSource { .. } => {
            catalog_source::interpret(edits, design, op)
        }
        Op::SetCase { .. }
        | Op::RemoveCase { .. }
        | Op::SetSource { .. }
        | Op::RemoveSource { .. }
        | Op::MoveSource { .. }
        | Op::SetHarness { .. }
        | Op::RemoveHarness { .. }
        | Op::CreateManeuver { .. }
        | Op::SetManeuver { .. }
        | Op::RemoveManeuver { .. } => records::interpret(edits, design, op),
        Op::CreateRequirement { .. } | Op::SetRequirement { .. } | Op::RemoveRequirement { .. } => {
            requirements::interpret(edits, design, op)
        }
        _ => others::interpret(edits, design, op),
    }
}
