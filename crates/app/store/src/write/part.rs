//! 部品ファイルの編集（属性・フィールド・取付・ポート）と、新しい部品ファイルの正規形。

use super::Edits;
use super::doc::{self, set, set_opt, string, strings, table_mut, table_opt};
use crate::codec::{encode_quaternion, encode_scalar, encode_value, encode_vector};
use crate::error::StoreError;
use crate::profile as format;
use fune_model::design::{Mount, Part, PartField, Pose};
use fune_model::value::{Authored, Scalar, Value};
use toml_edit::{DocumentMut, Item, Table};

/// `[mount]` の表。
fn mount_table(mount: &Mount) -> Table {
    let mut t = Table::new();
    t.insert("on", string(mount.on.as_str()));
    if let Some(point) = &mount.point {
        t.insert("point", string(point));
    }
    t
}

/// `[pose]`・`[mount_points.<名前>]` の表。
fn pose_table(pose: &Pose) -> Table {
    let mut t = Table::new();
    if let Some(p) = &pose.position {
        t.insert("position", Item::Value(encode_vector(p, None)));
    }
    if let Some(q) = &pose.orientation {
        t.insert("orientation", Item::Value(encode_quaternion(q)));
    }
    t
}

/// 表の中身を `desired` に合わせる（同じ値の行は触らない）。
pub fn sync_table(existing: &mut Table, desired: Table) {
    let keep: Vec<String> = desired.iter().map(|(k, _)| k.to_string()).collect();
    let stale: Vec<String> = existing
        .iter()
        .map(|(k, _)| k.to_string())
        .filter(|k| !keep.contains(k))
        .collect();
    for k in stale {
        existing.remove(&k);
    }
    for (k, v) in desired {
        match (existing.get_mut(&k), v) {
            (Some(Item::Table(old)), Item::Table(new)) => sync_table(old, new),
            (_, v) => set(existing, &k, v),
        }
    }
}

/// 部品ファイルの見出しの表の正規の順。
pub const PART_SECTIONS: &[&str] = &["mount", "pose", "mount_points", "attr", "ports"];

/// 部品ファイルを編集する（新しく足した表は正規の位置に置く）。
fn edit_part<T>(
    edits: &mut Edits,
    file: &str,
    f: impl FnOnce(&mut DocumentMut) -> Result<T, StoreError>,
) -> Result<T, StoreError> {
    edits.edit(file, |doc| {
        let out = f(doc)?;
        doc::order_tables(doc, PART_SECTIONS);
        Ok(out)
    })
}

/// 表を置き換える（無ければ作って `=` を揃える。`None` なら消す）。
fn sync_section(
    doc: &mut DocumentMut,
    key: &str,
    desired: Option<Table>,
) -> Result<(), StoreError> {
    match desired {
        Some(t) => {
            let fresh = !doc.contains_key(key);
            let table = table_mut(doc, &[key])?;
            sync_table(table, t);
            if fresh {
                format::align_table(table);
            }
        }
        None => {
            doc.as_table_mut().remove(key);
        }
    }
    Ok(())
}

/// 見出しのキー（正規の順: label → description → catalog → kind → candidates → count → maturity）。
fn insert_header(top: &mut Table, part: &Part) {
    let fields = [
        ("label", part.label.as_deref()),
        ("description", part.description.as_deref()),
        ("catalog", part.catalog.as_deref()),
        ("kind", part.kind.as_ref().map(|k| k.as_str())),
    ];
    for (key, value) in fields {
        if let Some(v) = value {
            top.insert(key, string(v));
        }
    }
    if !part.candidates.is_empty() {
        top.insert(
            "candidates",
            strings(part.candidates.iter().map(String::as_str)),
        );
    }
    if part.count != 1 {
        top.insert("count", Item::Value(i64::from(part.count).into()));
    }
    if let Some(m) = part.maturity {
        top.insert("maturity", string(m.as_str()));
    }
}

/// 新しい部品ファイルの本文（正規形: 見出しのキー → mount → pose → mount_points → attr → ports）。
pub fn new_part_text(part: &Part) -> String {
    let mut doc = DocumentMut::new();
    let top = doc.as_table_mut();
    insert_header(top, part);
    if let Some(m) = &part.mount {
        top.insert("mount", Item::Table(mount_table(m)));
    }
    if let Some(p) = &part.pose {
        top.insert("pose", Item::Table(pose_table(p)));
    }
    if !part.mount_points.is_empty() {
        let mut points = Table::new();
        points.set_implicit(true);
        for (name, pose) in &part.mount_points {
            points.insert(name, Item::Table(pose_table(pose)));
        }
        top.insert("mount_points", Item::Table(points));
    }
    if !part.attr.is_empty() {
        let mut attr = Table::new();
        for (id, value) in &part.attr {
            attr.insert(id.as_str(), encode_value(value));
        }
        top.insert("attr", Item::Table(attr));
    }
    if !part.ports.is_empty() {
        let mut ports = Table::new();
        ports.set_implicit(true);
        for (name, binding) in &part.ports {
            let mut t = Table::new();
            if let Some(net) = &binding.net {
                t.insert("net", string(net.as_str()));
            }
            ports.insert(name, Item::Table(t));
        }
        top.insert("ports", Item::Table(ports));
    }
    format::finish(doc)
}

/// 属性の値を丸ごと書く（状態ごとの値は状態ごとの行に合わせる）。
pub fn set_attr(
    edits: &mut Edits,
    file: &str,
    attr: &str,
    value: &Authored<Value>,
) -> Result<(), StoreError> {
    edit_part(edits, file, |doc| {
        let table = table_mut(doc, &["attr"])?;
        match (table.get_mut(attr), encode_value(value)) {
            (Some(Item::Table(old)), Item::Table(new)) => sync_table(old, new),
            (_, item) => set(table, attr, item),
        }
        Ok(())
    })
}

/// 状態ごとの値の 1 セルを書く（`[attr.<attr>]` の 1 行）。
pub fn set_attr_state(
    edits: &mut Edits,
    file: &str,
    attr: &str,
    state: &str,
    cell: &Authored<Scalar>,
) -> Result<(), StoreError> {
    edit_part(edits, file, |doc| {
        let table = table_mut(doc, &["attr", attr])?;
        set(
            table,
            state,
            Item::Value(encode_scalar(&cell.value, cell.from.as_ref())),
        );
        Ok(())
    })
}

/// 属性の上書きを消す（`state` があればそのセルだけ）。消したら true。
pub fn unset_attr(
    edits: &mut Edits,
    file: &str,
    attr: &str,
    state: Option<&str>,
) -> Result<bool, StoreError> {
    edit_part(edits, file, |doc| {
        let removed = match state {
            Some(state) => {
                table_opt(doc, &["attr", attr]).is_some_and(|t| t.remove(state).is_some())
            }
            None => table_opt(doc, &["attr"]).is_some_and(|t| t.remove(attr).is_some()),
        };
        doc::drop_if_empty(doc, &["attr", attr]);
        doc::drop_if_empty(doc, &["attr"]);
        Ok(removed)
    })
}

/// 型付きのフィールドを書く。
pub fn set_field(edits: &mut Edits, file: &str, field: &PartField) -> Result<(), StoreError> {
    let item: Option<Item> = match field {
        PartField::Label(v)
        | PartField::Description(v)
        | PartField::Catalog(v)
        | PartField::Kind(v) => v.as_deref().map(string),
        PartField::Count(1) => None,
        PartField::Count(n) => Some(Item::Value(i64::from(*n).into())),
        PartField::Maturity(m) => m.map(|m| string(m.as_str())),
        PartField::Candidates(c) if c.is_empty() => None,
        PartField::Candidates(c) => Some(strings(c.iter().map(String::as_str))),
    };
    edit_part(edits, file, |doc| {
        set_opt(doc.as_table_mut(), field.name(), item);
        Ok(())
    })
}

/// 取付と姿勢を書く。
pub fn place(
    edits: &mut Edits,
    file: &str,
    mount: Option<&Mount>,
    pose: Option<&Pose>,
) -> Result<(), StoreError> {
    edit_part(edits, file, |doc| {
        sync_section(doc, "mount", mount.map(mount_table))?;
        sync_section(doc, "pose", pose.map(pose_table))?;
        Ok(())
    })
}

/// 取付先の ID を書き換える（名前の変更に伴う）。
pub fn rename_mount_target(edits: &mut Edits, file: &str, to: &str) -> Result<(), StoreError> {
    edit_part(edits, file, |doc| {
        if let Some(t) = table_opt(doc, &["mount"]) {
            set(t, "on", string(to));
        }
        Ok(())
    })
}

/// ポートのネット所属を書く（None ならポートの表ごと消す）。
pub fn set_port(
    edits: &mut Edits,
    file: &str,
    port: &str,
    net: Option<&str>,
) -> Result<(), StoreError> {
    edit_part(edits, file, |doc| {
        match net {
            Some(net) => set(table_mut(doc, &["ports", port])?, "net", string(net)),
            None => {
                if let Some(ports) = table_opt(doc, &["ports"]) {
                    ports.remove(port);
                }
                doc::drop_if_empty(doc, &["ports"]);
            }
        }
        Ok(())
    })
}

/// 取付点を書く（None なら消す）。
pub fn set_mount_point(
    edits: &mut Edits,
    file: &str,
    point: &str,
    pose: Option<&Pose>,
) -> Result<(), StoreError> {
    edit_part(edits, file, |doc| {
        match pose {
            Some(p) => {
                let fresh = table_opt(doc, &["mount_points", point]).is_none();
                let table = table_mut(doc, &["mount_points", point])?;
                sync_table(table, pose_table(p));
                if fresh {
                    format::align_table(table);
                }
            }
            None => {
                if let Some(points) = table_opt(doc, &["mount_points"]) {
                    points.remove(point);
                }
                doc::drop_if_empty(doc, &["mount_points"]);
            }
        }
        Ok(())
    })
}
