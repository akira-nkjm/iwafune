//! 編集の取り消し・やり直し（`fune undo`・`fune redo`・`POST /undo`・`POST /redo`）。
//! 控えの保存は `fune_store::journal`、書き戻す適用は `fune_ops::history`。ここは結果を表示用の型にするだけ。
//!
//! 履歴は編集（Op の適用）ごとに積み、上限は 100 件・本文の合計 32 MiB（古いものから捨てる）。
//! `fune fmt`・`fune catalog fetch|update`（Op ではない書き込み）は履歴に積まず、取り消せない。
//! これらで設計ファイルが変わると、それ以前の編集の取り消しは revision の確認で断られる。

use crate::dto::{HistoryAction, HistoryEntryView, HistoryStepResult, HistoryView};
use crate::edit::changed_files;
use crate::error::Result;
use crate::workspace::{Workspace, op_error};
use fune_ops::{Direction, StepOutcome};
use fune_store::journal::{Journal, MAX_ENTRIES};

/// 編集の履歴（`fune history`・`GET /history`）。
pub fn view(ws: &Workspace) -> Result<HistoryView> {
    let journal = Journal::load(ws.root());
    Ok(HistoryView {
        can_undo: journal.undoable().is_some(),
        can_redo: journal.redoable().is_some(),
        limit: u32::try_from(MAX_ENTRIES).unwrap_or(u32::MAX),
        entries: journal
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| HistoryEntryView {
                cli: e.cli.clone(),
                at: e.at,
                files: e.changes.iter().map(|c| c.path.clone()).collect(),
                undone: i >= journal.head,
            })
            .collect(),
    })
}

/// 直前の編集を取り消す。
pub fn undo(ws: &Workspace) -> Result<HistoryStepResult> {
    let out = fune_ops::undo(ws.root()).map_err(|e| op_error(ws, e))?;
    Ok(result_of(out))
}

/// 取り消した編集をやり直す。
pub fn redo(ws: &Workspace) -> Result<HistoryStepResult> {
    let out = fune_ops::redo(ws.root()).map_err(|e| op_error(ws, e))?;
    Ok(result_of(out))
}

fn result_of(out: StepOutcome) -> HistoryStepResult {
    HistoryStepResult {
        action: match out.direction {
            Direction::Undo => HistoryAction::Undo,
            Direction::Redo => HistoryAction::Redo,
        },
        revision: out.revision.0,
        previous_revision: out.previous_revision.0,
        changed: changed_files(&out.changes),
        edit_cli: out.cli,
    }
}
