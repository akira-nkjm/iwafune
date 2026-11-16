//! 共有カタログ（`[[catalog]] git = …`）の取得と更新（`fune catalog fetch|update`・`POST /catalog/fetch|update`）。
//! docs/model.md §12。
//!
//! 「外部データのキャッシュ」の例外として、ここだけがプロジェクトの `.fune/catalogs/` に書く（git で管理しない）:
//! 取得元の写し `<名前>/repo.git`（bare）と、commit ごとの中身 `<名前>/<commit>/`（`catalog/`・`sources/` の直下の
//! `.toml` だけ）。読むのは fune-store（`catalog.lock` の commit のキャッシュ）。`catalog.lock` は store の書き込みで書く。
//!
//! | モジュール | 中身 |
//! |---|---|
//! | `git` | git の CLI（写しの作成・rev の解決・commit の取得・ファイルの取り出し） |
//! | `cache` | commit の中身をキャッシュに置く（一時ディレクトリ → 名前の変更） |
//! | `changes` | 前後の設計の共有の品の差分（変わった品と値） |

mod cache;
mod changes;
mod git;

use crate::dto::{CatalogFetchRequest, CatalogSyncEntry, CatalogSyncResult, CatalogUpdateRequest};
use crate::error::{Result, ServiceError};
use crate::workspace::{Workspace, store_error};
use fune_model::design::{CatalogLock, CatalogSource, LockedCatalog};
use fune_store::FileChange;
use fune_store::layout::LOCK_FILE;
use std::path::Path;

/// lock を合わせ、lock の commit を取得してキャッシュに置く（`fune catalog fetch`）。lock が fune.toml と合っていれば
/// commit を動かさない。無い・違う共有カタログは `rev` を解決して lock に書く（`locked` なら書かずに誤り）。
/// lock にあって fune.toml に無い記録は外す。
pub fn fetch(ws: &Workspace, req: &CatalogFetchRequest) -> Result<CatalogSyncResult> {
    sync(ws, None, req.locked)
}

/// 共有カタログ 1 つの lock を `rev` の今の commit に上げる（`fune catalog update <名前>`）。ほかは `fetch` と同じ。
/// 結果に変わった品と値の差分を入れる。
pub fn update(ws: &Workspace, req: &CatalogUpdateRequest) -> Result<CatalogSyncResult> {
    sync(ws, Some(&req.name), false)
}

fn sync(ws: &Workspace, target: Option<&str>, locked: bool) -> Result<CatalogSyncResult> {
    let root = ws.root();
    let _lock = fune_store::lock::ProjectLock::acquire(root, fune_ops::apply::LOCK_WAIT)
        .map_err(store_error)?;
    let before = fune_store::load(root).map_err(store_error)?;
    let design = &before.design;
    if let Some(name) = target
        && !design
            .project
            .catalogs
            .iter()
            .any(|c| c.name == name && c.shared_git().is_some())
    {
        return Err(ServiceError::not_found("shared catalog", name));
    }
    let old = design.catalog_lock.clone().unwrap_or_default();
    let mut lock = CatalogLock::default();
    let mut catalogs = Vec::new();
    for source in &design.project.catalogs {
        let Some(git) = source.shared_git() else {
            continue;
        };
        let previous = old.get(&source.name);
        let keep = previous
            .filter(|l| l.matches(source) && target != Some(source.name.as_str()))
            .map(|l| l.commit.as_str());
        if keep.is_none() && locked {
            return Err(not_locked(&source.name));
        }
        let commit = fetch_one(root, source, git, keep).map_err(|e| named(&source.name, e))?;
        catalogs.push(CatalogSyncEntry {
            name: source.name.clone(),
            git: git.to_owned(),
            rev: source.rev.clone(),
            commit: commit.clone(),
            previous: previous.map(|l| l.commit.clone()),
        });
        lock.catalogs.push(LockedCatalog {
            name: source.name.clone(),
            git: git.to_owned(),
            rev: source.rev.clone(),
            commit,
        });
    }
    let lock_changed = write_lock(root, before.files.get(LOCK_FILE), &lock, locked)?;
    let after = fune_store::load(root).map_err(store_error)?;
    Ok(CatalogSyncResult {
        catalogs,
        lock_changed,
        items: changes::between(&before.design, &after.design),
    })
}

/// 共有カタログ 1 つを取得してキャッシュに置き、commit を返す。`keep` は動かさない lock の commit。
/// 既にキャッシュにあれば git を呼ばない（取得元に繋がらなくても読める）。
fn fetch_one(root: &Path, source: &CatalogSource, git: &str, keep: Option<&str>) -> Result<String> {
    let name = source.name.as_str();
    fune_model::id::validate_entity_id(name)
        .map_err(|e| ServiceError::InvalidInput(e.to_string()))?;
    if let Some(commit) = keep
        && cache::exists(root, name, commit)
    {
        return Ok(commit.to_owned());
    }
    let mirror = git::Mirror::open(&root.join(fune_store::layout::shared_catalog_mirror(name)))?;
    let commit = match keep {
        Some(commit) => {
            mirror.ensure(git, commit)?;
            commit.to_owned()
        }
        None => mirror.resolve(git, source.rev.as_deref())?,
    };
    cache::extract(root, name, &mirror, &commit)?;
    Ok(commit)
}

/// lock の本文が変わるなら書く（書いたら true）。`locked` なら書かずに誤り。
fn write_lock(
    root: &Path,
    current: Option<&str>,
    lock: &CatalogLock,
    locked: bool,
) -> Result<bool> {
    if current.is_none() && lock.catalogs.is_empty() {
        return Ok(false);
    }
    let text = fune_store::write::project::catalog_lock_text(lock).map_err(store_error)?;
    if current == Some(text.as_str()) {
        return Ok(false);
    }
    if locked {
        return Err(ServiceError::InvalidInput(
            "catalog.lock does not match fune.toml, so --locked will not rewrite it. Run `fune catalog fetch` and commit catalog.lock".into(),
        ));
    }
    let change = FileChange {
        path: LOCK_FILE.to_owned(),
        before: current.map(str::to_owned),
        after: Some(text),
    };
    fune_store::write::commit(root, &[change]).map_err(store_error)?;
    Ok(true)
}

fn not_locked(name: &str) -> ServiceError {
    ServiceError::InvalidInput(format!(
        "catalog.lock for shared catalog `{name}` is missing or does not match fune.toml, so --locked cannot fetch it. Run `fune catalog fetch` and commit catalog.lock"
    ))
}

/// 取得の誤りに共有カタログの名前を付ける。
fn named(name: &str, e: ServiceError) -> ServiceError {
    match e {
        ServiceError::Data(message) => {
            ServiceError::Data(format!("Shared catalog `{name}`: {message}"))
        }
        other => other,
    }
}
