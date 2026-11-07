//! 追跡するファイルの集合（相対パス → 本文）。読み込みと、書き込み前の変更計画の両方がこれを使う。

use crate::error::StoreError;
pub use crate::fileset::FileSet;
use crate::layout::{IGNORED_DIRS, WORK_DIR};
use std::fs;
use std::path::Path;

/// 追跡するファイルの拡張子。
const TRACKED_EXTENSIONS: &[&str] = &["toml", "csv"];

/// プロジェクトのルートから追跡ファイルを読む（`.` で始まる名前・作業ディレクトリ・`results/` などは飛ばす）。
pub fn scan(root: &Path) -> Result<FileSet, StoreError> {
    let mut set = FileSet::new();
    walk(root, "", &mut set)?;
    Ok(set)
}

fn walk(dir: &Path, rel: &str, set: &mut FileSet) -> Result<(), StoreError> {
    let entries = fs::read_dir(dir).map_err(|e| StoreError::io(dir, e))?;
    let mut names: Vec<(String, bool)> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| StoreError::io(dir, e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_dir = entry
            .file_type()
            .map_err(|e| StoreError::io(&entry.path(), e))?
            .is_dir();
        names.push((name, is_dir));
    }
    names.sort();
    for (name, is_dir) in names {
        if name.starts_with('.') || name == WORK_DIR {
            continue;
        }
        let child_rel = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        let path = dir.join(&name);
        if is_dir {
            if rel.is_empty() && IGNORED_DIRS.contains(&name.as_str()) {
                continue;
            }
            walk(&path, &child_rel, set)?;
        } else if TRACKED_EXTENSIONS
            .iter()
            .any(|ext| name.ends_with(&format!(".{ext}")))
        {
            let text = fs::read_to_string(&path).map_err(|e| StoreError::io(&path, e))?;
            set.insert(child_rel, text);
        }
    }
    Ok(())
}
