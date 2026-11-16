//! commit の中身をキャッシュ（`.fune/catalogs/<名前>/<commit>/`）に置く。読む側（fune-store）は、このディレクトリが
//! あれば中身が揃っているとみなすので、一時ディレクトリに書いてから名前を変える。

use super::git::{Mirror, io_error};
use crate::error::{Result, ServiceError};
use fune_store::layout::{SHARED_CATALOG_DIRS, SHARED_ITEMS_DIR, is_cache_key, shared_catalog_dir};
use std::path::{Path, PathBuf};

/// commit の中身がキャッシュにあるか。
pub(super) fn exists(root: &Path, name: &str, commit: &str) -> bool {
    is_cache_key(name, commit) && root.join(shared_catalog_dir(name, commit)).is_dir()
}

/// commit の `catalog/`（ディレクトリの下も）・`sources/`（直下だけ）の `.toml` をキャッシュに置く（既にあれば何もしない）。
pub(super) fn extract(root: &Path, name: &str, mirror: &Mirror, commit: &str) -> Result<()> {
    if !is_cache_key(name, commit) {
        return Err(ServiceError::Data(format!(
            "Cannot use commit `{commit}` as a cache name"
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
            // 出典は平ら。品はディレクトリの下も（`.` で始まる名前は読まない）
            let nested_ok = *sub == SHARED_ITEMS_DIR;
            if !file.ends_with(".toml")
                || (file.contains('/') && !nested_ok)
                || file
                    .split('/')
                    .any(|seg| seg.is_empty() || seg.starts_with('.'))
            {
                continue;
            }
            let target = tmp.join(sub).join(file);
            let dir = target.parent().unwrap_or(tmp);
            std::fs::create_dir_all(dir).map_err(|e| io_error(dir, &e))?;
            std::fs::write(&target, bytes).map_err(|e| io_error(&target, &e))?;
        }
    }
    Ok(())
}
