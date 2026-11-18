//! 編集の取り消し・やり直し。履歴（[`fune_store::journal`]）に控えた前後の中身を書き戻す適用で、ロック → 読み込み →
//! revision の確認 → 反映（[`fune_store::commit`]）の順は `apply` と同じ。
//!
//! 取り消しは「編集の後の revision」、やり直しは「編集の前の revision」と今の revision が一致するときだけ通す。
//! 違えば履歴を通さずに設計ファイルが書き換わっているので、何も書かずに [`OpError::HistoryMismatch`] で断る。
//! 戻す・やり直すこと自体は履歴に新しい編集として積まず、位置（`head`）だけを動かす。

use crate::apply::LOCK_WAIT;
use crate::error::OpError;
use fune_model::design::Revision;
use fune_store::journal::{Entry, Journal};
use fune_store::lock::ProjectLock;
use fune_store::{FileChange, commit, load};
use std::path::Path;

/// 取り消しかやり直しか。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Undo,
    Redo,
}

/// 取り消し・やり直しの結果。
#[derive(Debug, Clone, PartialEq)]
pub struct StepOutcome {
    pub direction: Direction,
    /// 取り消した（やり直した）編集の CLI のコマンド
    pub cli: Vec<String>,
    /// 実際に書き戻した変更（`before` が今の中身、`after` が戻した後の中身）
    pub changes: Vec<FileChange>,
    /// 書き戻す前の revision
    pub previous_revision: Revision,
    /// 書き戻した後の revision
    pub revision: Revision,
}

/// 直前の編集を取り消す。
pub fn undo(root: &Path) -> Result<StepOutcome, OpError> {
    step(root, Direction::Undo)
}

/// 取り消した編集をやり直す。
pub fn redo(root: &Path) -> Result<StepOutcome, OpError> {
    step(root, Direction::Redo)
}

fn step(root: &Path, direction: Direction) -> Result<StepOutcome, OpError> {
    let redo = direction == Direction::Redo;
    let _lock = ProjectLock::acquire(root, LOCK_WAIT)?;
    let mut history = Journal::load(root);
    let entry = match direction {
        Direction::Undo => history.undoable(),
        Direction::Redo => history.redoable(),
    }
    .cloned()
    .ok_or(OpError::NothingToStep { redo })?;
    let current = load(root)?.design.revision;
    let (expected, result) = match direction {
        Direction::Undo => (&entry.after, &entry.before),
        Direction::Redo => (&entry.before, &entry.after),
    };
    if *expected != current {
        return Err(OpError::HistoryMismatch {
            redo,
            expected: expected.clone(),
            actual: current,
        });
    }
    let changes = reversed(&entry, direction);
    commit(root, &changes)?;
    match direction {
        Direction::Undo => history.head -= 1,
        Direction::Redo => history.head += 1,
    }
    // 保存に失敗しても書き戻しは済んでいる。控えが古くなっても次の確認で断られるので安全。
    let _ = history.save(root);
    Ok(StepOutcome {
        direction,
        cli: entry.cli,
        changes,
        previous_revision: current,
        revision: result.clone(),
    })
}

/// 控えた変更を、書き戻す向きの変更にする（取り消しは前後を入れ替える）。
fn reversed(entry: &Entry, direction: Direction) -> Vec<FileChange> {
    entry
        .changes
        .iter()
        .map(|c| match direction {
            Direction::Undo => FileChange {
                path: c.path.clone(),
                before: c.after.clone(),
                after: c.before.clone(),
            },
            Direction::Redo => c.clone(),
        })
        .collect()
}
