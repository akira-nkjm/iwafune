//! 出典の付いた値の形（docs/model.md §10.2 の 3）。**この形を知っているのはここだけ**。
//!
//! 出典（`from` の表）の付いた値は、インライン表の 1 行ではなく 1 行 1 要素で書く。値を置く表の見出しが
//! [`MAX_HEADER_DEPTH`] 段に収まれば値の見出しの表（`[attr.mass]`）、収まらなければドット付きキー
//! （`[attr.power]` の下の `on.value = …`）。どちらでも `from` はドット付きキー（`from.source = …`）。
//! 出典の無い値は 1 行のインライン表のまま。形は行の長さではなく出典の有無だけで決める。
//!
//! 書き出し（codec・records）は [`lay_out`]、整形（`fune fmt`）は [`spread_rows`] を通す。

use toml_edit::{InlineTable, Item, RawString, Table, Value};

/// 見出しの表の段の上限（§10.2 の 2「入れ子は 2 段まで」）。
pub const MAX_HEADER_DEPTH: usize = 2;

/// 出典（`from` の表）の付いたインライン表か。
pub fn is_sourced(value: &Value) -> bool {
    value.as_inline_table().is_some_and(inline_sourced)
}

fn inline_sourced(table: &InlineTable) -> bool {
    table.get("from").is_some_and(Value::is_inline_table)
}

/// 出典の付いた値をすでに表で書いているか（見出しの表・ドット付きキーのどちらでも）。
fn is_sourced_table(table: &Table) -> bool {
    table
        .get("from")
        .is_some_and(|f| f.is_table() || f.is_inline_table())
}

/// 深さ `depth` の表の行を見出しの表にできないか（ドット付きキーで書くか）。ルートの深さは 0。
fn dotted_at(depth: usize) -> bool {
    depth + 1 > MAX_HEADER_DEPTH
}

/// 値の行 → ファイルの項目。出典の付いたインライン表は 1 行 1 要素の表にし、それ以外はそのまま値の行。
/// `depth` は値を置く表の見出しの段数（`[attr]` の行なら 1）。
pub fn lay_out(value: Value, depth: usize) -> Item {
    match value {
        Value::InlineTable(t) if inline_sourced(&t) => {
            let mut table = rows(t, dotted_at(depth));
            if !table.is_dotted() {
                crate::profile::align_rows(&mut table);
            }
            Item::Table(table)
        }
        other => Item::Value(other),
    }
}

/// インライン表 → 1 行 1 要素の表（中のインライン表はドット付きキー）。装飾は捨てる。
fn rows(inline: InlineTable, dotted: bool) -> Table {
    let mut table = Table::new();
    for (key, value) in inline {
        let item = match value {
            Value::InlineTable(t) => Item::Table(rows(t, true)),
            mut v => {
                v.decor_mut().clear();
                Item::Value(v)
            }
        };
        table.insert(&key, item);
    }
    table.set_dotted(dotted);
    table
}

/// 値の行の無い表の見出しを書かない（`[attr]` の行がすべて `[attr.<名前>]` になったときなど）。
pub fn hide_empty_header(table: &mut Table) {
    let has_tables = table.iter().any(|(_, item)| item.is_table());
    if !table.is_dotted() && has_tables && table.get_values().is_empty() {
        table.set_implicit(true);
    }
}

/// 表の行のうち出典の付いた値を §10.2 の形に直す（`fune fmt`）。`depth` は `table` の見出しの段数。
/// 古い形（インライン表の 1 行）も、見出しの表とドット付きキーの取り違えも直す。行の上のコメントと行末のコメントは残す。
pub fn spread_rows(table: &mut Table, depth: usize) {
    let dotted = dotted_at(depth);
    let keys: Vec<String> = table.iter().map(|(k, _)| k.to_string()).collect();
    for k in &keys {
        let switched = match table.get_mut(k) {
            Some(Item::Value(v)) if is_sourced(v) => {
                spread_value(table, k, dotted);
                continue;
            }
            Some(Item::Table(t)) if is_sourced_table(t) => {
                let switched = t.is_dotted() != dotted;
                retype(t, dotted);
                switched
            }
            _ => false,
        };
        if switched && let Some(mut key) = table.key_mut(k) {
            key.leaf_decor_mut().clear();
            key.dotted_decor_mut().clear();
        }
    }
    hide_empty_header(table);
}

/// すでに表で書いた出典付きの値を、見出しの表かドット付きキーに揃える（中のインライン表もドット付きキーに）。
fn retype(table: &mut Table, dotted: bool) {
    table.set_dotted(dotted);
    let keys: Vec<String> = table.iter().map(|(k, _)| k.to_string()).collect();
    for k in &keys {
        if let Some(item) = table.get_mut(k)
            && let Item::Value(Value::InlineTable(t)) = item
        {
            *item = Item::Table(rows(std::mem::take(t), true));
        }
    }
}

/// 行の上のコメント（キーの前の装飾）と行末のコメント（値の後ろの装飾）。
fn comments(table: &Table, key: &str) -> (Option<String>, Option<String>) {
    let text = |raw: Option<&RawString>| {
        raw.and_then(RawString::as_str)
            .filter(|s| s.contains('#'))
            .map(str::to_string)
    };
    let above = table.key(key).and_then(|k| text(k.leaf_decor().prefix()));
    let after = match table.get(key) {
        Some(Item::Value(v)) => text(v.decor().suffix()),
        _ => None,
    };
    (above, after)
}

/// インライン表の 1 行を 1 行 1 要素にする（キーの位置は変えない）。
fn spread_value(table: &mut Table, key: &str, dotted: bool) {
    let (above, after) = comments(table, key);
    let Some(Item::Value(Value::InlineTable(inline))) = table.get_mut(key).map(std::mem::take)
    else {
        return;
    };
    let mut spread = rows(inline, dotted);
    if dotted {
        keep_comments_on_first_row(&mut spread, above, after);
    } else {
        crate::profile::align_rows(&mut spread);
        if let Some(above) = above {
            spread
                .decor_mut()
                .set_prefix(format!("\n{}", above.trim_start_matches('\n')));
        }
        if let Some(after) = after {
            spread.decor_mut().set_suffix(after);
        }
    }
    // 既にあるキーへの insert は位置を保ち、キーの装飾（揃えの空白）を消す。
    table.insert(key, Item::Table(spread));
}

/// ドット付きキーの行では、コメントを最初の行に付ける。
fn keep_comments_on_first_row(table: &mut Table, above: Option<String>, after: Option<String>) {
    let Some(first) = table.iter().next().map(|(k, _)| k.to_string()) else {
        return;
    };
    if let Some(above) = above
        && let Some(mut key) = table.key_mut(&first)
    {
        key.leaf_decor_mut().set_prefix(above);
    }
    if let Some(after) = after
        && let Some(Item::Value(v)) = table.get_mut(&first)
    {
        v.decor_mut().set_suffix(after);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::{finish, parse};

    fn spread(text: &str) -> String {
        let mut doc = parse("t.toml", text).unwrap();
        spread_all(doc.as_table_mut(), 0);
        finish(doc)
    }

    fn spread_all(table: &mut Table, depth: usize) {
        spread_rows(table, depth);
        for (_, item) in table.iter_mut() {
            if let Item::Table(t) = item
                && !t.is_dotted()
            {
                spread_all(t, depth + 1);
            }
        }
    }

    #[test]
    fn sourced_attributes_get_their_own_table() {
        let src = "[attr]\nmass = { value = 115.0, unit = \"g\", from = { source = \"ds_example\", note = \"Mass [g]: 115.\" } }\nsize = { x = 1.0, y = 2.0, z = 3.0, unit = \"mm\" }\n";
        let want = "[attr]\nsize = { x = 1.0, y = 2.0, z = 3.0, unit = \"mm\" }\n\n[attr.mass]\nvalue       = 115.0\nunit        = \"g\"\nfrom.source = \"ds_example\"\nfrom.note   = \"Mass [g]: 115.\"\n";
        assert_eq!(spread(src), want);
        assert_eq!(spread(want), want);
    }

    #[test]
    fn sourced_state_cells_use_dotted_keys() {
        let src = "[attr.power]\noff = { value = 0.0, unit = \"W\" }\non  = { value = 1.0, unit = \"W\", from = { source = \"x\" } }  # 見積\n";
        let want = "[attr.power]\noff            = { value = 0.0, unit = \"W\" }\non.value       = 1.0  # 見積\non.unit        = \"W\"\non.from.source = \"x\"\n";
        assert_eq!(spread(src), want);
        assert_eq!(spread(want), want);
    }

    #[test]
    fn an_attribute_table_without_rows_has_no_header_and_keeps_comments() {
        let src = "kind = \"rw\"\n\n[attr]\n# EM の実測\nmass = { value = 0.152, unit = \"kg\", from = { source = \"em\" } }  # 2026-10\n";
        let want = "kind = \"rw\"\n\n# EM の実測\n[attr.mass]  # 2026-10\nvalue       = 0.152\nunit        = \"kg\"\nfrom.source = \"em\"\n";
        assert_eq!(spread(src), want);
        assert_eq!(spread(want), want);
    }

    #[test]
    fn mixed_and_misplaced_forms_are_normalized() {
        // 見出しの表の中の `from` のインライン表、ドット付きキーで書いた属性
        let src = "[attr]\nmass.value = 1.0\nmass.unit = \"kg\"\nmass.from = { source = \"x\" }\n";
        let want = "[attr.mass]\nvalue       = 1.0\nunit        = \"kg\"\nfrom.source = \"x\"\n";
        assert_eq!(spread(src), want);
    }

    #[test]
    fn values_without_provenance_stay_on_one_line() {
        let src = "[attr]\nmass = { value = 1.0, unit = \"kg\" }\n";
        assert_eq!(spread(src), src);
        let v: Value = "{ value = 1.0, unit = \"kg\" }".parse().unwrap();
        assert!(matches!(lay_out(v, 1), Item::Value(_)));
    }
}
