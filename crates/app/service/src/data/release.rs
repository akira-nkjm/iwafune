//! リリース配布のデータ（DE440 など）の共通の仕組み。**データ 1 つは [`Dataset`] 1 件の表の行**で、
//! 取得（`gh release download`）・SHA-256 の照合・状態・sat-astro などへの登録はすべてここを通る。
//! 新しいデータを足すときは、表（`data::RELEASE_DATASETS`）に 1 行と、バイト列を受け取って登録する関数（[`Installer`]）を足す。

use super::verify::{Stamp, VerifyCache};
use super::{io_error, sha256_file};
use crate::dto::DataItem;
use crate::error::{Result, ServiceError};
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

/// バイト列を受け取って、使う側（sat-astro など）に登録する関数。失敗は理由の文で返す（状態に出す）。
pub(super) type Installer = fn(Vec<u8>) -> std::result::Result<(), String>;

/// リリースの成果物を取ってくる関数（成果物 [`ReleaseSpec::file`] を `tmp` の下に置く）。試験では差し替える。
pub(super) type Downloader<'a> = &'a dyn Fn(&ReleaseSpec, &Path) -> Result<()>;

/// データ 1 つの決まり（表の 1 行の中身）。
pub(super) struct ReleaseSpec {
    /// ID（`fune data fetch <id>`、`POST /data/<id>/fetch`）
    pub(super) id: &'static str,
    /// リリースの成果物の名前で、キャッシュに置くファイル名でもある
    pub(super) file: &'static str,
    /// リリースを持つリポジトリ（`owner/name`）
    pub(super) repo: &'static str,
    /// リリースのタグ
    pub(super) tag: &'static str,
    /// 成果物の SHA-256（小文字の 16 進）
    pub(super) sha256: &'static str,
    /// 成果物の大きさ（バイト）。取得したものの取り違えを、ハッシュの前に知らせるのに使う
    pub(super) size: u64,
    /// 何に使うか（利用者に見せる。英語）
    pub(super) used_by: &'static str,
    /// 出所と利用条件（利用者に見せる。英語）
    pub(super) license: &'static str,
    /// 取得したバイト列を登録する関数
    pub(super) installer: Installer,
}

/// 表の 1 行と、プロセスの中での状態。
pub(super) struct Dataset {
    pub(super) spec: ReleaseSpec,
    /// 登録を試みた結果（一度だけ試す）
    installed: OnceLock<std::result::Result<(), String>>,
    /// SHA-256 を確かめた結果（GUI の設定画面は開くたびに状態を問い合わせる）
    verified: VerifyCache,
}

impl Dataset {
    pub(super) const fn new(spec: ReleaseSpec) -> Self {
        Self {
            spec,
            installed: OnceLock::new(),
            verified: VerifyCache::new(),
        }
    }

    /// 状態。
    pub(super) fn item(&self, dir: &Path) -> Result<DataItem> {
        let spec = &self.spec;
        let path = dir.join(spec.file);
        let meta = std::fs::metadata(&path).ok();
        let size = meta.as_ref().map(std::fs::Metadata::len);
        let verified = match &meta {
            Some(m) => Some(self.verified.get_or_check(Stamp::of(&path, m), || {
                Ok(sha256_file(&path)? == spec.sha256)
            })?),
            None => None,
        };
        let install_error = match self.installed.get() {
            Some(Err(e)) => Some(e.clone()),
            _ => None,
        };
        Ok(DataItem {
            id: spec.id.to_owned(),
            path: path.display().to_string(),
            present: size.is_some(),
            size: size.and_then(|b| crate::units::auto_from(b as f64, "B", "Information")),
            sha256_expected: Some(spec.sha256.to_owned()),
            verified,
            installed: matches!(self.installed.get(), Some(Ok(()))),
            install_error,
            source: format!("gh release download {} --repo {}", spec.tag, spec.repo),
            fetched_at: None,
            used_by: spec.used_by.to_owned(),
            license: Some(spec.license.to_owned()),
        })
    }

    /// 取得してキャッシュ `dir` に置き、登録する。一時ディレクトリ `tmp` に落として大きさと SHA-256 を確かめてから置き換えるので、
    /// 食い違えば既存のファイルはそのまま。
    pub(super) fn fetch(&self, tmp: &Path, dir: &Path, download: Downloader<'_>) -> Result<()> {
        let spec = &self.spec;
        download(spec, tmp)?;
        let downloaded = tmp.join(spec.file);
        let len = std::fs::metadata(&downloaded)
            .map_err(|e| io_error(&downloaded, e))?
            .len();
        if len != spec.size {
            return Err(ServiceError::Data(format!(
                "Size mismatch for {} (expected {} bytes, got {len})",
                spec.id, spec.size
            )));
        }
        let digest = sha256_file(&downloaded)?;
        if digest != spec.sha256 {
            return Err(ServiceError::Data(format!(
                "SHA-256 mismatch for {} (expected {}, got {digest})",
                spec.id, spec.sha256
            )));
        }
        let target = dir.join(spec.file);
        std::fs::rename(&downloaded, &target).map_err(|e| io_error(&target, e))?;
        self.install_cached(dir);
        Ok(())
    }

    /// キャッシュにファイルがあれば読み込んで登録する（プロセスで一度だけ）。
    pub(super) fn install_cached(&self, dir: &Path) {
        let path = dir.join(self.spec.file);
        if !path.is_file() {
            return;
        }
        let _ = self.installed.get_or_init(|| {
            let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            (self.spec.installer)(bytes)
        });
    }
}

/// 本物の取得: `gh release download`（GitHub CLI が要る）。
pub(super) fn gh_download(spec: &ReleaseSpec, tmp: &Path) -> Result<()> {
    let output = Command::new("gh")
        .args(["release", "download", spec.tag, "--repo", spec.repo])
        .args(["--pattern", spec.file, "--clobber", "--dir"])
        .arg(tmp)
        .output()
        .map_err(|e| {
            ServiceError::Data(format!(
                "Cannot start gh (install the GitHub CLI and run `gh auth login`): {e}"
            ))
        })?;
    if !output.status.success() {
        return Err(ServiceError::Data(format!(
            "gh release download failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}
