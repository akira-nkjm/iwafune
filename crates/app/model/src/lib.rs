//! # fune-model — 設計データの型・解決・検証（I/O しない）
//!
//! | モジュール | 責務 |
//! |---|---|
//! | `id` | 型付き ID と ID の規則（一意性は大文字小文字を無視。`same_id`・`clash`・`duplicates`） |
//! | `chain` | 連鎖のたどり方（取付・製品ツリーの親・種別の継承。循環を見つけて止める） |
//! | `enum_text` | 列挙の値の綴り（serde・`as_str`・`Display`・`FromStr`・値の一覧）を 1 回だけ書く仕組み（`enum_text!`） |
//! | `field_names` | 部品・カタログ品・ネットのフィールド名の正本（ファイルのキー・パス記法・CLI の名前と正規の順） |
//! | `value` | 書かれたままの量（数値＋単位文字列）・値の型・出典。`value::units` は単位の唯一の窓口（中身は sat-units）、`value::text` は人の入力の書式（読む向きと書く向き）、`value::json` は JSON の中の量・ベクトルの見分け |
//! | `design` | Design 層の型（Project・Part・Group・Mode・Net・Harness・Budget・設定など。docs/model.md §4）と、設定をパス記法で読む関数、解析の系の一覧（`AnalysisSystem`）、参照の索引（`refs`）、設定の量の次元の表（`setting_dims`） |
//! | `lookup` | Design の組み込み fallback 検索（型定義から分離） |
//! | `builtin` | 組み込みの属性定義・種別定義（解析が前提にする標準の語彙）と、状態・系・列挙の表示名（日英） |
//! | `part_system` | 部品・フォルダの系（`Design::system_of`。置き場所 → 役割の系。`design` と `builtin` の両方を見る葉） |
//! | `path` | パラメーターのパス記法（`RW_X.mass`、`RW_X.power.on`） |
//! | `resolve` | Design → Resolved（カタログ・上書きの重ね合わせ、SI 換算、出所） |
//! | `check` | 検証（severity 付きの診断） |
//! | `diagnostic` | 診断の型 |
//! | `target` | 診断と不足入力の対象（部品・属性・設定・モードなどを構造で持つ。文字列のパスは `Display` だけ） |
//!
//! 設計データのファイル I/O はしない（読み書きは fune-store）。TOML を読むのは、埋め込みの組み込み定義（`builtin`）だけ。

pub mod builtin;
pub mod chain;
pub mod check;
pub mod design;
pub mod diagnostic;
pub mod enum_text;
pub mod field_names;
pub mod id;
pub mod part_system;
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
