//! # fune-store — 設計データのディレクトリ ⇄ Design（ファイル I/O はここだけ）
//!
//! | モジュール | 責務 |
//! |---|---|
//! | `layout` | ディレクトリの規約（どの種類のエンティティがどこに置かれるか。設定のパス → ファイルの唯一の対応。ファイル → 読み込みの診断の対象） |
//! | `project` | プロジェクトのルート（fune.toml）の探索と初期化、git のブランチ名 |
//! | `read` | ファイル群を読んで Design を作る（位置情報付き） |
//! | `codec` | 属性の値のファイル上の形 ⇄ `Value`（読みと書きの両方が使う） |
//! | `write` | 変更をファイルに反映する（toml_edit で最小差分）。一時ファイル → rename で原子的に |
//! | `fileset` / `profile` | 読み書きが共有するファイル集合・TOML の解析と整形部品 |
//! | `format` | 正規形の整形（TOML 1.0 の範囲・キーの順序） |
//! | `revision` | 追跡ファイルのハッシュ（楽観ロック用） |
//! | `lock` | プロセス間の書き込みロック |
//!
//! 検証はしない（`fune_model::check`）。読み込みで見つかった構文・形の誤りは `Design::load_diagnostics` に残す。

pub mod codec;
pub mod error;
pub mod format;
pub mod layout;
pub mod lock;
pub mod project;
pub mod read;
pub mod revision;
pub mod write;

pub use error::StoreError;
pub use read::{FileSet, Snapshot, load};
pub use write::{Edits, FileChange, apply_changes, commit};

pub mod fileset;
pub mod profile;
