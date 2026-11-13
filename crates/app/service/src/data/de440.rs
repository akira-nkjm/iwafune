//! DE440 の暦表: リリースからの取得（`gh release download`）・状態・sat-astro への登録。

use super::verify::{Stamp, VerifyCache};
use super::{io_error, sha256_file};
use crate::dto::DataItem;
use crate::error::{Result, ServiceError};
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

/// DE440 のファイルの情報。
pub(super) const DE440: DataSpec = DataSpec {
    id: "de440",
    file: "de440.bin",
    repo: "akira-nkjm/iwafune",
    tag: "data-de440-v1",
    sha256: "75efbddb8ca6b1965a624a12e1cf7b7a9a231982b51106a1ecf000bdc8069d50",
};

pub(super) struct DataSpec {
    pub(super) id: &'static str,
    file: &'static str,
    repo: &'static str,
    tag: &'static str,
    sha256: &'static str,
}

/// プロセスの中で DE440 を読み込んだ結果（一度だけ試す）。
static DE440_INSTALLED: OnceLock<std::result::Result<(), String>> = OnceLock::new();

/// DE440 のファイルの SHA-256 を確かめた結果（GUI の設定画面は開くたびに状態を問い合わせる）。
static DE440_VERIFIED: VerifyCache = VerifyCache::new();

/// DE440 の状態。
pub(super) fn item(dir: &Path) -> Result<DataItem> {
    let path = dir.join(DE440.file);
    let meta = std::fs::metadata(&path).ok();
    let size = meta.as_ref().map(std::fs::Metadata::len);
    let verified = match &meta {
        Some(m) => Some(DE440_VERIFIED.get_or_check(Stamp::of(&path, m), || {
            Ok(sha256_file(&path)? == DE440.sha256)
        })?),
        None => None,
    };
    let install_error = match DE440_INSTALLED.get() {
        Some(Err(e)) => Some(e.clone()),
        _ => None,
    };
    Ok(DataItem {
        id: DE440.id.to_owned(),
        path: path.display().to_string(),
        present: size.is_some(),
        size: size.and_then(|b| crate::units::auto_from(b as f64, "B", "Information")),
        sha256_expected: Some(DE440.sha256.to_owned()),
        verified,
        installed: matches!(DE440_INSTALLED.get(), Some(Ok(()))),
        install_error,
        source: format!("gh release download {} --repo {}", DE440.tag, DE440.repo),
        fetched_at: None,
    })
}

/// DE440 を取得してキャッシュ `dir` に置き、読み込む。一時ディレクトリ `tmp` に落として SHA-256 を確かめてから置き換える。
pub(super) fn fetch(tmp: &Path, dir: &Path) -> Result<()> {
    download_and_verify(tmp, &dir.join(DE440.file))?;
    install_cached_ephemeris();
    Ok(())
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
    let path = match super::cache_dir() {
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
