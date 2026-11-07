//! 属性定義（量の種類）と種別定義（docs/model.md §3）。
//!
//! 組み込みの定義（[`crate::builtin`]）とプロジェクト独自の定義（`defs/attribute/<id>.toml`・
//! `defs/kind/<id>.toml`）は同じ形。serde の形がそのままファイルの形になる（`id` はファイル名から決まる）。

use crate::id::Id;
use crate::value::ValueType;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// どのバジェットに入るか。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum BudgetKind {
    Mass,
    Power,
    Data,
}

impl BudgetKind {
    /// 配分と集計の次元。
    pub fn dimension(self) -> &'static str {
        match self {
            BudgetKind::Mass => "Mass",
            BudgetKind::Power => "Power",
            BudgetKind::Data => "Information",
        }
    }
}

/// ネットの種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum NetKind {
    Power,
    Data,
    Rf,
}

impl NetKind {
    /// ファイルに書く綴り。
    pub fn as_str(self) -> &'static str {
        match self {
            NetKind::Power => "power",
            NetKind::Data => "data",
            NetKind::Rf => "rf",
        }
    }
}

/// 属性定義（quantity kind）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct AttributeDef {
    /// ファイル名（組み込みは表のキー）から決まる
    #[serde(default)]
    pub id: Id<AttributeDef>,
    pub label: String,
    /// 英語の表示名（GUI の英語表示。無ければ `label`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_en: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 値の型
    #[serde(rename = "type")]
    pub value_type: ValueType,
    /// 機器の状態ごとに値を持つか（電力・データレートなど）
    #[serde(default, skip_serializing_if = "is_false")]
    pub by_state: bool,
    /// 次元名（sat-units の名前。例: `"Mass"`）。無ければ `units` の先頭の単位の次元
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimension: Option<String>,
    /// 許可する単位（主な単位）。この外の単位は同じ次元でも warning
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub units: Vec<String>,
    /// 表示単位（無ければ `units` の先頭）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_unit: Option<String>,
    /// 値域の下限・上限（SI）。外れたら warning
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    /// 列挙の語彙（空なら自由）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub symbols: Vec<String>,
    /// どのバジェットに入るか
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<BudgetKind>,
    /// 既定値（SI のスカラー）。書かれていなければこの値で解決する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<f64>,
    /// 方向ベクトル（単位ベクトルであるべきもの）
    #[serde(default, skip_serializing_if = "is_false")]
    pub unit_vector: bool,
    /// 画面で並べる系（`basic`・`mass`・`power`・`thermal`・`attitude`・`comm`・`cdh`・`structure`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

impl AttributeDef {
    /// 表示単位（`display_unit` か `units` の先頭。無次元は空文字列）。
    pub fn display_unit(&self) -> &str {
        self.display_unit
            .as_deref()
            .or_else(|| self.units.first().map(String::as_str))
            .unwrap_or("")
    }
}

/// 種別定義。`extends` の連鎖で状態・属性・ポートを継承する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct KindDef {
    #[serde(default)]
    pub id: Id<KindDef>,
    pub label: String,
    /// 英語の表示名（GUI の英語表示。無ければ `label`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_en: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extends: Option<Id<KindDef>>,
    /// 解析が部品を見分ける役割（registry §3 の role）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// 機器の状態の語彙（空なら継承元のもの。どこにも無ければ状態を持たない）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub states: Vec<String>,
    /// 必須の属性（欠けていれば warning）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<Id<AttributeDef>>,
    /// 任意の属性
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub optional: Vec<Id<AttributeDef>>,
    /// ポートの雛形（ポート名 → ネットの種類）
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub ports: BTreeMap<String, NetKind>,
}

impl std::fmt::Display for BudgetKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Mass => "mass",
            Self::Power => "power",
            Self::Data => "data",
        })
    }
}
impl std::str::FromStr for BudgetKind {
    type Err = String;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "mass" => Ok(Self::Mass),
            "power" => Ok(Self::Power),
            "data" => Ok(Self::Data),
            _ => Err(format!("Unknown BudgetKind: {text}")),
        }
    }
}

impl std::fmt::Display for NetKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
impl std::str::FromStr for NetKind {
    type Err = String;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "power" => Ok(Self::Power),
            "data" => Ok(Self::Data),
            "rf" => Ok(Self::Rf),
            _ => Err(format!("Unknown NetKind: {text}")),
        }
    }
}
