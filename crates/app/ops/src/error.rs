//! Op の誤り（HTTP の状態と CLI の終了コードに写せる分類）。

use fune_model::Diagnostic;
use fune_model::design::Revision;
use fune_store::StoreError;

/// Op を計画・適用できなかった理由。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum OpError {
    /// 読んだ後にほかで書き換えられた（`base_revision` が今の revision と違う）
    #[error(
        "競合: 基準の revision {expected} が今の revision {actual} と違います（読み直してください）"
    )]
    Conflict {
        expected: Revision,
        actual: Revision,
    },
    #[error("{what} がありません")]
    NotFound { what: String },
    #[error("{what} は既にあります")]
    AlreadyExists { what: String },
    /// Op の中身が不正（ID の規則・必須の項目の欠けなど）
    #[error("{message}")]
    Invalid { message: String },
    /// 適用後の設計に新しい error が出る
    #[error("適用すると {} 件の error が出るので書き込みません", diagnostics.len())]
    Rejected { diagnostics: Vec<Diagnostic> },
    #[error(transparent)]
    Store(#[from] StoreError),
}

impl OpError {
    /// CLI の終了コード（1 検証エラー / 2 引数エラー / 3 競合）。
    pub fn exit_code(&self) -> i32 {
        match self {
            OpError::Rejected { .. } => 1,
            OpError::Conflict { .. } => 3,
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
