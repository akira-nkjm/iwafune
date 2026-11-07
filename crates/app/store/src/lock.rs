//! プロセス間の書き込みロック（`.fune/lock`）。
//!
//! `create_new` でロックファイルを作れたプロセスだけが書く。`Drop` で消す。
//! 古すぎるロック（[`STALE_AFTER`] より前のもの）は落ちたプロセスの残骸とみなして取り直す。

use crate::error::StoreError;
use crate::layout::WORK_DIR;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// これより古いロックは残骸とみなす。
pub const STALE_AFTER: Duration = Duration::from_secs(120);

/// 書き込みロック。持っている間だけ有効。
#[derive(Debug)]
pub struct ProjectLock {
    path: PathBuf,
}

impl ProjectLock {
    /// ロックを取る。`wait` の間は取れるまで待つ。
    pub fn acquire(root: &Path, wait: Duration) -> Result<Self, StoreError> {
        let dir = root.join(WORK_DIR);
        fs::create_dir_all(&dir).map_err(|e| StoreError::io(&dir, e))?;
        let path = dir.join("lock");
        let start = Instant::now();
        loop {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(mut file) => {
                    // 中身は人が見るためだけのもの（書けなくてもロックは有効）。
                    let _ = writeln!(file, "pid {}", std::process::id());
                    return Ok(Self { path });
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    if is_stale(&path) {
                        // 残骸を消して取り直す（消せなければ次の周回で待つ）。
                        let _ = fs::remove_file(&path);
                        continue;
                    }
                    if start.elapsed() >= wait {
                        return Err(StoreError::Locked {
                            lock: path.display().to_string(),
                        });
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(e) => return Err(StoreError::io(&path, e)),
            }
        }
    }
}

fn is_stale(path: &Path) -> bool {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age > STALE_AFTER)
}

impl Drop for ProjectLock {
    fn drop(&mut self) {
        // 消せなくても次の取得が残骸として扱う。
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_lock_waits_then_fails() {
        let dir = tempfile::tempdir().unwrap();
        let first = ProjectLock::acquire(dir.path(), Duration::ZERO).unwrap();
        let second = ProjectLock::acquire(dir.path(), Duration::from_millis(50));
        assert!(matches!(second, Err(StoreError::Locked { .. })));
        drop(first);
        assert!(ProjectLock::acquire(dir.path(), Duration::ZERO).is_ok());
    }
}
