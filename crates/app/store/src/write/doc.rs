//! toml_edit の小さな道具。既存の行の装飾（空白・コメント）を保ち、新しい行は周りの揃え方に合わせる。

use crate::error::StoreError;
pub use crate::profile::parse;
use crate::profile::row_key_len;
use toml_edit::{DocumentMut, Item, Key, Table, Value};

/// 表の中の `=` の揃え幅（すべての値の行が同じ幅で揃っていればその幅）。
/// ドット付きキーの行（出典の付いた値）も全体の長さで数える。
fn aligned_width(table: &Table) -> Option<usize> {
    let mut width = None;
    for (path, _) in table.get_values() {
        let key = path.last()?;
        let suffix = key
            .leaf_decor()
            .suffix()
            .and_then(|s| s.as_str())
            .unwrap_or(" ");
        let w = row_key_len(&path) + suffix.len();
        match width {
            None => width = Some(w),
            Some(prev) if prev == w => {}
            Some(_) => return None,
        }
    }
    width
}

/// 新しいキーを、周りの行と `=` を揃えて足す。新しいキーが今の揃え幅より長ければ、表ごと揃え直す
/// （`[pose]` に `orientation` を足すときなど。揃えないと `fune fmt --check` に通らない）。
/// ドット付きキーの行の表（出典の付いた状態のセル）を足したときも表ごと揃え直す。
pub fn insert_aligned(table: &mut Table, key: &str, item: Item) {
    let width = aligned_width(table);
    let is_value = item.is_value();
    let is_dotted = item.as_table().is_some_and(Table::is_dotted);
    table.insert(key, item);
    if is_dotted && width.is_some() {
        crate::profile::align_rows(table);
    }
    if !is_value {
        return;
    }
    let Some(width) = width else { return };
    let Some(len) = table.key(key).map(|k| k.display_repr().len()) else {
        return;
    };
    if len + 1 > width {
        crate::profile::align_table(table);
    } else if let Some(mut k) = table.key_mut(key) {
        k.leaf_decor_mut().set_suffix(" ".repeat(width - len));
    }
}

/// 値を置き換える。既存の値の装飾（前後の空白・行末のコメント）は引き継ぐ。
/// 同じ値なら何もしない（差分を出さない）。
pub fn set(table: &mut Table, key: &str, item: Item) {
    match (table.get_mut(key), item) {
        (Some(Item::Value(old)), Item::Value(mut new)) => {
            if values_equal(old, &new) {
                return;
            }
            *new.decor_mut() = old.decor().clone();
            *old = new;
        }
        (Some(Item::Table(old)), Item::Table(new)) if old.is_dotted() == new.is_dotted() => {
            *old = new;
        }
        (Some(_), item) => replace_kind(table, key, item),
        (None, item) => insert_aligned(table, key, item),
    }
}

/// 行の種類を変える（値の行 ⇄ 見出しの表・ドット付きキー。出典を付けた・外したとき）。キーの位置は保ち、
/// 揃っていた表は揃え直し、値の行が無くなった表の見出しは書かない。
fn replace_kind(table: &mut Table, key: &str, item: Item) {
    let aligned = aligned_width(table).is_some();
    // 既にあるキーへの insert は位置を保ち、キーの装飾（揃えの空白）を消す。
    table.insert(key, item);
    if aligned {
        crate::profile::align_rows(table);
    }
    crate::profile::hide_empty_header(table);
}

/// 行を消す（揃っていた表は残りの行で揃え直し、値の行が無くなった表の見出しは書かない。fmt と同じ形にする）。消したら true。
pub fn remove_aligned(table: &mut Table, key: &str) -> bool {
    let aligned = aligned_width(table).is_some();
    if table.remove(key).is_none() {
        return false;
    }
    if aligned {
        crate::profile::align_rows(table);
    }
    crate::profile::hide_empty_header(table);
    true
}

/// 値があれば置き換え、None なら消す。
pub fn set_opt(table: &mut Table, key: &str, item: Option<Item>) {
    match item {
        Some(item) => set(table, key, item),
        None => {
            table.remove(key);
        }
    }
}

/// 表（`[a.b]`）を確保する。途中の表は暗黙（見出しを書かない）にする。
pub fn table_mut<'a>(doc: &'a mut DocumentMut, path: &[&str]) -> Result<&'a mut Table, StoreError> {
    let mut table: &mut Table = doc.as_table_mut();
    for (i, key) in path.iter().enumerate() {
        let last = i + 1 == path.len();
        let entry = table.entry(key).or_insert_with(|| {
            let mut t = Table::new();
            t.set_implicit(!last);
            Item::Table(t)
        });
        if let Item::Value(Value::InlineTable(inline)) = entry {
            // インライン表は見出しの表に直す（書き足すため）。
            let t = std::mem::take(inline).into_table();
            *entry = Item::Table(t);
        }
        table = entry.as_table_mut().ok_or_else(|| StoreError::Edit {
            file: String::new(),
            message: format!("`{}` is not a table", path[..=i].join(".")),
        })?;
    }
    Ok(table)
}

/// 表があれば返す（無ければ None）。
pub fn table_opt<'a>(doc: &'a mut DocumentMut, path: &[&str]) -> Option<&'a mut Table> {
    let mut table: &mut Table = doc.as_table_mut();
    for key in path {
        table = table.get_mut(key)?.as_table_mut()?;
    }
    Some(table)
}

/// 中身が空になった表を消す（`[attr]` の最後の行を消したときなど）。
pub fn drop_if_empty(doc: &mut DocumentMut, path: &[&str]) {
    let Some((last, parents)) = path.split_last() else {
        return;
    };
    let mut table: &mut Table = doc.as_table_mut();
    for key in parents {
        match table.get_mut(key).and_then(Item::as_table_mut) {
            Some(t) => table = t,
            None => return,
        }
    }
    if table
        .get(last)
        .and_then(Item::as_table)
        .is_some_and(Table::is_empty)
    {
        table.remove(last);
    }
}

/// 見出しの表を正規の順（`order` に挙げた最上位のキーの順、同じキーの中は元の順）に並べ直す。
/// 新しく足した表（`[attr]` など）を正しい位置に置くため。既に正規の順なら何も変わらない。
pub fn order_tables(doc: &mut DocumentMut, order: &[&str]) {
    let rank = |key: &str| order.iter().position(|k| *k == key).unwrap_or(order.len());
    let mut entries: Vec<(usize, isize, Vec<String>)> = Vec::new();
    let top: Vec<String> = doc.as_table().iter().map(|(k, _)| k.to_string()).collect();
    for key in &top {
        if let Some(t) = doc.as_table().get(key).and_then(Item::as_table) {
            collect_tables(t, vec![key.clone()], rank(key), &mut entries);
        }
    }
    entries.sort_by_key(|(r, pos, _)| (*r, *pos));
    for (i, (_, _, path)) in entries.into_iter().enumerate() {
        let Some(table) = table_at(doc.as_table_mut(), &path) else {
            continue;
        };
        table.set_position(Some(isize::try_from(i + 1).unwrap_or(isize::MAX)));
    }
}

fn table_at<'a>(table: &'a mut Table, path: &[String]) -> Option<&'a mut Table> {
    match path.split_first() {
        None => Some(table),
        Some((first, rest)) => table_at(table.get_mut(first)?.as_table_mut()?, rest),
    }
}

fn collect_tables(
    table: &Table,
    path: Vec<String>,
    rank: usize,
    out: &mut Vec<(usize, isize, Vec<String>)>,
) {
    out.push((rank, table.position().unwrap_or(isize::MAX), path.clone()));
    for (key, item) in table.iter() {
        if let Some(t) = item.as_table() {
            let mut child = path.clone();
            child.push(key.to_string());
            collect_tables(t, child, rank, out);
        }
    }
}

/// 値の意味が等しいか（書き方の違い・装飾は無視）。
pub fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(x), Value::String(y)) => x.value() == y.value(),
        (Value::Integer(x), Value::Integer(y)) => x.value() == y.value(),
        (Value::Float(x), Value::Float(y)) => x.value() == y.value(),
        (Value::Boolean(x), Value::Boolean(y)) => x.value() == y.value(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(p, q)| values_equal(p, q))
        }
        (Value::InlineTable(x), Value::InlineTable(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| values_equal(v, w)))
        }
        _ => false,
    }
}

/// 文字列の値。
pub fn string(s: &str) -> Item {
    Item::Value(Value::from(s))
}

/// 文字列の配列。
pub fn strings<'a>(items: impl IntoIterator<Item = &'a str>) -> Item {
    let array: toml_edit::Array = items.into_iter().map(Value::from).collect();
    Item::Value(Value::Array(array))
}

/// 表のキーの名前を変える（行の装飾と `=` の位置を保つ）。変えたら true。
pub fn rename_key(table: &mut Table, from: &str, to: &str) -> bool {
    let sorted = {
        let keys: Vec<&str> = table.iter().map(|(k, _)| k).collect();
        keys.windows(2).all(|w| w[0] <= w[1])
    };
    let aligned = aligned_width(table).is_some();
    let Some((old, item)) = table.remove_entry(from) else {
        return false;
    };
    let mut key = Key::new(to);
    let mut decor = old.leaf_decor().clone();
    // 値の行（ドット付きキーも）だけ `=` の位置を保つ。見出しの表（`[attr.x]`）のキーには空白を足さない。
    let is_row = item.is_value() || item.as_table().is_some_and(Table::is_dotted);
    if is_row {
        let old_suffix = decor.suffix().and_then(|s| s.as_str()).map_or(1, str::len);
        let total = old.display_repr().len() + old_suffix;
        let new_len = key.display_repr().len();
        decor.set_suffix(" ".repeat(total.saturating_sub(new_len).max(1)));
    }
    *key.leaf_decor_mut() = decor;
    table.insert_formatted(&key, item);
    if sorted {
        table.sort_values();
    }
    // 揃っていた表は、新しいキーの長さで揃え直す（最も長いキーの名前を変えると幅が変わる。揃えないと `fune fmt --check` に通らない）
    if aligned {
        crate::profile::align_rows(table);
    }
    true
}

/// 文字列の配列の要素を置き換える（装飾を保つ）。
pub fn replace_in_array(item: &mut Item, from: &str, to: &str) {
    if let Some(array) = item.as_array_mut() {
        for v in array.iter_mut() {
            if v.as_str() == Some(from) {
                let decor = v.decor().clone();
                *v = Value::from(to);
                *v.decor_mut() = decor;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_keeps_comments_and_alignment() {
        let mut doc = parse("t", "a    = 1.0  # note\nbbbb = 2.0\n").unwrap();
        set(doc.as_table_mut(), "a", Item::Value(Value::from(3.0)));
        insert_aligned(doc.as_table_mut(), "cc", Item::Value(Value::from(4.0)));
        assert_eq!(
            doc.to_string(),
            "a    = 3.0  # note\nbbbb = 2.0\ncc   = 4.0\n"
        );
    }

    #[test]
    fn a_longer_new_key_realigns_the_whole_table() {
        // `fune part place` が `[pose]` に `orientation` を足すとき（揃えないと fmt --check に落ちる）
        let mut doc = parse("t", "position = 1.0\n").unwrap();
        insert_aligned(
            doc.as_table_mut(),
            "orientation",
            Item::Value(Value::from(2.0)),
        );
        assert_eq!(doc.to_string(), "position    = 1.0\norientation = 2.0\n");
    }

    #[test]
    fn dotted_rows_count_in_the_width_and_removal_realigns() {
        // 出典の付いたセル（`on.from.source`）の幅で揃えた表から、そのセルを消すと残りの行を揃え直す（fmt と同じ）。
        let src = "off            = 0.0\non.value       = 1.0\non.from.source = \"x\"\n";
        let mut doc = parse("t", src).unwrap();
        insert_aligned(doc.as_table_mut(), "idle", Item::Value(Value::from(0.5)));
        assert!(doc.to_string().ends_with("idle           = 0.5\n"), "{doc}");
        assert!(remove_aligned(doc.as_table_mut(), "on"));
        assert_eq!(doc.to_string(), "off  = 0.0\nidle = 0.5\n");
    }

    #[test]
    fn equal_values_are_left_untouched() {
        let mut doc = parse("t", "a = { value = 1.0, unit = \"kg\" }\n").unwrap();
        let mut t = toml_edit::InlineTable::new();
        t.insert("value", 1.0.into());
        t.insert("unit", "kg".into());
        set(doc.as_table_mut(), "a", Item::Value(Value::InlineTable(t)));
        assert_eq!(doc.to_string(), "a = { value = 1.0, unit = \"kg\" }\n");
    }
}
