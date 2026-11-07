//! 変更計画を store で書き込む（ロック → 読み込み → revision の確認 → 計画 → 反映）。

use crate::error::OpError;
use crate::op::Op;
use crate::plan::{PlanSummary, plan};
use fune_model::design::Revision;
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
    }
    Ok(ApplyOutcome {
        plan: PlanSummary::from(&planned),
        written,
    })
}
