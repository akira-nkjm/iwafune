//! 外部データ（DE440 の暦表）の取得・状態・読み込み。**ファイルを読むのはここで、sat-astro はバイト列を受け取るだけ。**
//!
//! 置き場所はユーザーごとのキャッシュ: `$IWAFUNE_DATA_DIR`、無ければ `$XDG_CACHE_HOME/iwafune/data`、無ければ `~/.cache/iwafune/data`。
//! 取得は `gh release download`（gh のログインを使う）で行い、SHA-256 を確かめてから置く。

use crate::dto::{DataItem, DataStatus};
use crate::error::{Result, ServiceError};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

/// DE440 のファイルの情報。
const DE440: DataSpec = DataSpec {
    id: "de440",
    file: "de440.bin",
    repo: "akira-nkjm/iwafune",
    tag: "data-de440-v1",
    sha256: "75efbddb8ca6b1965a624a12e1cf7b7a9a231982b51106a1ecf000bdc8069d50",
};

struct DataSpec {
    id: &'static str,
    file: &'static str,
    repo: &'static str,
    tag: &'static str,
    sha256: &'static str,
}

/// プロセスの中で DE440 を読み込んだ結果（一度だけ試す）。
static DE440_INSTALLED: OnceLock<std::result::Result<(), String>> = OnceLock::new();

/// DE440 のファイルの SHA-256 を確かめた結果（GUI の設定画面は開くたびに状態を問い合わせる）。
static DE440_VERIFIED: VerifyCache = VerifyCache::new();

/// キャッシュのディレクトリ。
pub fn cache_dir() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("IWAFUNE_DATA_DIR").filter(|d| !d.is_empty()) {
        return Ok(PathBuf::from(dir));
    }
    if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME").filter(|d| !d.is_empty()) {
        return Ok(PathBuf::from(xdg).join("iwafune/data"));
    }
    std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(|h| PathBuf::from(h).join(".cache/iwafune/data"))
        .ok_or_else(|| {
            ServiceError::Data(
                "キャッシュの場所が決められません（HOME も XDG_CACHE_HOME も無い）".into(),
            )
        })
}

/// データの状態（`GET /data/status`、`fune data status`）。ファイルの SHA-256 も確かめる
/// （同じプロセスの中では、ファイルの大きさと更新時刻が変わらない間は前回の結果を使う）。
pub fn data_status() -> Result<DataStatus> {
    let dir = cache_dir()?;
    let path = dir.join(DE440.file);
    let meta = std::fs::metadata(&path).ok();
    let size = meta.as_ref().map(std::fs::Metadata::len);
    let verified = match &meta {
        Some(m) => Some(DE440_VERIFIED.get_or_check(Stamp::of(&path, m), || {
            Ok(sha256_file(&path)? == DE440.sha256)
        })?),
        None => None,
    };
    let installed = matches!(DE440_INSTALLED.get(), Some(Ok(())));
    let install_error = match DE440_INSTALLED.get() {
        Some(Err(e)) => Some(e.clone()),
        _ => None,
    };
    Ok(DataStatus {
        cache_dir: dir.display().to_string(),
        ephemeris: ephemeris_source(),
        items: vec![DataItem {
            id: DE440.id.to_owned(),
            path: path.display().to_string(),
            present: size.is_some(),
            size: size.and_then(|b| crate::units::auto_from(b as f64, "B", "Information")),
            sha256_expected: DE440.sha256.to_owned(),
            verified,
            installed,
            install_error,
            source: format!("gh release download {} --repo {}", DE440.tag, DE440.repo),
        }],
    })
}

/// DE440 を取得してキャッシュに置く（`POST /data/de440/fetch`、`fune data fetch de440`）。
/// 一時ディレクトリに落として SHA-256 を確かめてから置き換えるので、失敗しても既存のファイルは壊れない。
pub fn fetch(id: &str) -> Result<DataStatus> {
    if id != DE440.id {
        return Err(ServiceError::not_found("データ", id));
    }
    let dir = cache_dir()?;
    std::fs::create_dir_all(&dir).map_err(|e| io_error(&dir, e))?;
    let tmp = dir.join(format!(".download-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).map_err(|e| io_error(&tmp, e))?;
    let result = download_and_verify(&tmp, &dir.join(DE440.file));
    let _ = std::fs::remove_dir_all(&tmp);
    result?;
    install_cached_ephemeris();
    data_status()
}

fn download_and_verify(tmp: &Path, target: &Path) -> Result<()> {
    let output = Command::new("gh")
        .args(["release", "download", DE440.tag, "--repo", DE440.repo])
        .args(["--pattern", DE440.file, "--clobber", "--dir"])
        .arg(tmp)
        .output()
        .map_err(|e| {
            ServiceError::Data(format!(
                "gh を起動できません（GitHub CLI を入れて `gh auth login` してください）: {e}"
            ))
        })?;
    if !output.status.success() {
        return Err(ServiceError::Data(format!(
            "gh release download が失敗しました: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let downloaded = tmp.join(DE440.file);
    let digest = sha256_file(&downloaded)?;
    if digest != DE440.sha256 {
        return Err(ServiceError::Data(format!(
            "SHA-256 が一致しません（期待 {}、実際 {digest}）",
            DE440.sha256
        )));
    }
    std::fs::rename(&downloaded, target).map_err(|e| io_error(target, e))
}

/// キャッシュに DE440 があれば読み込んで sat-astro に渡す（プロセスで一度だけ。CLI と HTTP サーバーの起動時に明示的に呼ぶ）。
pub fn install_cached_ephemeris() {
    let path = match cache_dir() {
        Ok(dir) => dir.join(DE440.file),
        Err(_) => return,
    };
    if !path.is_file() {
        return;
    }
    let _ = DE440_INSTALLED.get_or_init(|| {
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        install_de440_bytes(bytes)
    });
}

/// DE440 のバイト列を sat-astro に渡す。
fn install_de440_bytes(bytes: Vec<u8>) -> std::result::Result<(), String> {
    let table = sat_astro::ephem::De440::from_bytes(bytes).map_err(|e| e.to_string())?;
    sat_astro::ephem::install_de440(table).map_err(|e| e.to_string())
}

fn sha256_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).map_err(|e| io_error(path, e))?;
    let digest = Sha256::digest(&bytes);
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
}

fn io_error(path: &Path, e: std::io::Error) -> ServiceError {
    ServiceError::Data(format!("{}: {e}", path.display()))
}

/// 運用期間（2020 年代）の元期で、解析が使う暦の種類。
fn ephemeris_source() -> sat_astro::ephem::EphemerisSource {
    sat_astro::ephem::source_at(sat_astro::time::Epoch::from_calendar(2026, 1, 1, 0, 0, 0.0))
}

/// ファイルの印（場所・大きさ・更新時刻）。印が同じ間は中身も同じとみなす。
#[derive(Debug, Clone, PartialEq, Eq)]
struct Stamp {
    path: PathBuf,
    len: u64,
    /// 更新時刻が取れない環境では `None`（覚えずに毎回確かめる）
    modified: Option<SystemTime>,
}

impl Stamp {
    fn of(path: &Path, meta: &std::fs::Metadata) -> Self {
        Self {
            path: path.to_path_buf(),
            len: meta.len(),
            modified: meta.modified().ok(),
        }
    }
}

/// SHA-256 を確かめた結果の覚え。ファイルの印が変わったら確かめ直す。
struct VerifyCache(Mutex<Option<(Stamp, bool)>>);

impl VerifyCache {
    const fn new() -> Self {
        Self(Mutex::new(None))
    }

    /// 印が前回と同じなら前回の結果を返し、違えば `check` で確かめて覚える。
    fn get_or_check(&self, stamp: Stamp, check: impl FnOnce() -> Result<bool>) -> Result<bool> {
        let cacheable = stamp.modified.is_some();
        if cacheable
            && let Ok(guard) = self.0.lock()
            && let Some((last, ok)) = guard.as_ref()
            && *last == stamp
        {
            return Ok(*ok);
        }
        let ok = check()?;
        if cacheable && let Ok(mut guard) = self.0.lock() {
            *guard = Some((stamp, ok));
        }
        Ok(ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(calls.get(), 1, "同じ印なら確かめ直さない");
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
