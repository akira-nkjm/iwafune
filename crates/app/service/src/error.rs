//! サービスのエラー。HTTP の状態や CLI の終了コードは知らず、[`ErrorKind`] という分類だけを持つ。
//! server は分類 → HTTP の状態、cli は分類 → 終了コードに写す。

use crate::dto::{DiagnosticView, ErrorEnvelope, ErrorInfo};

/// エラーの分類（呼び出し側が状態や終了コードを決めるのに使う）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// 指定したもの（部品・解析・プロジェクト）が無い。
    NotFound,
    /// 入力が不正（引数・単位・Op の形）。
    InvalidInput,
    /// 検証エラーで書き込まなかった。
    Validation,
    /// 今の状態とぶつかる（revision が違う・ロックが取れない・同じ ID がもうある）。
    Conflict,
    /// それ以外（I/O など）。
    Internal,
}

/// fune-service のエラー。
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("Project not found: {0} (pass the directory containing fune.toml with -C)")]
    ProjectNotFound(String),
    #[error("{what} not found: {id}")]
    NotFound { what: &'static str, id: String },
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("{what} already exists")]
    AlreadyExists { what: String },
    #[error("Cannot parse unit: {input} ({reason})")]
    Unit { input: String, reason: String },
    #[error("Nothing was written because of validation errors ({})", .diagnostics.len())]
    Validation { diagnostics: Vec<DiagnosticView> },
    #[error("Revision mismatch (given {expected}, current {actual}). Reload and try again")]
    Conflict { expected: String, actual: String },
    #[error("Nothing to {action}")]
    NothingToStep { action: &'static str },
    #[error(
        "Cannot {action}: the design files changed outside the edit history since that edit \
         (expected revision {expected}, found {actual}). Edits made by hand, by git, or by `fune fmt` / `fune catalog fetch` end the history at that point"
    )]
    HistoryConflict {
        action: &'static str,
        expected: String,
        actual: String,
    },
    #[error("Cannot run the analysis: {0}")]
    Analysis(String),
    #[error("Cannot read or write the design data: {0}")]
    Store(String),
    #[error("Another process is writing ({0}). Try again in a moment")]
    Locked(String),
    #[error("External data: {0}")]
    Data(String),
    #[error("Cannot update fune: {0}")]
    SelfUpdate(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl ServiceError {
    /// 分類。
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::ProjectNotFound(_) | Self::NotFound { .. } | Self::NothingToStep { .. } => {
                ErrorKind::NotFound
            }
            Self::InvalidInput(_) | Self::Unit { .. } => ErrorKind::InvalidInput,
            Self::Validation { .. } => ErrorKind::Validation,
            Self::Conflict { .. }
            | Self::HistoryConflict { .. }
            | Self::Locked(_)
            | Self::AlreadyExists { .. } => ErrorKind::Conflict,
            Self::Analysis(_)
            | Self::Store(_)
            | Self::Data(_)
            | Self::SelfUpdate(_)
            | Self::Internal(_) => ErrorKind::Internal,
        }
    }

    /// 機械向けのコード（`{error:{code}}` に入る）。
    pub fn code(&self) -> &'static str {
        match self {
            Self::ProjectNotFound(_) => "project_not_found",
            Self::NotFound { .. } => "not_found",
            Self::InvalidInput(_) => "invalid_input",
            Self::AlreadyExists { .. } => "already_exists",
            Self::Unit { .. } => "unit_error",
            Self::Validation { .. } => "validation_failed",
            Self::Conflict { .. } => "revision_conflict",
            Self::NothingToStep { .. } => "history_empty",
            Self::HistoryConflict { .. } => "history_conflict",
            Self::Locked(_) => "locked",
            Self::Analysis(_) => "analysis_failed",
            Self::Store(_) => "store_error",
            Self::Data(_) => "data_error",
            Self::SelfUpdate(_) => "self_update_failed",
            Self::Internal(_) => "internal",
        }
    }

    /// エラーに付いている診断（検証エラーのとき）。
    pub fn diagnostics(&self) -> &[DiagnosticView] {
        match self {
            Self::Validation { diagnostics } => diagnostics,
            _ => &[],
        }
    }

    /// 機械向けの表現（HTTP の本文と CLI の `--json` で共通）。
    pub fn envelope(&self) -> ErrorEnvelope {
        ErrorEnvelope {
            error: ErrorInfo {
                code: self.code().to_owned(),
                message: self.to_string(),
            },
            diagnostics: self.diagnostics().to_vec(),
        }
    }

    pub(crate) fn not_found(what: &'static str, id: impl Into<String>) -> Self {
        Self::NotFound {
            what,
            id: id.into(),
        }
    }
}

/// fune-service の Result。
pub type Result<T, E = ServiceError> = std::result::Result<T, E>;
