//! 診断の型（severity・code・対象・file・line）。

use crate::enum_text::enum_text;
use crate::target::Target;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

enum_text! {
    /// 診断の重さ。error は書き込みを拒む（新たに増える場合）。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
    pub enum Severity: "診断の重さ" {
        Error = "error",
        Warning = "warning",
        Info = "info",
    }
}

/// 検証の結果 1 件。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
pub struct Diagnostic {
    pub severity: Severity,
    /// 機械向けのコード（例: `"unit.dimension"`）
    pub code: String,
    pub message: String,
    /// 何についての診断か（構造。表示用のパスは [`Diagnostic::path`]）
    pub target: Option<Target>,
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
            target: None,
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

    /// 対象を付ける。
    pub fn with_target(mut self, target: impl Into<Target>) -> Self {
        self.target = Some(target.into());
        self
    }

    /// 対象の表示用のパス（`Target` の `Display`。例: `"RW_X.mass"`・`"modes.NOMINAL.TRX"`）。
    pub fn path(&self) -> Option<String> {
        self.target.as_ref().map(Target::to_string)
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
