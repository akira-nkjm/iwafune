//! ファイルから求めた結果（SHA-256 の照合・読み込んだ表）の覚え。ファイルの印が変わらない間は求め直さない。

use crate::error::Result;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

/// ファイルの印（場所・大きさ・更新時刻）。印が同じ間は中身も同じとみなす。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Stamp {
    path: PathBuf,
    len: u64,
    /// 更新時刻が取れない環境では `None`（覚えずに毎回確かめる）
    modified: Option<SystemTime>,
}

impl Stamp {
    pub(super) fn of(path: &Path, meta: &std::fs::Metadata) -> Self {
        Self {
            path: path.to_path_buf(),
            len: meta.len(),
            modified: meta.modified().ok(),
        }
    }
}

/// ファイルから求めた結果の覚え（SHA-256 を確かめた結果・読み込んだ表）。ファイルの印が変わったら求め直す。
pub(super) struct StampCache<T>(Mutex<Option<(Stamp, T)>>);

/// SHA-256 を確かめた結果の覚え。
pub(super) type VerifyCache = StampCache<bool>;

impl<T: Clone> StampCache<T> {
    pub(super) const fn new() -> Self {
        Self(Mutex::new(None))
    }

    /// 印が前回と同じなら前回の結果を返し、違えば `check` で求めて覚える（誤りは覚えない）。
    pub(super) fn get_or_check(
        &self,
        stamp: Stamp,
        check: impl FnOnce() -> Result<T>,
    ) -> Result<T> {
        let cacheable = stamp.modified.is_some();
        if cacheable
            && let Ok(guard) = self.0.lock()
            && let Some((last, value)) = guard.as_ref()
            && *last == stamp
        {
            return Ok(value.clone());
        }
        let value = check()?;
        if cacheable && let Ok(mut guard) = self.0.lock() {
            *guard = Some((stamp, value.clone()));
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ServiceError;
    use std::cell::Cell;
    use std::time::Duration;

    fn stamp(len: u64, secs: u64) -> Stamp {
        Stamp {
            path: PathBuf::from("/cache/de440.bin"),
            len,
            modified: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(secs)),
        }
    }

    #[test]
    fn verify_cache_reuses_the_result_while_the_file_is_unchanged() {
        let cache = VerifyCache::new();
        let calls = Cell::new(0);
        let check = |ok: bool| {
            calls.set(calls.get() + 1);
            Ok(ok)
        };
        assert!(cache.get_or_check(stamp(10, 1), || check(true)).unwrap());
        assert!(cache.get_or_check(stamp(10, 1), || check(false)).unwrap());
        assert_eq!(calls.get(), 1, "the same stamp is not verified again");
        // 大きさか更新時刻が変われば確かめ直す
        assert!(!cache.get_or_check(stamp(11, 1), || check(false)).unwrap());
        assert!(cache.get_or_check(stamp(11, 2), || check(true)).unwrap());
        assert_eq!(calls.get(), 3);
    }

    #[test]
    fn verify_cache_does_not_remember_without_a_modified_time() {
        let cache = VerifyCache::new();
        let calls = Cell::new(0);
        let no_time = Stamp {
            modified: None,
            ..stamp(10, 1)
        };
        for _ in 0..2 {
            cache
                .get_or_check(no_time.clone(), || {
                    calls.set(calls.get() + 1);
                    Ok(true)
                })
                .unwrap();
        }
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn verify_cache_does_not_remember_errors() {
        let cache = VerifyCache::new();
        let failed = cache.get_or_check(stamp(10, 1), || Err(ServiceError::Data("io".into())));
        assert!(failed.is_err());
        assert!(cache.get_or_check(stamp(10, 1), || Ok(true)).unwrap());
    }
}
