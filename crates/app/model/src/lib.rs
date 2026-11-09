//! # fune-model — 設計データの型・解決・検証（I/O しない）
//!
//! | モジュール | 責務 |
//! |---|---|
//! | `id` | 型付き ID と ID の規則 |
//! | `value` | 書かれたままの量（数値＋単位文字列）・値の型・出典。`value::units` は単位の唯一の窓口（中身は sat-units）、`value::text` は人の入力の解釈、`value::json` は JSON の中の量・ベクトルの見分け |
//! | `design` | Design 層の型（Project・Part・Group・Mode・Net・Harness・Budget・設定など。docs/model.md §4）と、設定をパス記法で読む関数 |
//! | `lookup` | Design の組み込み fallback 検索（型定義から分離） |
//! | `builtin` | 組み込みの属性定義・種別定義（解析が前提にする標準の語彙）と、状態・系・列挙の表示名（日英） |
//! | `path` | パラメーターのパス記法（`RW_X.mass`、`RW_X.power.on`） |
//! | `resolve` | Design → Resolved（カタログ・上書きの重ね合わせ、SI 換算、出所） |
//! | `check` | 検証（severity 付きの診断） |
//! | `diagnostic` | 診断の型 |
//! | `target` | 診断と不足入力の対象（部品・属性・設定・モードなどを構造で持つ。文字列のパスは `Display` だけ） |
//!
//! 設計データのファイル I/O はしない（読み書きは fune-store）。TOML を読むのは、埋め込みの組み込み定義（`builtin`）だけ。

pub mod builtin;
pub mod check;
pub mod design;
pub mod diagnostic;
pub mod id;
pub mod path;
pub mod resolve;
pub mod target;
pub mod value;

pub use check::check;
pub use design::Design;
pub use diagnostic::{Diagnostic, Severity};
pub use id::Id;
pub use resolve::{Resolved, resolve};
pub use target::Target;

mod lookup;
