//! カタログ品の置き場所（docs/model.md §7.8・§12）。品のファイルは `<カタログのパス>/<ディレクトリ…>/<ID>.toml`。
//! ディレクトリは置き場所だけで、品の ID（ファイル名）にも部品からの参照にも入らない。

use super::{FileRole, SHARED_ITEMS_DIR, shared_catalog_dir};
use fune_model::Design;
use fune_model::design::{CatalogSource, EntityKey, split_catalog_ref};
use fune_model::id::validate_entity_id;

/// カタログ品の置き場所。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogLocation {
    /// このプロジェクトの中（`[[catalog]] path = …`）。書き換えられる。`file` は今の（読んだ）ファイル、
    /// 読んでいない品はカタログの直下の既定の位置。`base` はカタログのディレクトリ
    Local { file: String, base: String },
    /// 共有カタログ（`[[catalog]] git = …`）。読み取り専用
    Shared { source: String },
    /// `<カタログ名>/<ID>` の形でない、またはカタログ名が fune.toml に無い
    Unknown,
}

/// カタログ品の参照（`"local/rw_10mNms"`）→ 置き場所。
pub fn catalog_location(design: &Design, reference: &str) -> CatalogLocation {
    let Some((source, id)) = split_catalog_ref(reference) else {
        return CatalogLocation::Unknown;
    };
    match design.project.catalogs.iter().find(|c| c.name == source) {
        Some(c) => match c.local_dir() {
            Some(path) => {
                let base = path.trim_end_matches('/').to_string();
                let file = design
                    .source_map
                    .file_of(&EntityKey::Catalog(reference.to_string()))
                    .map_or_else(|| catalog_item_path(&base, &[], id), str::to_string);
                CatalogLocation::Local { file, base }
            }
            None => CatalogLocation::Shared {
                source: source.to_string(),
            },
        },
        None => CatalogLocation::Unknown,
    }
}

/// `<カタログのディレクトリ>/<dirs…>/<id>.toml`。
pub fn catalog_item_path(base: &str, dirs: &[String], id: &str) -> String {
    let mut path = base.to_string();
    for d in dirs {
        path.push('/');
        path.push_str(d);
    }
    format!("{path}/{id}.toml")
}

/// 人が打ったディレクトリ（カタログのディレクトリからの相対。`a/b`。`.` はカタログの直下）→ ディレクトリ名の列。
/// 名前はそれぞれ ID の規則に従う（読み込みの検査と同じ [`validate_entity_id`]）。**この文字列の読み方はここだけ。**
pub fn parse_catalog_dir(text: &str) -> Result<Vec<String>, String> {
    let text = text.trim();
    if text == "." {
        return Ok(Vec::new());
    }
    if text.is_empty() {
        return Err("The directory is empty (use `.` for the catalog root)".into());
    }
    text.split('/')
        .map(|seg| {
            validate_entity_id(seg)
                .map(|()| seg.to_string())
                .map_err(|e| format!("Invalid directory name in `{text}`: {e}"))
        })
        .collect()
}

/// 品のディレクトリ（カタログのディレクトリからの名前の列。直下なら空）。読んでいない品は空。
/// 共有カタログの品はキャッシュの `catalog/` からの列。
pub fn catalog_item_dir(design: &Design, reference: &str) -> Vec<String> {
    let dir = || -> Option<Vec<String>> {
        let (source, _) = split_catalog_ref(reference)?;
        let c: &CatalogSource = design.project.catalogs.iter().find(|c| c.name == source)?;
        let prefix = match c.local_dir() {
            Some(path) => path.trim_end_matches('/').to_string(),
            None => {
                let commit = &design.catalog_lock.as_ref()?.get(source)?.commit;
                format!("{}/{SHARED_ITEMS_DIR}", shared_catalog_dir(source, commit))
            }
        };
        let file = design
            .source_map
            .file_of(&EntityKey::Catalog(reference.to_string()))?;
        let rest = file.strip_prefix(&prefix)?.strip_prefix('/')?;
        let mut segments: Vec<String> = rest.split('/').map(str::to_string).collect();
        segments.pop();
        Some(segments)
    };
    dir().unwrap_or_default()
}

/// `catalogs`（カタログ名, ディレクトリ）のうち、このファイル（拡張子を除いた相対パス）を含む最も深いものの役割。
pub(super) fn catalog_role(stem_path: &str, catalogs: &[(String, String)]) -> Option<FileRole> {
    let (name, rest) = catalogs
        .iter()
        .filter_map(|(name, dir)| {
            let rest = stem_path.strip_prefix(dir.as_str())?.strip_prefix('/')?;
            Some((dir.len(), name, rest))
        })
        .max_by_key(|(len, ..)| *len)
        .map(|(_, name, rest)| (name, rest))?;
    let mut dirs: Vec<String> = rest.split('/').map(str::to_string).collect();
    let id = dirs.pop().filter(|id| !id.is_empty())?;
    if dirs.iter().any(String::is_empty) {
        return None;
    }
    Some(FileRole::Catalog {
        source: name.clone(),
        dirs,
        id,
    })
}
