//! 属性の値のファイル上の形 ⇄ [`Value`]（**属性の値の形を知っているのはここだけ**）。
//!
//! | 値 | ファイルの形 |
//! |---|---|
//! | スカラー | `{ value = 0.62, unit = "kg", from = { source = "…" } }`、不確かさは `min` / `max`、無次元は `0.6` |
//! | 状態ごと | `[attr.power]` の下に `on = { value = 1.2, unit = "W" }` を 1 行 1 セル |
//! | ベクトル | `{ x = 0.0, y = 0.0, z = 1.0 }`（単位があれば `unit`） |
//! | クォータニオン | `{ w = 1.0, x = 0.0, y = 0.0, z = 0.0 }` |
//! | 範囲 | `{ min = -20.0, max = 60.0, unit = "degC" }` |
//! | 曲線（1 変数の関数） | `{ at = [0.0, 10.0], at_unit = "km^2/s^2", values = [480.0, 360.0], unit = "kg" }` |
//! | 列挙・文字列・真偽 | `"al6061"`・`true`（出典を付けるなら `{ value = "al6061", from = … }`） |
//! | 表 | `{ table = "SA_IV" }` |
//!
//! 出典（`from`）の付いた値は 1 行 1 要素で書く（`[attr.mass]` の下に `value`・`unit`・`from.source`…、状態ごとの値の
//! セルは `[attr.power]` の下に `on.value`・`on.from.source`…）。形は `crate::profile::lay_out` が決める。読み込みはどちらの形も同じ値になる。

use crate::profile::lay_out;
use fune_model::value::{
    Authored, Curve, Provenance, Quaternion, RangeValue, Scalar, Tensor3, Value, ValueType, Vector3,
};
use std::collections::BTreeMap;
use toml_edit::{InlineTable, Item, Table};

/// `[attr]` の見出しの段数（属性の行を置く表の深さ。状態ごとの値のセルは 1 段深い）。
const ATTR_DEPTH: usize = 1;

type TomlTable = toml::Table;

/// ファイルの値 → [`Value`]。`hint` は属性定義の値の型（文字列を列挙と文字列のどちらで読むか）。
pub fn decode_value(raw: &toml::Value, hint: Option<ValueType>) -> Result<Authored<Value>, String> {
    match raw {
        toml::Value::Float(_) | toml::Value::Integer(_) => {
            Ok(Authored::bare(Value::Scalar(Scalar::new(number(raw)?, ""))))
        }
        toml::Value::Boolean(b) => Ok(Authored::bare(Value::Bool(*b))),
        toml::Value::String(s) => Ok(Authored::bare(text_value(s, hint))),
        toml::Value::Table(t) => decode_table(t, hint),
        other => Err(format!(
            "{} cannot be written as an attribute value",
            other.type_str()
        )),
    }
}

fn text_value(s: &str, hint: Option<ValueType>) -> Value {
    match hint {
        Some(ValueType::Enum) => Value::Enum(s.to_string()),
        Some(ValueType::Table) => Value::Table(s.to_string()),
        _ => Value::Text(s.to_string()),
    }
}

fn number(raw: &toml::Value) -> Result<f64, String> {
    match raw {
        toml::Value::Float(f) => Ok(*f),
        #[allow(clippy::cast_precision_loss)]
        toml::Value::Integer(i) => Ok(*i as f64),
        other => Err(format!("Expected a number but found {}", other.type_str())),
    }
}

fn opt_number(t: &TomlTable, key: &str) -> Result<Option<f64>, String> {
    t.get(key).map(number).transpose()
}

fn req_number(t: &TomlTable, key: &str) -> Result<f64, String> {
    opt_number(t, key)?.ok_or_else(|| format!("`{key}` is missing"))
}

fn opt_string(t: &TomlTable, key: &str) -> Result<Option<String>, String> {
    match t.get(key) {
        None => Ok(None),
        Some(toml::Value::String(s)) => Ok(Some(s.clone())),
        Some(other) => Err(format!(
            "`{key}` must be a string (found {})",
            other.type_str()
        )),
    }
}

fn provenance(t: &TomlTable) -> Result<Option<Provenance>, String> {
    t.get("from")
        .map(|v| {
            v.clone()
                .try_into::<Provenance>()
                .map_err(|e| format!("`from`: {e}"))
        })
        .transpose()
}

fn only_keys(t: &TomlTable, allowed: &[&str]) -> Result<(), String> {
    match t.keys().find(|k| !allowed.contains(&k.as_str())) {
        Some(k) => Err(format!(
            "Key `{k}` is not allowed (allowed: {})",
            allowed.join(", ")
        )),
        None => Ok(()),
    }
}

fn decode_table(t: &TomlTable, hint: Option<ValueType>) -> Result<Authored<Value>, String> {
    let from = provenance(t)?;
    let has = |k: &str| t.contains_key(k);
    let value = if has("value") {
        match t.get("value") {
            Some(toml::Value::String(s)) => {
                only_keys(t, &["value", "from"])?;
                text_value(s, hint)
            }
            Some(toml::Value::Boolean(b)) => {
                only_keys(t, &["value", "from"])?;
                Value::Bool(*b)
            }
            _ => Value::Scalar(scalar(t)?),
        }
    } else if has("w") {
        only_keys(t, &["w", "x", "y", "z", "from"])?;
        Value::Quat(Quaternion {
            w: req_number(t, "w")?,
            x: req_number(t, "x")?,
            y: req_number(t, "y")?,
            z: req_number(t, "z")?,
        })
    } else if has("x") {
        only_keys(t, &["x", "y", "z", "unit", "frame", "from"])?;
        Value::Vec3(Vector3 {
            x: req_number(t, "x")?,
            y: req_number(t, "y")?,
            z: req_number(t, "z")?,
            unit: opt_string(t, "unit")?,
            frame: opt_string(t, "frame")?,
        })
    } else if has("min") && has("max") {
        only_keys(t, &["min", "max", "unit", "from"])?;
        Value::Range(RangeValue {
            min: req_number(t, "min")?,
            max: req_number(t, "max")?,
            unit: opt_string(t, "unit")?.unwrap_or_default(),
        })
    } else if has("at") {
        only_keys(t, &["at", "at_unit", "values", "unit", "from"])?;
        Value::Curve(Curve {
            at: number_array(t, "at")?,
            at_unit: opt_string(t, "at_unit")?.unwrap_or_default(),
            values: number_array(t, "values")?,
            unit: opt_string(t, "unit")?.unwrap_or_default(),
        })
    } else if has("m") {
        only_keys(t, &["m", "unit", "frame", "from"])?;
        Value::Tensor3(tensor(t)?)
    } else if has("table") {
        only_keys(t, &["table", "from"])?;
        Value::Table(opt_string(t, "table")?.unwrap_or_default())
    } else {
        return by_state(t).map(Authored::bare);
    };
    Ok(Authored { value, from })
}

fn scalar(t: &TomlTable) -> Result<Scalar, String> {
    only_keys(t, &["value", "min", "max", "unit", "from"])?;
    Ok(Scalar {
        value: req_number(t, "value")?,
        min: opt_number(t, "min")?,
        max: opt_number(t, "max")?,
        unit: opt_string(t, "unit")?.unwrap_or_default(),
    })
}

/// 数の配列（曲線の引数・値）。
fn number_array(t: &TomlTable, key: &str) -> Result<Vec<f64>, String> {
    t.get(key)
        .and_then(toml::Value::as_array)
        .ok_or_else(|| format!("`{key}` must be an array of numbers"))?
        .iter()
        .map(number)
        .collect()
}

fn tensor(t: &TomlTable) -> Result<Tensor3, String> {
    let rows = t
        .get("m")
        .and_then(toml::Value::as_array)
        .ok_or("`m` must be a 3×3 array")?;
    let mut m = [[0.0; 3]; 3];
    if rows.len() != 3 {
        return Err("`m` must be a 3×3 array".into());
    }
    for (row, raw) in m.iter_mut().zip(rows) {
        let cells = raw.as_array().ok_or("`m` must be a 3×3 array")?;
        if cells.len() != 3 {
            return Err("`m` must be a 3×3 array".into());
        }
        for (cell, v) in row.iter_mut().zip(cells) {
            *cell = number(v)?;
        }
    }
    Ok(Tensor3 {
        m,
        unit: opt_string(t, "unit")?.unwrap_or_default(),
        frame: opt_string(t, "frame")?,
    })
}

/// 状態ごとの値（キーが状態、値がスカラー）。
fn by_state(t: &TomlTable) -> Result<Value, String> {
    if t.is_empty() {
        return Err("An empty table is not a value".into());
    }
    let mut cells = BTreeMap::new();
    for (state, raw) in t {
        let cell = match raw {
            toml::Value::Table(c) => Authored {
                value: scalar(c).map_err(|e| format!("State `{state}`: {e}"))?,
                from: provenance(c)?,
            },
            toml::Value::Float(_) | toml::Value::Integer(_) => {
                Authored::bare(Scalar::new(number(raw)?, ""))
            }
            other => {
                return Err(format!(
                    "{} cannot be written as a value of state `{state}`",
                    other.type_str()
                ));
            }
        };
        cells.insert(state.clone(), cell);
    }
    Ok(Value::ByState(cells))
}

// ---- 書き出し

/// 物理量の数値（常に float で書く。TOML 1.0 の範囲）。
pub fn float(v: f64) -> toml_edit::Value {
    toml_edit::Value::from(v)
}

/// 出典の表（`{ source, at, note }` の順）。**`from` の表の書き出しはここだけ**（量の行も使う）。
pub fn encode_from(from: &Provenance) -> InlineTable {
    let mut f = InlineTable::new();
    for (key, v) in [
        ("source", &from.source),
        ("at", &from.at),
        ("note", &from.note),
    ] {
        if let Some(v) = v {
            f.insert(key, v.as_str().into());
        }
    }
    f
}

/// 出典があれば `from` を足す。
pub fn push_from(t: &mut InlineTable, from: Option<&Provenance>) {
    if let Some(from) = from {
        t.insert("from", toml_edit::Value::InlineTable(encode_from(from)));
    }
}

/// スカラーの行（`{ value, min, max, unit, from }`）。無次元で不確かさも出典も無ければ数値だけ。
pub fn encode_scalar(s: &Scalar, from: Option<&Provenance>) -> toml_edit::Value {
    if s.unit.is_empty() && s.min.is_none() && s.max.is_none() && from.is_none() {
        return float(s.value);
    }
    let mut t = InlineTable::new();
    t.insert("value", float(s.value));
    if let Some(m) = s.min {
        t.insert("min", float(m));
    }
    if let Some(m) = s.max {
        t.insert("max", float(m));
    }
    t.insert("unit", s.unit.as_str().into());
    push_from(&mut t, from);
    toml_edit::Value::InlineTable(t)
}

/// ベクトル（`{ x, y, z, unit }`）。
pub fn encode_vector(v: &Vector3, from: Option<&Provenance>) -> toml_edit::Value {
    let mut t = InlineTable::new();
    t.insert("x", float(v.x));
    t.insert("y", float(v.y));
    t.insert("z", float(v.z));
    if let Some(u) = &v.unit {
        t.insert("unit", u.as_str().into());
    }
    if let Some(f) = &v.frame {
        t.insert("frame", f.as_str().into());
    }
    push_from(&mut t, from);
    toml_edit::Value::InlineTable(t)
}

/// クォータニオン（`{ w, x, y, z }`）。
pub fn encode_quaternion(q: &Quaternion) -> toml_edit::Value {
    let mut t = InlineTable::new();
    t.insert("w", float(q.w));
    t.insert("x", float(q.x));
    t.insert("y", float(q.y));
    t.insert("z", float(q.z));
    toml_edit::Value::InlineTable(t)
}

fn text(s: &str, from: Option<&Provenance>) -> toml_edit::Value {
    match from {
        None => s.into(),
        Some(_) => {
            let mut t = InlineTable::new();
            t.insert("value", s.into());
            push_from(&mut t, from);
            toml_edit::Value::InlineTable(t)
        }
    }
}

/// 状態ごとの値の 1 セル（`[attr.<名前>]` の 1 行。出典があればドット付きキーの行）。
pub fn encode_cell(cell: &Authored<Scalar>) -> Item {
    lay_out(
        encode_scalar(&cell.value, cell.from.as_ref()),
        ATTR_DEPTH + 1,
    )
}

/// [`Value`] → ファイルの値（`[attr]` の 1 行）。状態ごとの値と出典の付いた値は `[attr.<名前>]` の表になる。
pub fn encode_value(value: &Authored<Value>) -> Item {
    let from = value.from.as_ref();
    let v = match &value.value {
        Value::Scalar(s) => encode_scalar(s, from),
        Value::ByState(cells) => {
            let mut table = Table::new();
            for (state, cell) in cells {
                table.insert(state, encode_cell(cell));
            }
            return Item::Table(table);
        }
        Value::Vec3(v) => encode_vector(v, from),
        Value::Quat(q) => encode_quaternion(q),
        Value::Tensor3(t) => {
            let mut it = InlineTable::new();
            let rows: toml_edit::Array =
                t.m.iter()
                    .map(|row| {
                        let r: toml_edit::Array = row.iter().map(|v| float(*v)).collect();
                        toml_edit::Value::Array(r)
                    })
                    .collect();
            it.insert("m", toml_edit::Value::Array(rows));
            it.insert("unit", t.unit.as_str().into());
            push_from(&mut it, from);
            toml_edit::Value::InlineTable(it)
        }
        Value::Enum(s) | Value::Text(s) => text(s, from),
        Value::Bool(b) => match from {
            None => (*b).into(),
            Some(_) => {
                let mut t = InlineTable::new();
                t.insert("value", (*b).into());
                push_from(&mut t, from);
                toml_edit::Value::InlineTable(t)
            }
        },
        Value::Range(r) => {
            let mut t = InlineTable::new();
            t.insert("min", float(r.min));
            t.insert("max", float(r.max));
            t.insert("unit", r.unit.as_str().into());
            push_from(&mut t, from);
            toml_edit::Value::InlineTable(t)
        }
        Value::Curve(c) => {
            let numbers = |xs: &[f64]| xs.iter().map(|x| float(*x)).collect::<toml_edit::Array>();
            let mut t = InlineTable::new();
            t.insert("at", numbers(&c.at).into());
            if !c.at_unit.is_empty() {
                t.insert("at_unit", c.at_unit.as_str().into());
            }
            t.insert("values", numbers(&c.values).into());
            t.insert("unit", c.unit.as_str().into());
            push_from(&mut t, from);
            toml_edit::Value::InlineTable(t)
        }
        Value::Table(id) => {
            let mut t = InlineTable::new();
            t.insert("table", id.as_str().into());
            push_from(&mut t, from);
            toml_edit::Value::InlineTable(t)
        }
    };
    lay_out(v, ATTR_DEPTH)
}

#[cfg(test)]
mod tests;
