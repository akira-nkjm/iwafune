//! 複数ファイルにまたがる変更をまとめて反映する（tmp ＋ rename、失敗したら戻す）。
//!
//! 1. 準備: 書くファイルをすべて同じディレクトリの tmp（`.<名前>.tmp`）に書き出す。失敗しても正本は無傷。
//! 2. 退避: 上書き・削除するファイルを `.<名前>.bak` に複製する。
//! 3. 反映: tmp を rename で置き換え、削除対象を消す。途中で失敗したら反映済みの分を退避から戻し、
//!    新規作成した分は消してからエラーを返す。
//!
//! tmp と退避は `.` で始まるので、読み込み（[`crate::read::scan`]）には見えない。
//! 反映の最中にプロセスごと落ちた場合は退避が残るので、人が戻せる。

use super::FileChange;
use crate::error::StoreError;
use std::fs;
use std::path::{Path, PathBuf};

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.with_file_name(format!(".{name}.{suffix}"))
}

fn fail(message: String) -> StoreError {
    StoreError::Commit { message }
}

/// 変更をまとめて反映する。失敗したら変更前の状態へ戻してからエラーを返す。
pub fn commit(root: &Path, changes: &[FileChange]) -> Result<(), StoreError> {
    let targets: Vec<PathBuf> = changes.iter().map(|c| root.join(&c.path)).collect();
    let tmps = prepare(changes, &targets)?;
    let backups = match back_up(&targets) {
        Ok(b) => b,
        Err(e) => {
            cleanup(&tmps);
            return Err(e);
        }
    };
    for (applied, (change, target)) in changes.iter().zip(&targets).enumerate() {
        let result = match &change.after {
            Some(_) => fs::rename(sibling(target, "tmp"), target),
            None if target.exists() => fs::remove_file(target),
            None => Ok(()),
        };
        if let Err(error) = result {
            let restored = roll_back(&targets[..applied], &backups);
            cleanup(&tmps);
            if restored {
                cleanup(&backups);
            }
            return Err(fail(format!(
                "Could not apply {}: {error} ({})",
                target.display(),
                if restored {
                    "rolled back to the previous state"
                } else {
                    "could not roll back; restore from the .*.bak files"
                }
            )));
        }
    }
    cleanup(&backups);
    remove_empty_dirs(root, changes);
    Ok(())
}

/// 書くファイルを tmp に書き出す。
fn prepare(changes: &[FileChange], targets: &[PathBuf]) -> Result<Vec<PathBuf>, StoreError> {
    let mut tmps = Vec::new();
    for (change, target) in changes.iter().zip(targets) {
        let Some(contents) = &change.after else {
            continue;
        };
        let tmp = sibling(target, "tmp");
        let written = target
            .parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|()| fs::write(&tmp, contents));
        if let Err(error) = written {
            cleanup(&tmps);
            let _ = fs::remove_file(&tmp);
            return Err(fail(format!("Cannot write {}: {error}", tmp.display())));
        }
        tmps.push(tmp);
    }
    Ok(tmps)
}

/// 既存のファイルを退避する。退避したファイルの (元, 退避先)。
fn back_up(targets: &[PathBuf]) -> Result<Vec<PathBuf>, StoreError> {
    let mut backups = Vec::new();
    for target in targets {
        if !target.exists() {
            continue;
        }
        let backup = sibling(target, "bak");
        if let Err(error) = fs::copy(target, &backup) {
            cleanup(&backups);
            return Err(fail(format!(
                "Cannot back up {}: {error}",
                target.display()
            )));
        }
        backups.push(backup);
    }
    Ok(backups)
}

/// 反映済みの分を戻す。退避があれば戻し、無ければ（新規作成なら）消す。
fn roll_back(applied: &[PathBuf], backups: &[PathBuf]) -> bool {
    let mut ok = true;
    for target in applied {
        let backup = sibling(target, "bak");
        let result = if backups.contains(&backup) {
            fs::copy(&backup, target).map(|_| ())
        } else if target.exists() {
            fs::remove_file(target)
        } else {
            Ok(())
        };
        ok &= result.is_ok();
    }
    ok
}

fn cleanup(paths: &[PathBuf]) {
    for path in paths {
        // 掃除の失敗は致命ではない（`.` 始まりなので読み込みには見えない）。
        let _ = fs::remove_file(path);
    }
}

/// 削除・移動で空になった `parts/` の下のディレクトリを消す（git は空のディレクトリを持たないため）。
fn remove_empty_dirs(root: &Path, changes: &[FileChange]) {
    for change in changes.iter().filter(|c| c.after.is_none()) {
        let mut dir = root.join(&change.path);
        while let Some(parent) = dir.parent().map(Path::to_path_buf) {
            if parent == root || fs::remove_dir(&parent).is_err() {
                break;
            }
            dir = parent;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(path: &str, before: Option<&str>, after: Option<&str>) -> FileChange {
        FileChange {
            path: path.into(),
            before: before.map(Into::into),
            after: after.map(Into::into),
        }
    }

    #[test]
    fn writes_and_removes_files_atomically() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("parts/A")).unwrap();
        fs::write(dir.path().join("parts/A/X.toml"), "old").unwrap();
        let changes = vec![
            change("parts/A/X.toml", Some("old"), None),
            change("parts/B/X.toml", None, Some("new")),
        ];
        commit(dir.path(), &changes).unwrap();
        assert!(!dir.path().join("parts/A").exists());
        assert_eq!(
            fs::read_to_string(dir.path().join("parts/B/X.toml")).unwrap(),
            "new"
        );
        let leftovers: Vec<_> = fs::read_dir(dir.path().join("parts/B"))
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().starts_with('.'))
            .collect();
        assert!(leftovers.is_empty());
    }

    #[test]
    fn rolls_back_when_a_step_fails() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.toml"), "a0").unwrap();
        // ディレクトリを置いておくと、そこへの rename は失敗する。
        fs::create_dir_all(dir.path().join("b.toml/inner")).unwrap();
        let changes = vec![
            change("a.toml", Some("a0"), Some("a1")),
            change("b.toml", None, Some("b1")),
        ];
        assert!(commit(dir.path(), &changes).is_err());
        assert_eq!(fs::read_to_string(dir.path().join("a.toml")).unwrap(), "a0");
    }
}
