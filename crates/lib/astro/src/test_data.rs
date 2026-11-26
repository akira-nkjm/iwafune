//! 試験用: 外部データ（キャッシュに置いた配布ファイル）の探索。ファイルが無ければ試験は読み飛ばす（読み飛ばしたと表示する）。
//!
//! 置き場所は `fune_service::data::cache_dir` と同じ規則（`$IWAFUNE_DATA_DIR`、無ければ `$XDG_CACHE_HOME/iwafune/data`、
//! 無ければ `~/.cache/iwafune/data`）。sat-astro は fune-service に依存できないので、試験の中だけで同じ規則を書く。

use std::path::PathBuf;

fn cache_dir() -> Option<PathBuf> {
    let var = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty());
    if let Some(dir) = var("IWAFUNE_DATA_DIR") {
        return Some(PathBuf::from(dir));
    }
    if let Some(xdg) = var("XDG_CACHE_HOME") {
        return Some(PathBuf::from(xdg).join("iwafune/data"));
    }
    var("HOME").map(|h| PathBuf::from(h).join(".cache/iwafune/data"))
}

/// キャッシュにあるファイル `name` の中身。無ければ読み飛ばしの表示を出して `None`。
pub(crate) fn cached_bytes(name: &str) -> Option<Vec<u8>> {
    let path = cache_dir()?.join(name);
    match std::fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(_) => {
            eprintln!(
                "SKIPPED: {} is not in the data cache (set IWAFUNE_DATA_DIR; see docs/trapped-radiation.md)",
                path.display()
            );
            None
        }
    }
}
