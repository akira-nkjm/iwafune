//! TOML の共通プロファイル処理（read・write・format から使う）。
//!
//! 出典の付いた値の 1 行 1 要素の形（docs/model.md §10.2 の 3）は [`sourced`]、名前の変わったキーの書き換えは [`renamed`]。
use crate::error::StoreError;
use toml_edit::{DocumentMut, Item, Key, Table};

pub mod renamed;
mod sourced;
pub use sourced::{MAX_HEADER_DEPTH, hide_empty_header, is_sourced, lay_out, spread_rows};
/// 整数のまま書くキー（ID・員数・回数・フェーズの並び順 `order`・日付の探索の格子の点の数）。
pub const INTEGER_KEYS: &[&str] = &[
    "count",
    "active",
    "cores",
    "schema",
    "orbits",
    "order",
    "departure_steps",
    "arrival_steps",
];
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

/// 文書の `=` を揃えて本文にする（新しいファイルの書き出しにも使う）。値の行の無い表の見出しは書かない。
pub fn finish(mut doc: DocumentMut) -> String {
    hide_empty_headers(doc.as_table_mut());
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

/// 入れ子の表すべてに [`hide_empty_header`] を当てる。
fn hide_empty_headers(table: &mut Table) {
    for (_, item) in table.iter_mut() {
        if let Item::Table(t) = item {
            hide_empty_headers(t);
            hide_empty_header(t);
        }
    }
}

/// 値の行のキーの表示の長さ（ドット付きキーは区切りの `.` も含めた全体）。
pub fn row_key_len(path: &[&Key]) -> usize {
    path.iter().map(|k| k.display_repr().len()).sum::<usize>() + path.len().saturating_sub(1)
}

/// 表の値の行（ドット付きキーの行も含む）の `=` を揃える。入れ子の見出しの表は揃えない。
pub fn align_rows(table: &mut Table) {
    let width = table
        .get_values()
        .iter()
        .map(|(path, _)| row_key_len(path))
        .max()
        .unwrap_or(0);
    pad_rows(table, 0, width);
}

/// `offset` はドット付きキーの親までの長さ。
fn pad_rows(table: &mut Table, offset: usize, width: usize) {
    let keys: Vec<String> = table.iter().map(|(k, _)| k.to_string()).collect();
    for k in &keys {
        let len = table.key(k).map_or(0, |key| key.display_repr().len());
        match table.get_mut(k) {
            Some(Item::Value(v)) => {
                let suffix = v
                    .decor()
                    .suffix()
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();
                v.decor_mut().set_prefix(" ");
                v.decor_mut().set_suffix(suffix);
                if let Some(mut key) = table.key_mut(k) {
                    let pad = width.saturating_sub(offset + len) + 1;
                    key.leaf_decor_mut().set_suffix(" ".repeat(pad));
                }
            }
            Some(Item::Table(t)) if t.is_dotted() => {
                pad_rows(t, offset + len + 1, width);
                if let Some(mut key) = table.key_mut(k) {
                    key.dotted_decor_mut().clear();
                }
            }
            _ => {}
        }
    }
}

/// 表ごとに `=` を揃える（値の行のキーの幅の最大に合わせる。入れ子の見出しの表も表ごとに）。
pub fn align_table(table: &mut Table) {
    align_rows(table);
    for (_, item) in table.iter_mut() {
        match item {
            Item::Table(t) if !t.is_dotted() => align_table(t),
            Item::ArrayOfTables(a) => a.iter_mut().for_each(align_table),
            _ => {}
        }
    }
}
