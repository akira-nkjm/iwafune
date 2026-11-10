//! モード・ネット・定義・設定のファイルの編集と、新しいファイルの正規形。

use super::Edits;
use super::doc::{self, set_opt, string, strings, table_mut};
use crate::codec::{encode_vector, float};
use crate::error::StoreError;
use crate::profile as format;
use fune_model::design::{
    AttributeDef, KindDef, Mode, Net, NetField, NetFieldName, StateAssign, ViewNode,
};
use fune_model::path::PortRef;
use fune_model::value::{Quantity, SettingValue};
use serde::Serialize;
use toml_edit::{DocumentMut, InlineTable, Item, Table, Value};

/// 量（`{ value, unit, from }`）。
pub fn quantity(q: &Quantity) -> Item {
    let mut t = InlineTable::new();
    t.insert("value", float(q.value));
    t.insert("unit", q.unit.as_str().into());
    if let Some(from) = &q.from {
        let mut f = InlineTable::new();
        if let Some(s) = &from.source {
            f.insert("source", s.as_str().into());
        }
        if let Some(n) = &from.note {
            f.insert("note", n.as_str().into());
        }
        t.insert("from", Value::InlineTable(f));
    }
    Item::Value(Value::InlineTable(t))
}

/// モードの 1 行（`{ state = "rx", duty = { tx = 0.1 } }`）。
fn state_row(assign: &StateAssign) -> Item {
    let mut t = InlineTable::new();
    t.insert("state", assign.state.as_str().into());
    if !assign.duty.is_empty() {
        let mut duty = InlineTable::new();
        for (s, d) in &assign.duty {
            duty.insert(s, float(*d));
        }
        t.insert("duty", Value::InlineTable(duty));
    }
    Item::Value(Value::InlineTable(t))
}

fn is_sorted(table: &Table) -> bool {
    let keys: Vec<&str> = table.iter().map(|(k, _)| k).collect();
    keys.windows(2).all(|w| w[0] <= w[1])
}

/// 新しいモードのファイル。
pub fn new_mode_text(mode: &Mode) -> String {
    let mut doc = DocumentMut::new();
    if let Some(l) = &mode.label {
        doc.insert("label", string(l));
    }
    if let Some(d) = &mode.description {
        doc.insert("description", string(d));
    }
    let mut states = Table::new();
    for (part, assign) in &mode.states {
        states.insert(part.as_str(), state_row(assign));
    }
    doc.insert("states", Item::Table(states));
    format::finish(doc)
}

/// モードの表の 1 行を書く（None なら消す）。行は ID 順を保つ。
pub fn set_mode_row(
    edits: &mut Edits,
    file: &str,
    part: &str,
    assign: Option<&StateAssign>,
) -> Result<(), StoreError> {
    edits.edit(file, |doc| {
        let table = table_mut(doc, &["states"])?;
        let sorted = is_sorted(table);
        let is_new = !table.contains_key(part);
        set_opt(table, part, assign.map(state_row));
        if sorted && is_new {
            table.sort_values();
        }
        Ok(())
    })
}

/// ポートの参照（`"PCDU.OUT3"`。書き方は `PortRef` の `Display` だけ）。
fn port(p: &PortRef) -> Item {
    string(&p.to_string())
}

/// 型付きのフィールド → ファイルの値（None なら書かない）。新しいファイル・丸ごとの書き込み・1 フィールドの書き込みで共通。
fn net_field_item(field: &NetField) -> Option<Item> {
    match field {
        NetField::Kind(k) => Some(string(k.as_str())),
        NetField::Label(v) | NetField::Protocol(v) => v.as_deref().map(string),
        NetField::Source(p) | NetField::Master(p) => p.as_ref().map(port),
        NetField::Rail(q) | NetField::Limit(q) | NetField::Bitrate(q) => q.as_ref().map(quantity),
    }
}

/// ネットの見出しのキー（正規の順は `NetFieldName` の宣言の順）。
fn net_items(net: &Net) -> Vec<(&'static str, Option<Item>)> {
    NetFieldName::ALL
        .iter()
        .map(|&key| (key.as_str(), net_field_item(&NetField::of(net, key))))
        .collect()
}

/// 新しいネットのファイル。
pub fn new_net_text(net: &Net) -> String {
    let mut doc = DocumentMut::new();
    for (key, item) in net_items(net) {
        if let Some(item) = item {
            doc.insert(key, item);
        }
    }
    format::finish(doc)
}

/// ネットのファイルを `net` に合わせる（同じ値の行は触らない）。
pub fn write_net(edits: &mut Edits, file: &str, net: &Net) -> Result<(), StoreError> {
    edits.edit(file, |doc| {
        for (key, item) in net_items(net) {
            set_opt(doc.as_table_mut(), key, item);
        }
        Ok(())
    })
}

/// ネットのフィールドを 1 つ書く（None なら消す）。
pub fn set_net_field(edits: &mut Edits, file: &str, field: &NetField) -> Result<(), StoreError> {
    let item = net_field_item(field);
    edits.edit(file, |doc| {
        set_opt(doc.as_table_mut(), field.name(), item);
        Ok(())
    })
}

/// serde の形をそのまま TOML にする（`id` はファイル名で決まるので書かない）。
pub(super) fn serde_text<T: Serialize>(path: &str, value: &T) -> Result<String, StoreError> {
    let text = toml::to_string(value).map_err(|e| StoreError::edit(path, e.to_string()))?;
    let mut doc = doc::parse(path, &text)?;
    doc.as_table_mut().remove("id");
    Ok(format::finish(doc))
}

/// 新しい属性定義のファイル。
pub fn new_attribute_text(path: &str, def: &AttributeDef) -> Result<String, StoreError> {
    serde_text(path, def)
}

/// 新しい種別定義のファイル。
pub fn new_kind_text(path: &str, def: &KindDef) -> Result<String, StoreError> {
    serde_text(path, def)
}

/// 設定の値 → TOML の値。
pub fn setting_item(value: &SettingValue) -> Item {
    match value {
        SettingValue::Number(n) => Item::Value(float(*n)),
        SettingValue::Integer(i) => Item::Value((*i).into()),
        SettingValue::Bool(b) => Item::Value((*b).into()),
        SettingValue::Text(s) => string(s),
        SettingValue::Quantity(q) => quantity(q),
        SettingValue::Vector(v) => Item::Value(encode_vector(v, None)),
        SettingValue::List(l) => strings(l.iter().map(String::as_str)),
    }
}

/// 設定ファイルの `keys` の位置に値を書く（None なら消す）。ファイルが無ければ作る。
/// 途中がインライン表（`cp_offset_aero = { x, … }`）ならその中を書き換える。
pub fn set_setting(
    edits: &mut Edits,
    file: &str,
    keys: &[String],
    value: Option<&SettingValue>,
) -> Result<(), StoreError> {
    let Some((last, parents)) = keys.split_last() else {
        return Err(StoreError::edit(file, "キーが空です"));
    };
    let item = value.map(setting_item);
    edits.edit_or_create(file, |doc| {
        let mut table: &mut Table = doc.as_table_mut();
        for (i, key) in parents.iter().enumerate() {
            if let Some(Item::Value(Value::InlineTable(inline))) = table.get_mut(key) {
                let rest = &keys[i + 1..];
                return set_in_inline(inline, rest, item).map_err(|m| StoreError::edit(file, m));
            }
            let entry = table
                .entry(key)
                .or_insert_with(|| Item::Table(Table::new()));
            table = entry
                .as_table_mut()
                .ok_or_else(|| StoreError::edit(file, format!("`{key}` は表ではありません")))?;
        }
        set_opt(table, last, item);
        let path: Vec<&str> = parents.iter().map(String::as_str).collect();
        doc::drop_if_empty(doc, &path);
        Ok(())
    })
}

fn set_in_inline(
    inline: &mut InlineTable,
    keys: &[String],
    item: Option<Item>,
) -> Result<(), String> {
    let Some((first, rest)) = keys.split_first() else {
        return Err("キーが空です".into());
    };
    if rest.is_empty() {
        match item.and_then(|i| i.into_value().ok()) {
            Some(v) => {
                inline.insert(first, v);
            }
            None => {
                inline.remove(first);
            }
        }
        return Ok(());
    }
    match inline.get_mut(first) {
        Some(Value::InlineTable(t)) => set_in_inline(t, rest, item),
        _ => Err(format!("`{first}` はインライン表ではありません")),
    }
}

/// 図の配置の 1 行（`{ x = 420, y = 120, w = 160, h = 60 }`。整数）。
fn view_row(node: &ViewNode) -> Item {
    let mut t = InlineTable::new();
    t.insert("x", i64::from(node.x).into());
    t.insert("y", i64::from(node.y).into());
    if let Some(w) = node.w {
        t.insert("w", i64::from(w).into());
    }
    if let Some(h) = node.h {
        t.insert("h", i64::from(h).into());
    }
    Item::Value(Value::InlineTable(t))
}

/// 図の配置を書く（`view/<名前>.toml` の `[nodes]`。挙げたノードだけを書き換え、`remove` は消す）。
/// 行は ID 順。ファイルが無ければ作る。消して空になった `[nodes]` は表ごと消す（元に戻したとき、書く前と同じ形にする）。
pub fn set_view_nodes(
    edits: &mut Edits,
    file: &str,
    nodes: &[(String, ViewNode)],
    remove: &[String],
) -> Result<(), StoreError> {
    edits.edit_or_create(file, |doc| {
        let table = table_mut(doc, &["nodes"])?;
        let sorted = is_sorted(table);
        let mut added = false;
        for (id, node) in nodes {
            added |= !table.contains_key(id);
            set_opt(table, id, Some(view_row(node)));
        }
        for id in remove {
            table.remove(id);
        }
        if sorted && added {
            table.sort_values();
        }
        doc::drop_if_empty(doc, &["nodes"]);
        Ok(())
    })
}

/// 手で引いた線の経路を `[edge]` に書く（TOML 1.0 のインライン表と配列）。None なら消し、空になった `[edge]` は表ごと消す。
pub fn set_view_edge(
    edits: &mut Edits,
    file: &str,
    key: &str,
    edge: Option<&fune_model::design::ViewEdge>,
) -> Result<(), StoreError> {
    edits.edit_or_create(file, |doc| {
        let table = table_mut(doc, &["edge"])?;
        let row = edge.map(|e| {
            let mut t = InlineTable::new();
            let mut points = toml_edit::Array::new();
            for p in &e.points {
                let mut point = toml_edit::Array::new();
                point.push(i64::from(p[0]));
                point.push(i64::from(p[1]));
                points.push(point);
            }
            t.insert("points", points.into());
            for (key, side) in [
                ("source_side", e.source_side),
                ("target_side", e.target_side),
            ] {
                if let Some(side) = side {
                    t.insert(key, side.as_str().into());
                }
            }
            for (key, offset) in [
                ("source_offset", e.source_offset),
                ("target_offset", e.target_offset),
            ] {
                if let Some(offset) = offset {
                    t.insert(key, i64::from(offset).into());
                }
            }
            Item::Value(Value::InlineTable(t))
        });
        set_opt(table, key, row);
        table.sort_values();
        doc::drop_if_empty(doc, &["edge"]);
        Ok(())
    })
}

/// 全ノード・辺の表示上書きを消す（未知のメタデータは保持する）。
pub fn reset_view(edits: &mut Edits, file: &str) -> Result<(), StoreError> {
    edits.edit_or_create(file, |doc| {
        doc.remove("nodes");
        doc.remove("edge");
        Ok(())
    })
}
