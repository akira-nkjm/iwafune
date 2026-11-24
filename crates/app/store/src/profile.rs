//! TOML の共通プロファイル処理（read・write・format から使う）。
//!
//! 出典の付いた値の 1 行 1 要素の形（docs/model.md §10.2 の 3）は [`sourced`]、名前の変わったキーの書き換えは [`renamed`]。
use crate::error::StoreError;
use toml_edit::{DocumentMut, Item, Key, Table};

pub mod renamed;
mod sourced;
pub use sourced::{MAX_HEADER_DEPTH, hide_empty_header, is_sourced, lay_out, spread_rows};
/// 整数のまま書くキー（ID・員数・回数・フェーズの並び順 `order`）。
pub const INTEGER_KEYS: &[&str] = &["count", "active", "cores", "schema", "orbits", "order"];
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

/// 値の行の並び（ドット付きキーは全体のパス）。キーの増減・名前の変更を見分ける印。
fn row_signature(table: &Table) -> Vec<String> {
    table
        .get_values()
        .iter()
        .map(|(path, _)| {
            path.iter()
                .map(|k| k.display_repr().to_string())
                .collect::<Vec<_>>()
                .join(".")
        })
        .collect()
}

/// 編集の前後を比べ、値の行のキーが変わった表と、新しく足した表だけ `=` を揃え直す（正規形の揃えは [`align_rows`] が唯一の定義）。
/// キーを消す・名前を変える編集で残りの行の揃えが古い幅のまま残ること、足した表が揃わないまま書かれることを防ぐ。
/// 変わっていない表は触らない（手で書いたコメントや装飾を保つ）。
pub fn realign_changed(before: &Table, after: &mut Table) {
    if row_signature(before) != row_signature(after) {
        align_rows(after);
    }
    for (key, item) in after.iter_mut() {
        let old = before.get(&key);
        match item {
            Item::Table(t) if !t.is_dotted() => match old {
                Some(Item::Table(o)) => realign_changed(o, t),
                // 編集で新しく足した表: 比べる相手が無いので、まるごと揃える
                _ => align_table(t),
            },
            Item::ArrayOfTables(a) => {
                let old_rows: Vec<&Table> = match old {
                    Some(Item::ArrayOfTables(o)) => o.iter().collect(),
                    _ => Vec::new(),
                };
                for (i, t) in a.iter_mut().enumerate() {
                    match old_rows.get(i) {
                        Some(o) => realign_changed(o, t),
                        // 新しく足した行（表）も、まるごと揃える
                        None => align_table(t),
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(text: &str) -> DocumentMut {
        parse("t.toml", text).unwrap()
    }

    #[test]
    fn realign_changed_fixes_only_tables_whose_keys_changed() {
        // 長いキーを消した表だけ揃え直す。キーが変わらない表（手で揃えを崩した行）とコメントは触らない
        let src = "[a]\nlong_key = 1.0  # 注\nk        = 2.0\n\n[b]\nx   = 1.0\nyyyy = 2.0\n";
        let before = doc(src);
        let mut after = before.clone();
        after["a"].as_table_mut().unwrap().remove("long_key");
        realign_changed(before.as_table(), after.as_table_mut());
        assert_eq!(
            after.to_string(),
            "[a]\nk = 2.0\n\n[b]\nx   = 1.0\nyyyy = 2.0\n"
        );
    }

    #[test]
    fn realign_changed_leaves_an_unchanged_document_alone() {
        let src = "[a]\nx   = 1.0\nyyyy = 2.0  # 注\n";
        let before = doc(src);
        let mut after = before.clone();
        realign_changed(before.as_table(), after.as_table_mut());
        assert_eq!(after.to_string(), src);
    }

    #[test]
    fn realign_changed_follows_rows_of_arrays_of_tables() {
        let before = doc("[[s]]\nlonger = 1.0\nk      = 2.0\n");
        let mut after = before.clone();
        after["s"]
            .as_array_of_tables_mut()
            .unwrap()
            .get_mut(0)
            .unwrap()
            .remove("longer");
        realign_changed(before.as_table(), after.as_table_mut());
        assert_eq!(after.to_string(), "[[s]]\nk = 2.0\n");
    }

    #[test]
    fn realign_changed_aligns_tables_and_rows_the_edit_added() {
        // 新しく足した表（入れ子も）と、表の配列に足した行は、比べる相手が無くても揃える。元からある表は触らない
        let before = doc("[a]\nx   = 1.0\nyyyy = 2.0\n\n[[s]]\nk = 1.0\n");
        let mut after = before.clone();
        let added = doc("[b.c]\nlong_key = 1.0\nk = 2.0\n\n[[s]]\nlonger = 1.0\nk = 2.0\n");
        after["b"] = added["b"].clone();
        let row = added["s"]
            .as_array_of_tables()
            .unwrap()
            .get(0)
            .unwrap()
            .clone();
        after["s"].as_array_of_tables_mut().unwrap().push(row);
        realign_changed(before.as_table(), after.as_table_mut());
        let text = after.to_string();
        // 表が並ぶ順は見ない（揃えだけを確かめる）
        assert!(
            text.contains("[b.c]\nlong_key = 1.0\nk        = 2.0\n"),
            "{text}"
        );
        assert!(
            text.contains("[[s]]\nlonger = 1.0\nk      = 2.0\n"),
            "{text}"
        );
        assert!(text.contains("[a]\nx   = 1.0\nyyyy = 2.0\n"), "{text}");
        assert!(text.contains("[[s]]\nk = 1.0\n"), "{text}");
    }
}
