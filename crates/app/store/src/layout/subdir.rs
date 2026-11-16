//! ディレクトリの下に置けるエンティティ（`catalog/`・`sources/`。docs/model.md §11.1）の置き場所の規則。
//! ディレクトリは置き場所だけで、ID（ファイル名）にも参照にも入らない。カタログ品も出典もここを通る
//! （ディレクトリ名の読み方・パスの組み立て・パスの分解・読んだあとのディレクトリ）。

use super::shared_catalog_dir;
use fune_model::Design;
use fune_model::design::EntityKey;
use fune_model::id::validate_entity_id;

/// 人が打ったディレクトリ（基準のディレクトリからの相対。`a/b`。`.` は基準の直下）→ ディレクトリ名の列。
/// 名前はそれぞれ ID の規則に従う（読み込みの検査と同じ [`validate_entity_id`]）。**この文字列の読み方はここだけ。**
pub fn parse_subdir(text: &str) -> Result<Vec<String>, String> {
    let text = text.trim();
    if text == "." {
        return Ok(Vec::new());
    }
    if text.is_empty() {
        return Err("The directory is empty (use `.` for the root)".into());
    }
    text.split('/')
        .map(|seg| {
            validate_entity_id(seg)
                .map(|()| seg.to_string())
                .map_err(|e| format!("Invalid directory name in `{text}`: {e}"))
        })
        .collect()
}

/// `<base>/<dirs…>/<id>.toml`。**このパスの組み立てはここだけ。**
pub fn file_in_dir(base: &str, dirs: &[String], id: &str) -> String {
    let mut path = base.to_string();
    for d in dirs {
        path.push('/');
        path.push_str(d);
    }
    format!("{path}/{id}.toml")
}

/// ディレクトリの列の表示（`a/b`。直下は `.`）。
pub fn dirs_text(dirs: &[String]) -> String {
    if dirs.is_empty() {
        ".".to_string()
    } else {
        dirs.join("/")
    }
}

/// 拡張子を除いた相対パスを `base` の下の（ディレクトリの列, ID）に分ける。`base` の下でない・名前が空なら None。
pub fn split_below(stem_path: &str, base: &str) -> Option<(Vec<String>, String)> {
    let rest = stem_path.strip_prefix(base)?.strip_prefix('/')?;
    let mut dirs: Vec<String> = rest.split('/').map(str::to_string).collect();
    let id = dirs.pop().filter(|id| !id.is_empty())?;
    if dirs.iter().any(String::is_empty) {
        return None;
    }
    Some((dirs, id))
}

/// 取得した共有カタログの `catalog/`・`sources/` の下の相対パスのうち、キャッシュに置く（読む）もの。
/// `.toml` で、`.` で始まる名前と空の名前を含まない。**キャッシュに置く条件はここだけ。**
pub fn is_cacheable(rel: &str) -> bool {
    rel.ends_with(".toml")
        && rel
            .split('/')
            .all(|seg| !seg.is_empty() && !seg.starts_with('.'))
}

/// 共有カタログのキャッシュの中の `sub`（`catalog` か `sources`）のディレクトリ。`catalog.lock` に無ければ None。
pub fn shared_sub_dir(design: &Design, name: &str, sub: &str) -> Option<String> {
    let commit = &design.catalog_lock.as_ref()?.get(name)?.commit;
    Some(format!("{}/{sub}", shared_catalog_dir(name, commit)))
}

/// 読んだエンティティのディレクトリ（`prefix` からの名前の列。直下なら空）。位置は source map のファイルから取る。
/// 読んでいなければ空。
pub fn dir_below(design: &Design, key: &EntityKey, prefix: Option<&str>) -> Vec<String> {
    let dir = || -> Option<Vec<String>> {
        let file = design.source_map.file_of(key)?;
        let (dirs, _) = split_below(file.strip_suffix(".toml")?, prefix?)?;
        Some(dirs)
    };
    dir().unwrap_or_default()
}
