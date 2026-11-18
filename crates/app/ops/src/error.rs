//! Op の誤り（HTTP の状態と CLI の終了コードに写せる分類）。

use fune_model::Diagnostic;
use fune_model::design::Revision;
use fune_store::StoreError;

/// Op を計画・適用できなかった理由。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum OpError {
    /// 読んだ後にほかで書き換えられた（`base_revision` が今の revision と違う）
    #[error(
        "Conflict: base revision {expected} differs from current revision {actual} (reload and retry)"
    )]
    Conflict {
        expected: Revision,
        actual: Revision,
    },
    #[error("{what} not found")]
    NotFound { what: String },
    #[error("{what} already exists")]
    AlreadyExists { what: String },
    /// Op の中身が不正（ID の規則・必須の項目の欠けなど）
    #[error("{message}")]
    Invalid { message: String },
    /// 適用後の設計に新しい error が出る
    #[error("Not written: applying this would produce {} error(s)", diagnostics.len())]
    Rejected { diagnostics: Vec<Diagnostic> },
    /// 取り消し・やり直しの対象が無い
    #[error("Nothing to {}", if *redo { "redo" } else { "undo" })]
    NothingToStep { redo: bool },
    /// 履歴の編集のあとに、履歴を通さずに設計ファイルが書き換わっている（手で編集・git・`fune fmt`・共有カタログの取得など）
    #[error(
        "Cannot {}: the design files changed outside the edit history since that edit \
         (expected revision {expected}, found {actual}). Edits made by hand, by git, or by `fune fmt` / `fune catalog fetch` end the history at that point",
        if *redo { "redo" } else { "undo" }
    )]
    HistoryMismatch {
        redo: bool,
        expected: Revision,
        actual: Revision,
    },
    #[error(transparent)]
    Store(#[from] StoreError),
}

impl OpError {
    /// CLI の終了コード（1 検証エラー / 2 引数エラー / 3 競合）。
    pub fn exit_code(&self) -> i32 {
        match self {
            OpError::Rejected { .. } => 1,
            OpError::Conflict { .. } | OpError::HistoryMismatch { .. } => 3,
            OpError::NothingToStep { .. } => 2,
            OpError::Store(StoreError::Locked { .. }) => 3,
            OpError::Store(_) => 1,
            OpError::NotFound { .. } | OpError::AlreadyExists { .. } | OpError::Invalid { .. } => 2,
        }
    }

    pub(crate) fn invalid(message: impl Into<String>) -> Self {
        OpError::Invalid {
            message: message.into(),
        }
    }

    pub(crate) fn not_found(what: impl Into<String>) -> Self {
        OpError::NotFound { what: what.into() }
    }

    pub(crate) fn exists(what: impl Into<String>) -> Self {
        OpError::AlreadyExists { what: what.into() }
    }
}
