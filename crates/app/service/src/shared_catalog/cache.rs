//! commit の中身をキャッシュ（`.fune/catalogs/<名前>/<commit>/`）に置く。読む側（fune-store）は、このディレクトリが
//! あれば中身が揃っているとみなすので、一時ディレクトリに書いてから名前を変える。

use super::git::{Mirror, io_error};
use crate::error::{Result, ServiceError};
use fune_store::layout::{SHARED_CATALOG_DIRS, is_cache_key, shared_catalog_dir};
use std::path::{Path, PathBuf};

/// commit の中身がキャッシュにあるか。
pub(super) fn exists(root: &Path, name: &str, commit: &str) -> bool {
    is_cache_key(name, commit) && root.join(shared_catalog_dir(name, commit)).is_dir()
}

/// commit の `catalog/`・`sources/` の直下の `.toml` をキャッシュに置く（既にあれば何もしない）。
pub(super) fn extract(root: &Path, name: &str, mirror: &Mirror, commit: &str) -> Result<()> {
    if !is_cache_key(name, commit) {
        return Err(ServiceError::Data(format!(
            "commit `{commit}` をキャッシュの名前にできません"
        )));
    }
    if exists(root, name, commit) {
        return Ok(());
    }
    let dir = root.join(shared_catalog_dir(name, commit));
    let tmp = PathBuf::from(format!("{}.tmp-{}", dir.display(), std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let result = write_all(&tmp, mirror, commit)
        .and_then(|()| std::fs::rename(&tmp, &dir).map_err(|e| io_error(&dir, &e)));
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&tmp);
    }
    result
}

fn write_all(tmp: &Path, mirror: &Mirror, commit: &str) -> Result<()> {
    std::fs::create_dir_all(tmp).map_err(|e| io_error(tmp, &e))?;
    for sub in SHARED_CATALOG_DIRS {
        for (path, bytes) in mirror.files_in(commit, sub)? {
            let Some(file) = path.strip_prefix(&format!("{sub}/")) else {
                continue;
            };
            if !file.ends_with(".toml") || file.contains('/') || file.starts_with('.') {
                continue;
            }
            let dir = tmp.join(sub);
            std::fs::create_dir_all(&dir).map_err(|e| io_error(&dir, &e))?;
            let target = dir.join(file);
            std::fs::write(&target, bytes).map_err(|e| io_error(&target, &e))?;
        }
    }
    Ok(())
}
