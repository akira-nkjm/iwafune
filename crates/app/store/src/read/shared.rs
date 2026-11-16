//! 共有カタログ（`[[catalog]] git = …`）の読み込み（docs/model.md §12）。
//!
//! - 走査: `catalog.lock` の commit ごとに、キャッシュ `.fune/catalogs/<名前>/<commit>/` の `catalog/`・`sources/`（どちらもディレクトリの下も）を
//!   読み取り専用の層（[`FileSet::shared`]）に読む。キャッシュを作るのは fune-service（`fune catalog fetch`）。
//! - 解釈: 品は `<名前>/<ID>`、出典も `<名前>/<ID>` の出所付きの ID で Design に入れ、品の中の出典の参照
//!   （`source`・`datasheet`・値の `from.source`）にも同じ出所を付ける（プロジェクトの `sources/` と ID が衝突しない）。

use super::decode::Decoder;
use super::files::FileSet;
use crate::error::StoreError;
use crate::layout::{self, LOCK_FILE, SHARED_CATALOG_DIRS, SHARED_ITEMS_DIR, SOURCES_DIR};
use fune_model::design::source::is_location;
use fune_model::design::{CatalogItem, CatalogLock, EntityKey, catalog_ref};
use fune_model::value::{Provenance, Value};
use std::fs;
use std::path::Path;

/// `catalog.lock` の本文を読む（形の誤りは文で返す）。
pub fn parse_lock(text: &str) -> Result<CatalogLock, String> {
    toml::from_str(text).map_err(|e| e.message().to_string())
}

/// lock の commit ごとのキャッシュを `set` の読み取り専用の層に読む（lock が無い・読めない・キャッシュが無いものは飛ばす。
/// 状態は検査が lock とキャッシュの有無から判定する）。
pub(super) fn scan_cache(root: &Path, set: &mut FileSet) -> Result<(), StoreError> {
    let Some(lock) = set.get(LOCK_FILE).and_then(|t| parse_lock(t).ok()) else {
        return Ok(());
    };
    for entry in &lock.catalogs {
        if !layout::is_cache_key(&entry.name, &entry.commit) {
            continue;
        }
        let dir = root.join(layout::shared_catalog_dir(&entry.name, &entry.commit));
        if !dir.is_dir() {
            continue;
        }
        let mut files = FileSet::new();
        for sub in SHARED_CATALOG_DIRS {
            read_toml(&dir.join(sub), sub, &mut files)?;
        }
        set.insert_shared(entry.name.clone(), files);
    }
    Ok(())
}

/// ディレクトリの下の `.toml` を `<rel>/<パス>.toml` として読む（無ければ何もしない。`.` で始まる名前は飛ばす）。
fn read_toml(dir: &Path, rel: &str, out: &mut FileSet) -> Result<(), StoreError> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry.map_err(|e| StoreError::io(dir, e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            read_toml(&path, &format!("{rel}/{name}"), out)?;
        } else if name.ends_with(".toml") && path.is_file() {
            let text = fs::read_to_string(&path).map_err(|e| StoreError::io(&path, e))?;
            out.insert(format!("{rel}/{name}"), text);
        }
    }
    Ok(())
}

impl Decoder<'_> {
    /// キャッシュのファイルを、ルートからの相対パス（`.fune/catalogs/<名前>/<commit>/…`）で引けるようにする
    /// （診断の位置と、読み込みの本文の引き先）。
    pub(super) fn collect_shared(&mut self) {
        let files = self.files;
        let lock = files.get(LOCK_FILE).and_then(|t| parse_lock(t).ok());
        for (name, shared) in files.shared() {
            let Some(entry) = lock.as_ref().and_then(|l| l.get(name)) else {
                continue;
            };
            let dir = layout::shared_catalog_dir(name, &entry.commit);
            for (path, text) in shared.iter() {
                self.shared.insert(format!("{dir}/{path}"), text);
            }
        }
    }

    /// `catalog.lock` を読む（無ければ None のまま）。
    pub(super) fn read_lock(&mut self) {
        let Some(text) = self.files.get(LOCK_FILE) else {
            return;
        };
        match parse_lock(text) {
            Ok(lock) => self.design.catalog_lock = Some(lock),
            Err(message) => self.diag(
                fune_model::Severity::Error,
                "toml.schema",
                LOCK_FILE,
                None,
                format!("{message} (rebuild it with `fune catalog fetch`)"),
            ),
        }
    }

    /// 読んだキャッシュのうち、fune.toml の共有カタログのものを品と出典として読む。
    pub(super) fn read_shared(&mut self) {
        let files = self.files;
        for (name, shared) in files.shared() {
            let commit = self
                .design
                .catalog_lock
                .as_ref()
                .and_then(|l| l.get(name))
                .map(|l| l.commit.clone());
            let is_shared = self
                .design
                .project
                .catalogs
                .iter()
                .any(|c| c.name == name && c.shared_git().is_some());
            let (Some(commit), true) = (commit, is_shared) else {
                continue;
            };
            let dir = layout::shared_catalog_dir(name, &commit);
            for path in shared.paths() {
                self.read_shared_file(name, &dir, path);
            }
            self.design.fetched_catalogs.insert(name.to_owned());
        }
    }

    /// キャッシュのファイル 1 つ（`catalog/[<ディレクトリ>/…]<ID>.toml` か `sources/[<ディレクトリ>/…]<ID>.toml`）。
    /// ID はファイル名で、ディレクトリは置き場所だけ。
    fn read_shared_file(&mut self, name: &str, dir: &str, rel: &str) {
        let Some((sub, _)) = rel.split_once('/') else {
            return;
        };
        let Some((dirs, id)) = rel
            .strip_suffix(".toml")
            .and_then(|stem| layout::split_below(stem, sub))
        else {
            return;
        };
        let path = format!("{dir}/{rel}");
        let reference = catalog_ref(name, &id);
        if sub == SHARED_ITEMS_DIR {
            let duplicate = self.design.catalog.contains_key(&reference);
            if !duplicate {
                self.design
                    .source_map
                    .insert_file(EntityKey::Catalog(reference.clone()), &path);
            }
            if self.read_catalog(&path, &reference, &dirs)
                && let Some(item) = self.design.catalog.get_mut(&reference)
            {
                qualify_item(item, name);
            }
        } else if sub == SOURCES_DIR {
            if !self.design.sources.contains_key(reference.as_str()) {
                self.design
                    .source_map
                    .insert_file(EntityKey::Source(reference.clone()), &path);
            }
            self.read_source(&path, &reference, &dirs);
        }
    }
}

/// 共有カタログの品の中の出典の参照に、そのカタログの名前を付ける（`ds_rw` → `lab/ds_rw`）。
/// `datasheet` に URL・パスを直接書いた古い形はそのまま残す（検査が `source.datasheet_location` の warning にする）。
fn qualify_item(item: &mut CatalogItem, name: &str) {
    let qualify = |id: &mut Option<String>| {
        if let Some(text) = id.as_mut() {
            *text = catalog_ref(name, text);
        }
    };
    qualify(&mut item.source);
    if item.datasheet.as_deref().is_some_and(|d| !is_location(d)) {
        qualify(&mut item.datasheet);
    }
    for value in item.attr.values_mut() {
        qualify_from(&mut value.from, name);
        if let Value::ByState(cells) = &mut value.value {
            for cell in cells.values_mut() {
                qualify_from(&mut cell.from, name);
            }
        }
    }
}

fn qualify_from(from: &mut Option<Provenance>, name: &str) {
    if let Some(source) = from.as_mut().and_then(|f| f.source.as_mut()) {
        *source = catalog_ref(name, source);
    }
}
