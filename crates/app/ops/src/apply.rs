//! 変更計画を store で書き込む（ロック → 読み込み → revision の確認 → 計画 → 反映 → 履歴に控える）。

use crate::cli::to_cli;
use crate::error::OpError;
use crate::op::Op;
use crate::plan::{PlanSummary, plan};
use fune_model::design::Revision;
use fune_store::journal::{self, Journal};
use fune_store::lock::ProjectLock;
use fune_store::{commit, load};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;
use ts_rs::TS;

/// ほかのプロセスの書き込みを待つ時間。
pub const LOCK_WAIT: Duration = Duration::from_secs(5);

/// 適用の指定。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
pub struct ApplyOptions {
    /// 読んだときの revision。今の revision と違えば [`OpError::Conflict`]（None なら確かめない）
    #[serde(default)]
    pub base_revision: Option<Revision>,
    /// true なら書き込まずに計画だけ返す
    #[serde(default)]
    pub dry_run: bool,
}

/// 適用の結果。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct ApplyOutcome {
    #[serde(flatten)]
    pub plan: PlanSummary,
    /// 書き込んだか（dry_run なら false）
    pub written: bool,
}

/// 反映した編集を履歴に控える（取り消し・やり直しの元。ロックを持って呼ぶ）。
/// 履歴の保存に失敗しても編集は有効なまま進める。控えが古くなっても、取り消しは revision の確認で断られるので安全。
fn record(root: &Path, ops: &[Op], planned: &crate::plan::Plan) {
    let mut history = Journal::load(root);
    history.record(journal::Entry {
        cli: ops.iter().flat_map(to_cli).collect(),
        at: journal::now(),
        before: planned.base_revision.clone(),
        after: planned.revision.clone(),
        changes: planned.changes.clone(),
    });
    let _ = history.save(root);
}

/// Op の列をプロジェクトに適用する。
pub fn apply(root: &Path, ops: &[Op], options: &ApplyOptions) -> Result<ApplyOutcome, OpError> {
    let _lock = ProjectLock::acquire(root, LOCK_WAIT)?;
    let snapshot = load(root)?;
    if let Some(base) = &options.base_revision
        && *base != snapshot.design.revision
    {
        return Err(OpError::Conflict {
            expected: base.clone(),
            actual: snapshot.design.revision.clone(),
        });
    }
    let planned = plan(&snapshot, ops)?;
    let written = !options.dry_run && !planned.changes.is_empty();
    if written {
        commit(root, &planned.changes)?;
        record(root, ops, &planned);
    }
    Ok(ApplyOutcome {
        plan: PlanSummary::from(&planned),
        written,
    })
}
