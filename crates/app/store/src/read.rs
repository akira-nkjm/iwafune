//! 読み込み: ディレクトリ → [`FileSet`] → [`Design`]（位置の索引付き）。
//!
//! | モジュール | 中身 |
//! |---|---|
//! | `files` | 追跡ファイルの集合と、ディレクトリの走査 |
//! | `spans` | キー → 行・列の索引 |
//! | `profile` | TOML プロファイル（§10.2）に沿わない書き方の検出（1.1 の構文・整数の物理量） |
//! | `decode` | ファイル群 → Design（設定・定義・モード・ネットなど） |
//! | `tree` | 部品・カタログ品のファイルと、ディレクトリの規則による製品ツリー |
//! | `shared` | 共有カタログ（`catalog.lock` の commit のキャッシュ）の走査と、出所付きの品・出典としての読み込み |

mod decode;
mod files;
mod profile;
mod shared;
mod spans;
mod tree;

pub use decode::decode;
pub use files::{FileSet, scan};
pub use shared::parse_lock;
pub use spans::LineIndex;

use crate::error::StoreError;
use crate::revision;
use fune_model::Design;
use std::path::{Path, PathBuf};

/// 読み込んだプロジェクト（ルート・ファイルの本文・Design）。編集の計画はこれを元に作る。
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub root: PathBuf,
    pub files: FileSet,
    pub design: Design,
}

/// ファイルの集合から Design を作り、revision を入れる。
pub fn design_of(files: &FileSet) -> Design {
    let mut design = decode(files);
    design.revision = revision::of(files);
    design
}

/// プロジェクトのルートを読む。
pub fn load(root: &Path) -> Result<Snapshot, StoreError> {
    let files = scan(root)?;
    let design = design_of(&files);
    Ok(Snapshot {
        root: root.to_path_buf(),
        files,
        design,
    })
}
