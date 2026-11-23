//! カタログ品の置き場所（docs/model.md §7.8・§12）。品のファイルは `<カタログのパス>/<ディレクトリ…>/<ID>.toml`。
//! ディレクトリは置き場所だけで、品の ID（ファイル名）にも部品からの参照にも入らない。

use super::subdir::{dir_below, file_in_dir, parse_subdir, shared_sub_dir, split_below};
use super::{FileRole, IGNORED_DIRS, PROJECT_DIRS, RESERVED_DIRS, SHARED_ITEMS_DIR};
use fune_model::Design;
use fune_model::design::{CatalogSource, EntityKey, split_catalog_ref};

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
                    .map_or_else(|| file_in_dir(&base, &[], id), str::to_string);
                CatalogLocation::Local { file, base }
            }
            None => CatalogLocation::Shared {
                source: source.to_string(),
            },
        },
        None => CatalogLocation::Unknown,
    }
}

/// 品のディレクトリ（カタログのディレクトリからの名前の列。直下なら空）。読んでいない品は空。
/// 共有カタログの品はキャッシュの `catalog/` からの列。
pub fn catalog_item_dir(design: &Design, reference: &str) -> Vec<String> {
    let prefix = || -> Option<String> {
        let (source, _) = split_catalog_ref(reference)?;
        let c: &CatalogSource = design.project.catalogs.iter().find(|c| c.name == source)?;
        match c.local_dir() {
            Some(path) => Some(path.trim_end_matches('/').to_string()),
            None => shared_sub_dir(design, source, SHARED_ITEMS_DIR),
        }
    };
    dir_below(
        design,
        &EntityKey::Catalog(reference.to_string()),
        prefix().as_deref(),
    )
}

/// `catalogs`（カタログ名, ディレクトリ）のうち、このファイル（拡張子を除いた相対パス）を含む最も深いものの役割。
pub(super) fn catalog_role(stem_path: &str, catalogs: &[(String, String)]) -> Option<FileRole> {
    let (name, dirs, id) = catalogs
        .iter()
        .filter_map(|(name, dir)| {
            let (dirs, id) = split_below(stem_path, dir)?;
            Some((dir.len(), name, dirs, id))
        })
        .max_by_key(|(len, ..)| *len)
        .map(|(_, name, dirs, id)| (name, dirs, id))?;
    Some(FileRole::Catalog {
        source: name.clone(),
        dirs,
        id,
    })
}

/// 人が打ったローカルカタログのディレクトリ（プロジェクトのルートからの相対。`a/b`）→ `[[catalog]] path` に書く形。
/// 名前はそれぞれ ID の規則に従い（[`parse_subdir`]）、ルート自身・規約が使うディレクトリ（[`PROJECT_DIRS`]。`parts`・`sources` など）・
/// ほかのカタログのディレクトリと同じか、その中・外であるものは使えない。**ローカルカタログの置き場所の規則はここだけ。**
pub fn parse_local_catalog_path(design: &Design, text: &str) -> Result<String, String> {
    let dirs = parse_subdir(text)?;
    let Some(first) = dirs.first() else {
        return Err(
            "The catalog directory cannot be the project root (give a directory such as `catalog`)"
                .into(),
        );
    };
    let used = PROJECT_DIRS.iter().chain(RESERVED_DIRS).chain(IGNORED_DIRS);
    if used.into_iter().any(|d| d == first) {
        return Err(format!(
            "`{first}` is a directory the project layout uses, so it cannot hold a catalog"
        ));
    }
    let path = dirs.join("/");
    for other in design
        .project
        .catalogs
        .iter()
        .filter_map(CatalogSource::local_dir)
    {
        let other = other.trim_end_matches('/');
        let nested = |a: &str, b: &str| a == b || a.starts_with(&format!("{b}/"));
        if nested(&path, other) || nested(other, &path) {
            return Err(format!(
                "The directory `{path}` overlaps the directory `{other}` of another catalog"
            ));
        }
    }
    Ok(path)
}
