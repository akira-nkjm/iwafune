//! 出典の置き場所（docs/model.md §7.8・§11.1）。出典のファイルは `sources/<ディレクトリ…>/<ID>.toml`。
//! 置き場所の規則は [`super::subdir`]（カタログ品と同じ）で、ここは出典の基準のディレクトリと位置だけを決める。

use super::SOURCES_DIR;
use super::subdir::{dir_below, file_in_dir, shared_sub_dir};
use fune_model::Design;
use fune_model::design::EntityKey;

/// 出典のファイル（`sources/<dirs…>/<id>.toml`）。
pub fn source_path(dirs: &[String], id: &str) -> String {
    file_in_dir(SOURCES_DIR, dirs, id)
}

/// 出典の今のファイル（読んだ位置）。読んでいない出典は `sources/` の直下の既定の位置。
pub fn source_file(design: &Design, id: &str) -> String {
    design
        .source_map
        .file_of(&EntityKey::Source(id.to_string()))
        .map_or_else(|| source_path(&[], id), str::to_string)
}

/// 出典のディレクトリ（`sources/` からの名前の列。直下なら空）。読んでいない出典は空。
/// 共有カタログの出典はキャッシュの `sources/` からの列。
pub fn source_dir(design: &Design, id: &str) -> Vec<String> {
    let prefix = match design.shared_catalog_of(id) {
        Some(c) => shared_sub_dir(design, &c.name, SOURCES_DIR),
        None => Some(SOURCES_DIR.to_string()),
    };
    dir_below(
        design,
        &EntityKey::Source(id.to_string()),
        prefix.as_deref(),
    )
}
