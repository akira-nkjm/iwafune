//! 取り消し・やり直しの結果の型（`POST /undo`・`POST /redo`、`fune undo`・`fune redo`）。

use super::ChangedFile;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 取り消しか、やり直しか。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum HistoryAction {
    Undo,
    Redo,
}

/// 取り消し・やり直しの結果。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct HistoryStepResult {
    pub action: HistoryAction,
    /// 書き戻した後の revision
    pub revision: String,
    /// 書き戻す前の revision
    pub previous_revision: String,
    /// 書き戻したファイル（差分は今の中身 → 戻した後の中身）
    pub changed: Vec<ChangedFile>,
    /// 取り消した（やり直した）編集と同じ操作をする CLI のコマンド（編集の見出しに使う）
    pub edit_cli: Vec<String>,
}

/// 編集の履歴（`GET /history`・`fune history`）。古い順。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct HistoryView {
    pub entries: Vec<HistoryEntryView>,
    /// 取り消せる編集があるか（revision の確認は取り消すときに行う）
    pub can_undo: bool,
    /// やり直せる編集があるか
    pub can_redo: bool,
    /// 控えておく編集の数の上限
    pub limit: u32,
}

/// 履歴の編集 1 件。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct HistoryEntryView {
    /// 同じ操作をする CLI のコマンド
    pub cli: Vec<String>,
    /// 適用した時刻（UNIX 秒）
    pub at: u64,
    /// 変えたファイル
    pub files: Vec<String>,
    /// 取り消し済み（やり直せる）か
    pub undone: bool,
}
