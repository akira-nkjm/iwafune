//! `fune self-update`: 最新のリリース（または指定した版）を確かめ、実行中の `fune` を置き換える。**CLI だけの操作**
//! （CLI のバイナリ自身を置き換えるので HTTP には出さない）。
//!
//! | モジュール | 中身 |
//! |---|---|
//! | `version` | `major.minor.patch` の解析と数としての比較 |
//! | `asset` | 成果物の名前・取得先・実行環境の選択・`.sha256` の読み方（**名前の規則はここだけ**） |
//! | `install` | 取得 → SHA-256 の照合 → 展開 → 版の確認 → 置き換え（失敗しても何も置き換えず、一時ディレクトリを消す） |
//! | `step` | 進行の報告（[`Step`]。表示は CLI） |
//! | `system` | 本物の [`Host`]（`curl`・`tar`・実行中のバイナリの `--version`） |
//!
//! ネットワークとコマンドの実行は [`Host`] を通す。テストはこれを差し替えて、ネットワーク無しで確かめる。
//! 取得は [`crate::fetch`]、SHA-256 は [`crate::data`] と共有する。

mod asset;
mod install;
mod step;
mod system;
mod version;

pub use asset::{Platform, archive_name, dir_name, download_url, parse_sha256_file, repository};
pub use step::{OnStep, Step};
pub use version::Version;

use crate::error::{Result, ServiceError};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// 外部とのやり取り（取得・展開・実行）。
pub trait Host {
    /// `url` の本文を文字列で取る。
    fn fetch_text(&self, url: &str) -> Result<String>;
    /// `url` を `to` に保存する。`progress` のときは進捗バーを出してよい（端末のときだけ CLI が求める）。
    fn download(&self, url: &str, to: &Path, progress: bool) -> Result<()>;
    /// `.tar.gz` を `into` に展開する。
    fn extract(&self, archive: &Path, into: &Path) -> Result<()>;
    /// `binary --version` の出力（標準出力）。
    fn run_version(&self, binary: &Path) -> Result<String>;
}

/// 置き換えの前提（このバイナリの版・環境・場所）。
#[derive(Debug, Clone)]
pub struct Context {
    /// リポジトリの URL
    pub repo: String,
    /// 実行中のバイナリの版
    pub current: Version,
    /// 成果物を選ぶ環境
    pub platform: Platform,
    /// 実行中のバイナリ（実体の絶対パス）
    pub exe: PathBuf,
}

impl Context {
    /// 実行中のバイナリの前提。
    pub fn system() -> Result<Self> {
        let exe = std::env::current_exe()
            .and_then(|p| p.canonicalize())
            .map_err(|e| {
                ServiceError::SelfUpdate(format!("Cannot locate the running fune binary: {e}"))
            })?;
        Ok(Self {
            repo: repository().to_owned(),
            current: Version::parse(env!("CARGO_PKG_VERSION"))?,
            platform: Platform::current()?,
            exe,
        })
    }
}

/// 確かめた結果（`fune self-update --check`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateCheck {
    /// 実行中の版
    pub current: String,
    /// 最新の版（`--version` を付けたときは指定した版）
    pub latest: String,
    /// 置き換える対象があるか（`--version` のときは常に true。同じ版の入れ直しと古い版への戻しも含む）
    pub available: bool,
    /// 実行中の版のほうが最新より新しい（開発中のビルド）
    pub ahead: bool,
    /// `--version` で版を指定した
    pub explicit: bool,
    /// 取得する成果物の名前
    pub asset: String,
    /// 成果物の取得先
    pub url: String,
}

/// 置き換えの結果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateOutcome {
    /// 置き換える前に確かめた内容
    pub check: UpdateCheck,
    /// 置き換えたバイナリの場所（置き換えなかったときは `None`）
    pub replaced: Option<String>,
}

/// 確かめる。`requested` は `--version` の値。
pub fn check(requested: Option<&str>, on_step: OnStep) -> Result<UpdateCheck> {
    check_with(&Context::system()?, &system::SystemHost, requested, on_step)
}

/// 確かめて、新しければ置き換える。`progress` は取得に進捗バーを出すか（端末のときだけ true にする）。
pub fn update(requested: Option<&str>, progress: bool, on_step: OnStep) -> Result<UpdateOutcome> {
    update_with(
        &Context::system()?,
        &system::SystemHost,
        requested,
        progress,
        on_step,
    )
}

/// [`check`]（前提と外部とのやり取りを渡す）。
pub fn check_with(
    ctx: &Context,
    host: &dyn Host,
    requested: Option<&str>,
    on_step: OnStep,
) -> Result<UpdateCheck> {
    let latest = match requested {
        Some(text) => Version::parse(text)?,
        None => {
            on_step(Step::CheckingLatest);
            latest_release(ctx, host)?
        }
    };
    let explicit = requested.is_some();
    let file = archive_name(latest, ctx.platform);
    Ok(UpdateCheck {
        current: ctx.current.to_string(),
        latest: latest.to_string(),
        available: explicit || latest > ctx.current,
        ahead: !explicit && latest < ctx.current,
        explicit,
        url: download_url(&ctx.repo, latest, &file),
        asset: file,
    })
}

/// [`update`]（前提と外部とのやり取りを渡す）。
pub fn update_with(
    ctx: &Context,
    host: &dyn Host,
    requested: Option<&str>,
    progress: bool,
    on_step: OnStep,
) -> Result<UpdateOutcome> {
    let check = check_with(ctx, host, requested, on_step)?;
    if !check.available {
        return Ok(UpdateOutcome {
            check,
            replaced: None,
        });
    }
    let version = Version::parse(&check.latest)?;
    let path = install::install(ctx, host, version, progress, on_step)?;
    Ok(UpdateOutcome {
        check,
        replaced: Some(path.display().to_string()),
    })
}

/// 最新のリリースの版（GitHub の API の `tag_name`）。
fn latest_release(ctx: &Context, host: &dyn Host) -> Result<Version> {
    let url = asset::latest_api_url(&ctx.repo)?;
    let body = host.fetch_text(&url).map_err(|e| {
        ServiceError::SelfUpdate(format!(
            "Cannot find the latest release ({e}). GitHub may be limiting requests; try again later or give a version with --version"
        ))
    })?;
    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        ServiceError::SelfUpdate(format!("Unexpected answer for the latest release: {e}"))
    })?;
    let tag = json
        .get("tag_name")
        .and_then(|t| t.as_str())
        .ok_or_else(|| ServiceError::SelfUpdate("The latest release has no tag_name".into()))?;
    Version::parse(tag)
}

#[cfg(test)]
mod tests;
