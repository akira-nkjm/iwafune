//! 書かれたままの量・値の型・出典（docs/model.md §4）。
//!
//! 単位は書かれた文字列のまま持つ（`mN*m*s` は `mN*m*s` のまま）。単位の解析と SI 換算は
//! [`units`] から sat-units を呼ぶ（ここ以外で単位文字列を解析しない）。
//! ファイル上の形（`{ value, unit, from }`）への変換は fune-store の codec が持つ。

mod date;
pub mod json;
mod text;
pub mod units;

pub use date::is_calendar_date;
pub use json::{is_quantity_json, quantity_from_json, quaternion_from_json, vector_from_json};
pub use text::{
    list_from_text, list_to_text, number_to_text, number_unit_to_text, parse_clock_hours,
    parse_number_unit, quantity_from_text, quantity_to_text, quaternion_to_text, range_to_text,
    setting_from_text, setting_to_text, value_from_text, value_to_text, vector_to_text,
};
pub use units::{UnitInfo, dimension_label, unit_dimension, unit_info};

use crate::enum_text::enum_text;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// 値の出典。`source` は `sources/<ID>.toml` の ID。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// 資料の中の位置（`p.12 表 3` のようなページ・表・節。書かれたまま）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl Provenance {
    /// 打った出典・位置・メモから作る。どれも無ければ None（出典を付けない）。
    pub fn from_parts(
        source: Option<String>,
        at: Option<String>,
        note: Option<String>,
    ) -> Option<Self> {
        (source.is_some() || at.is_some() || note.is_some()).then_some(Self { source, at, note })
    }
}

/// 書かれた値と、その出典。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct Authored<T> {
    pub value: T,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<Provenance>,
}

impl<T> Authored<T> {
    /// 出典なしの値。
    pub fn bare(value: T) -> Self {
        Self { value, from: None }
    }
}

/// 書かれたままの量（数値＋単位文字列＋任意の出典）。ファイルでも API でも `{ value, unit, from }`。
/// 無次元は `unit = ""`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Quantity {
    pub value: f64,
    #[serde(default)]
    pub unit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<Provenance>,
}

impl Quantity {
    /// 出典なしの量。
    pub fn new(value: f64, unit: impl Into<String>) -> Self {
        Self {
            value,
            unit: unit.into(),
            from: None,
        }
    }
}

/// スカラー（nominal と任意の不確かさ `min` / `max`。単位は共通）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(rename = "ScalarValue")]
pub struct Scalar {
    pub value: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default)]
    pub unit: String,
}

impl Scalar {
    /// 不確かさなしのスカラー。
    pub fn new(value: f64, unit: impl Into<String>) -> Self {
        Self {
            value,
            min: None,
            max: None,
            unit: unit.into(),
        }
    }
}

/// 3 次元ベクトル。`unit` が無ければ無次元（方向など）。`frame` は既定で部品座標。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Vector3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<String>,
}

impl Vector3 {
    /// 無次元のベクトル。
    pub fn unitless(x: f64, y: f64, z: f64) -> Self {
        Self {
            x,
            y,
            z,
            unit: None,
            frame: None,
        }
    }
}

/// クォータニオン（スカラー部が `w`）。子の座標系のベクトルを親の座標系へ回す回転。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Quaternion {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Quaternion {
    /// 恒等回転。
    pub const IDENTITY: Quaternion = Quaternion {
        w: 1.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
}

/// 3×3 のテンソル（慣性など）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(rename = "Tensor3Value")]
pub struct Tensor3 {
    pub m: [[f64; 3]; 3],
    #[serde(default)]
    pub unit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<String>,
}

/// 仕様の範囲（動作温度範囲など）。値のばらつきは [`Scalar`] の min / max で表す。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct RangeValue {
    pub min: f64,
    pub max: f64,
    #[serde(default)]
    pub unit: String,
}

/// 属性の値。型は AttributeDef が宣言し、`check` が一致を検査する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
#[ts(rename = "AttrValue")]
pub enum Value {
    Scalar(Scalar),
    /// 機器の状態ごとの値（電力・データレートなど）。状態ごとに出典を持てる。
    ByState(BTreeMap<String, Authored<Scalar>>),
    Vec3(Vector3),
    Quat(Quaternion),
    Tensor3(Tensor3),
    Enum(String),
    Bool(bool),
    Text(String),
    Range(RangeValue),
    /// `tables/<ID>` の参照
    Table(String),
}

impl Value {
    /// 値の形（AttributeDef の `type` と比べるため）。ByState は `Scalar` を返す（状態の有無は別に見る）。
    pub fn value_type(&self) -> ValueType {
        match self {
            Value::Scalar(_) | Value::ByState(_) => ValueType::Scalar,
            Value::Vec3(_) => ValueType::Vec3,
            Value::Quat(_) => ValueType::Quat,
            Value::Tensor3(_) => ValueType::Tensor3,
            Value::Enum(_) => ValueType::Enum,
            Value::Bool(_) => ValueType::Bool,
            Value::Text(_) => ValueType::Text,
            Value::Range(_) => ValueType::Range,
            Value::Table(_) => ValueType::Table,
        }
    }

    /// 書かれている単位（あれば）。ByState は最初の状態の単位。
    pub fn unit(&self) -> Option<&str> {
        match self {
            Value::Scalar(s) => Some(&s.unit),
            Value::ByState(m) => m.values().next().map(|c| c.value.unit.as_str()),
            Value::Vec3(v) => v.unit.as_deref(),
            Value::Tensor3(t) => Some(&t.unit),
            Value::Range(r) => Some(&r.unit),
            _ => None,
        }
    }
}

enum_text! {
    /// 値の型（AttributeDef の `type`）。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
    #[ts(rename = "AttrValueType")]
    pub enum ValueType: "値の型" {
        Scalar = "scalar",
        Vec3 = "vec3",
        Quat = "quat",
        Tensor3 = "tensor3",
        Enum = "enum",
        Bool = "bool",
        Text = "text",
        Range = "range",
        Table = "table",
    }
}

/// 値・単位の解析と検査のエラー。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ValueError {
    #[error("単位 `{unit}` を解析できません: {reason}")]
    Unit { unit: String, reason: String },
    #[error("次元が合いません: `{unit}` は {found} ですが {expected} が必要です")]
    Dimension {
        unit: String,
        expected: String,
        found: String,
    },
    #[error("次元名 `{0}` を知りません")]
    UnknownDimension(String),
    #[error("値 `{0}` を読めません")]
    Syntax(String),
    #[error("値の型が違います: {0}")]
    Type(String),
}

/// 設定（`conditions/`・`analysis/`・`budget/`）の 1 つの値。`SetSetting` の Op と store の書き込みが共有する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum SettingValue {
    Number(f64),
    Integer(i64),
    Bool(bool),
    Text(String),
    Quantity(Quantity),
    Vector(Vector3),
    Quaternion(Quaternion),
    List(Vec<String>),
}
