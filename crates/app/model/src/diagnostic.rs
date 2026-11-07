//! 診断の型（severity・code・path・file・line）。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 診断の重さ。error は書き込みを拒む（新たに増える場合）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// 検証の結果 1 件。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
pub struct Diagnostic {
    pub severity: Severity,
    /// 機械向けのコード（例: `"unit.dimension"`）
    pub code: String,
    pub message: String,
    /// 対象のパラメーターのパス（例: `"RW_X.mass"`・`"modes.NOMINAL.TRX"`）
    pub path: Option<String>,
    /// プロジェクトのルートからの相対パス
    pub file: Option<String>,
    /// 1 始まり
    pub line: Option<u32>,
    pub column: Option<u32>,
}

impl Diagnostic {
    /// 診断を作る（位置は後から [`Diagnostic::at`] で付ける）。
    pub fn new(severity: Severity, code: &str, message: impl Into<String>) -> Self {
        Self {
            severity,
            code: code.to_string(),
            message: message.into(),
            path: None,
            file: None,
            line: None,
            column: None,
        }
    }

    /// error の診断。
    pub fn error(code: &str, message: impl Into<String>) -> Self {
        Self::new(Severity::Error, code, message)
    }

    /// warning の診断。
    pub fn warning(code: &str, message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, code, message)
    }

    /// info の診断。
    pub fn info(code: &str, message: impl Into<String>) -> Self {
        Self::new(Severity::Info, code, message)
    }

    /// パラメーターのパスを付ける。
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// ファイルの位置を付ける。
    pub fn at(mut self, loc: Option<Loc>) -> Self {
        if let Some(loc) = loc {
            self.file = Some(loc.file);
            self.line = loc.line;
            self.column = loc.column;
        }
        self
    }

    /// error か。
    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

/// 診断の列に error があるか。
pub fn has_errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(Diagnostic::is_error)
}

/// ファイル内の位置。`file` はプロジェクトのルートからの相対パス（`/` 区切り）、行と列は 1 始まり。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(rename = "SourceLoc")]
pub struct Loc {
    pub file: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
}
