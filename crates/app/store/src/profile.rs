//! TOML の共通プロファイル処理（read・write・format から使う）。
use crate::error::StoreError;
use toml_edit::{DocumentMut, Item, Table};
/// 整数のまま書くキー（ID・員数・回数）。
pub const INTEGER_KEYS: &[&str] = &["count", "cores", "schema", "orbits"];
/// `view/` の座標（画面の整数座標）。`points` は手で引いた線の折れ点（`[edge]` の `[[x, y], …]`）、
/// `*_offset` は線の端子のカードの側の上での位置。
pub const VIEW_INTEGER_KEYS: &[&str] = &[
    "x",
    "y",
    "w",
    "h",
    "points",
    "source_offset",
    "target_offset",
];

/// ファイルごとの整数のまま書くキー。
pub fn integer_keys(path: &str) -> &'static [&'static str] {
    if path.starts_with("view/") {
        VIEW_INTEGER_KEYS
    } else {
        INTEGER_KEYS
    }
}
/// 本文を編集用の文書にする。
pub fn parse(path: &str, text: &str) -> Result<DocumentMut, StoreError> {
    text.parse::<DocumentMut>().map_err(|e| StoreError::Toml {
        file: path.to_string(),
        message: e.message().to_string(),
    })
}

/// 文書の `=` を揃えて本文にする（新しいファイルの書き出しにも使う）。
pub fn finish(mut doc: DocumentMut) -> String {
    align_table(doc.as_table_mut());
    let text = doc.to_string();
    let mut out: String = text
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    let trimmed = out.trim_start_matches('\n').len();
    out = out[out.len() - trimmed..].to_string();
    while out.ends_with('\n') {
        out.pop();
    }
    out.push('\n');
    out
}

/// 表ごとに `=` を揃える（値の行のキーの幅の最大に合わせる）。
pub fn align_table(table: &mut Table) {
    let width = table
        .iter()
        .filter(|(_, item)| item.is_value())
        .filter_map(|(k, _)| table.key(k).map(|key| key.display_repr().len()))
        .max()
        .unwrap_or(0);
    let keys: Vec<String> = table.iter().map(|(k, _)| k.to_string()).collect();
    for k in &keys {
        let is_value = table.get(k).is_some_and(Item::is_value);
        if let Some(mut key) = table.key_mut(k)
            && is_value
        {
            let len = key.display_repr().len();
            key.leaf_decor_mut().set_suffix(" ".repeat(width - len + 1));
        }
        if let Some(Item::Value(v)) = table.get_mut(k) {
            let suffix = v
                .decor()
                .suffix()
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            v.decor_mut().set_prefix(" ");
            v.decor_mut().set_suffix(suffix);
        }
    }
    for k in keys {
        match table.get_mut(&k) {
            Some(Item::Table(t)) => align_table(t),
            Some(Item::ArrayOfTables(a)) => a.iter_mut().for_each(align_table),
            _ => {}
        }
    }
}
