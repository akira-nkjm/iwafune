//! # fune-report — 設計書に入れる表と図の書き出し
//!
//! 設計データにも解析にも依存しない。サービス（`fune_service::report`）が解析の結果から表・図・値のデータ
//! （[`model`]）を組み立て、ここが `data.json`・SVG・Typst のモジュールのファイルにする。ファイルの置き方は [`files`]。
//!
//! | モジュール | 責務 |
//! |---|---|
//! | [`model`] | 項目・表・図・値のデータモデルと `data.json` の形 |
//! | [`labels`] | 見出し・注記の日本語と英語の対（書き出す文言はここだけ） |
//! | [`chart`] | 折れ線グラフの SVG（段・影・許容範囲の破線・凡例） |
//! | `spec`・`svg`・`legend`・`ticks` | グラフの入力の型・SVG の部品・凡例・軸の目盛り |
//! | [`palette`] | 図の色と線の型（図の色の決め場所はここだけ） |
//! | [`typst`] | Typst のモジュール（`iwafune.typ`）と、すべてを並べた例（`example.typ`） |
//! | [`files`] | 書き出すファイルの一覧（`data.json`・`iwafune.typ`・`example.typ`・`figs/*.svg`） |

pub mod chart;
pub mod files;
pub mod labels;
mod legend;
pub mod model;
pub mod palette;
mod spec;
mod svg;
mod ticks;
pub mod typst;

pub use files::{ReportFile, build_files};
pub use model::{Item as ReportItem, Lang as ReportLang, ReportData};

/// 書き出しのエラー。
#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("Cannot write data.json: {0}")]
    Json(#[from] serde_json::Error),
}
