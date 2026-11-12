//! 正規形の整形（`fune fmt`）。docs/model.md §10.2 の TOML プロファイルに合わせる。
//!
//! - 物理量の数値は float（`0` → `0.0`）。整数のままにするのは [`INTEGER_KEYS`] だけ
//! - インライン表は 1 行（TOML 1.1 の複数行のインライン表は 1.0 の書き方に直す）。文字列は基本形の引用
//! - ID をキーにする表（[`SORTED_TABLES`]）は ID 順
//! - 表ごとに `=` を揃える。コメントは残す。末尾は改行 1 つ

use crate::error::StoreError;
use crate::fileset::FileSet;
pub use crate::profile::{
    INTEGER_KEYS, VIEW_INTEGER_KEYS, align_table, finish, integer_keys, parse,
};
use crate::write::FileChange;
use toml_edit::{Item, Table, Value};

/// ID をキーにする表（行を ID 順に並べる）。
pub const SORTED_TABLES: &[&str] = &["states", "eclipse", "alloc", "mode_share", "nodes"];

/// 本文を正規形に整える。
pub fn format_text(path: &str, text: &str) -> Result<String, StoreError> {
    let mut doc = parse(path, text)?;
    normalize_table(doc.as_table_mut(), "", integer_keys(path));
    Ok(finish(doc))
}

/// 集合のうち整形で変わる TOML ファイルの変更。
pub fn format_files(files: &FileSet) -> Vec<FileChange> {
    files
        .iter()
        .filter(|(p, _)| p.ends_with(".toml"))
        .filter_map(|(path, text)| {
            let after = format_text(path, text).ok()?;
            (after != text).then(|| FileChange {
                path: path.to_string(),
                before: Some(text.to_string()),
                after: Some(after),
            })
        })
        .collect()
}

fn normalize_table(table: &mut Table, key: &str, ints: &[&str]) {
    if SORTED_TABLES.contains(&key) {
        table.sort_values();
    }
    let keys: Vec<String> = table.iter().map(|(k, _)| k.to_string()).collect();
    for k in keys {
        let Some(item) = table.get_mut(&k) else {
            continue;
        };
        match item {
            Item::Table(t) => normalize_table(t, &k, ints),
            Item::ArrayOfTables(a) => {
                for t in a.iter_mut() {
                    normalize_table(t, &k, ints);
                }
            }
            Item::Value(v) => normalize_value(v, &k, ints),
            Item::None => {}
        }
    }
}

fn normalize_value(value: &mut Value, key: &str, ints: &[&str]) {
    let decor = value.decor().clone();
    match value {
        Value::Integer(i) if !ints.contains(&key) => {
            #[allow(clippy::cast_precision_loss)]
            let f = *i.value() as f64;
            *value = Value::from(f);
        }
        Value::String(s) => {
            let plain = s.value().clone();
            *value = Value::from(plain);
        }
        Value::Array(a) => {
            for v in a.iter_mut() {
                normalize_value(v, key, ints);
                v.decor_mut().clear();
            }
            a.fmt();
        }
        Value::InlineTable(t) => {
            let keys: Vec<String> = t.iter().map(|(k, _)| k.to_string()).collect();
            for k in keys {
                if let Some(v) = t.get_mut(&k) {
                    normalize_value(v, &k, ints);
                }
            }
            t.fmt();
        }
        _ => {}
    }
    *value.decor_mut() = decor;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_numbers_inline_tables_and_alignment() {
        let src = "count = 2\nmass = { value = 1, unit = 'kg' }  # 実測\n\n[states]\nZ = { state = \"on\" }\nA = { state = \"off\", duty = { on = 0 } }\n";
        let out = format_text("t.toml", src).unwrap();
        assert_eq!(
            out,
            "count = 2\nmass  = { value = 1.0, unit = \"kg\" }  # 実測\n\n[states]\nA = { state = \"off\", duty = { on = 0.0 } }\nZ = { state = \"on\" }\n"
        );
        assert_eq!(format_text("t.toml", &out).unwrap(), out);
    }

    #[test]
    fn view_coordinates_and_edge_points_stay_integers() {
        // 図の座標と線の折れ点は画面の整数座標（読み込みの型も整数なので、小数にすると読めなくなる）
        let src = "[edge]\n\"P5V:PCDU.OUT2>CAM.PWR\" = { points = [[408, 507], [468, 507]], target_side = \"top\", target_offset = 40 }\n\n[nodes]\nRW_X = { x = 420, y = 120 }\n";
        assert_eq!(format_text("view/electrical.toml", src).unwrap(), src);
    }
}
