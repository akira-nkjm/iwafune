//! どの API でも使う共通の型（量の表示・診断・エラーの本文）。

use fune_model::{Diagnostic, Severity};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 量の表示。`value`＋`unit` は表示単位での値、`si` は SI 値、`display` は整形済みの文字列。
/// Web と CLI は `display` をそのまま出す（単位の換算をしない）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct QuantityView {
    /// 表示単位での数値
    pub value: f64,
    /// 表示単位（例: `"g"`、`"mN*m*s"`）。無次元は空文字列
    pub unit: String,
    /// SI 値
    pub si: f64,
    /// 次元の名前（例: `"Mass"`）。名前の無い次元は指数の表記（例: `"L^2 M T^-3"`）
    pub dimension: String,
    /// 表示文字列（例: `"620 g"`）
    pub display: String,
}

/// 検証の結果 1 件（`fune_model::Diagnostic` に該当行の本文を添えたもの）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct DiagnosticView {
    /// 部品に関する診断なら部品 ID（設定・カタログは null）。
    pub part: Option<String>,
    #[serde(flatten)]
    #[ts(flatten)]
    pub diagnostic: Diagnostic,
    /// 該当行の前後の本文（CLI と GUI が該当箇所を示すのに使う）
    pub excerpt: Option<SourceExcerpt>,
}

/// ファイルの一部（診断で該当箇所を示すため）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct SourceExcerpt {
    /// `text` の先頭行の行番号（1 始まり）
    pub first_line: u32,
    pub text: String,
}

/// 診断の件数。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DiagnosticCounts {
    pub errors: u32,
    pub warnings: u32,
    pub infos: u32,
}

impl DiagnosticCounts {
    /// 診断の列から数える。
    pub fn of(diagnostics: &[DiagnosticView]) -> Self {
        let mut c = Self::default();
        for d in diagnostics {
            match d.diagnostic.severity {
                Severity::Error => c.errors += 1,
                Severity::Warning => c.warnings += 1,
                Severity::Info => c.infos += 1,
            }
        }
        c
    }
}

/// エラーの本文 `{ error: { code, message }, diagnostics }`。HTTP の本文と CLI の `--json` で共通。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ErrorEnvelope {
    pub error: ErrorInfo,
    pub diagnostics: Vec<DiagnosticView>,
}

/// エラーのコードと文。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ErrorInfo {
    pub code: String,
    pub message: String,
}

/// 検証の結果（`GET /diagnostics`、`fune check`）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CheckReport {
    pub revision: String,
    pub counts: DiagnosticCounts,
    pub diagnostics: Vec<DiagnosticView>,
}
