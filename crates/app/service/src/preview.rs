//! 保存していない編集（Op の列）を当てた設計。下書きのまま解析やバジェットを見るため、書き込まずに
//! fune-ops で計画し、計画したあとの設計とファイルからスナップショットを作る。

use crate::error::Result;
use crate::workspace::{Snapshot, Workspace, op_error, store_error};
use fune_ops::Op;

/// `ops` を当てたスナップショット（書き込まない）。`ops` が空なら [`Workspace::snapshot`] と同じ。
/// 編集が通らない（検証で新しいエラーが出るなど）ときは、保存したときと同じエラーを返す。
pub fn snapshot_with(ws: &Workspace, ops: &[Op]) -> Result<Snapshot> {
    if ops.is_empty() {
        return ws.snapshot();
    }
    let store = fune_store::load(ws.root()).map_err(store_error)?;
    apply(ws, &store, ops)
}

/// 読み込み済みの `base` に `ops` を当てたスナップショット（ディスクを読み直さない。[`crate::session::Session::draft`] が使う）。
pub(crate) fn draft_of(ws: &Workspace, base: &Snapshot, ops: &[Op]) -> Result<Snapshot> {
    let store = fune_store::Snapshot {
        root: ws.root().to_path_buf(),
        files: base.files.clone(),
        design: base.design.clone(),
    };
    apply(ws, &store, ops)
}

/// 計画して、計画したあとの設計とファイルからスナップショットを作る（`snapshot_with` と `draft_of` の共通部分）。
fn apply(ws: &Workspace, store: &fune_store::Snapshot, ops: &[Op]) -> Result<Snapshot> {
    let plan = fune_ops::plan(store, ops).map_err(|e| op_error(ws, e))?;
    let mut files = store.files.clone();
    fune_store::apply_changes(&mut files, &plan.changes);
    Ok(Snapshot::of(plan.design, files))
}
