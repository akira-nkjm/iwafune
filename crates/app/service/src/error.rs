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
    #[error(
        "プロジェクトが見つかりません: {0}（fune.toml のあるディレクトリを -C で指定してください）"
    )]
    ProjectNotFound(String),
    #[error("{what}が見つかりません: {id}")]
    NotFound { what: &'static str, id: String },
    #[error("入力が不正です: {0}")]
    InvalidInput(String),
    #[error("{what} は既にあります")]
    AlreadyExists { what: String },
    #[error("単位を解釈できません: {input}（{reason}）")]
    Unit { input: String, reason: String },
    #[error("検証エラーがあるため書き込みませんでした（{} 件）", .diagnostics.len())]
    Validation { diagnostics: Vec<DiagnosticView> },
    #[error(
        "revision が一致しません（指定 {expected}、現在 {actual}）。読み直してからやり直してください"
    )]
    Conflict { expected: String, actual: String },
    #[error("解析を実行できません: {0}")]
    Analysis(String),
    #[error("設計データを読み書きできません: {0}")]
    Store(String),
    #[error("ほかのプロセスが書き込み中です（{0}）。しばらくしてからやり直してください")]
    Locked(String),
    #[error("外部データ: {0}")]
    Data(String),
    #[error("内部エラー: {0}")]
    Internal(String),
}

impl ServiceError {
    /// 分類。
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::ProjectNotFound(_) | Self::NotFound { .. } => ErrorKind::NotFound,
            Self::InvalidInput(_) | Self::Unit { .. } => ErrorKind::InvalidInput,
            Self::Validation { .. } => ErrorKind::Validation,
            Self::Conflict { .. } | Self::Locked(_) | Self::AlreadyExists { .. } => {
                ErrorKind::Conflict
            }
            Self::Analysis(_) | Self::Store(_) | Self::Data(_) | Self::Internal(_) => {
                ErrorKind::Internal
            }
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
            Self::Locked(_) => "locked",
            Self::Analysis(_) => "analysis_failed",
            Self::Store(_) => "store_error",
            Self::Data(_) => "data_error",
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
