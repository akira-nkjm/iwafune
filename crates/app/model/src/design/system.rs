//! 複数の部品にまたがるエンティティ: モード・ネット・ハーネス・解析ケース・出典（docs/model.md §7）。
//!
//! serde の形がそのままファイルの形（`id` はファイル名から決まるので書かない）。

use super::part::Part;
use crate::id::Id;
use crate::value::Quantity;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

pub use super::defs::NetKind;
pub use super::net::{Net, NetField};

/// 衛星の運用モード（`modes/<ID>.toml`）。部品 → 状態の表。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Mode {
    #[serde(default)]
    pub id: Id<Mode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub states: BTreeMap<Id<Part>, StateAssign>,
}

/// 基本の状態と、ほかの状態で過ごす時間の割合（残りは基本の状態）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct StateAssign {
    pub state: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub duty: BTreeMap<String, f64>,
}

impl StateAssign {
    /// 時間割合なしの割り当て。
    pub fn new(state: impl Into<String>) -> Self {
        Self {
            state: state.into(),
            duty: BTreeMap::new(),
        }
    }
}

/// ケーブル区間（物理。`harness/<ID>.toml`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Harness {
    #[serde(default)]
    pub id: Id<Harness>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// 端のポート（`"PCDU.OUT3"`）
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub nets: Vec<Id<Net>>,
    /// 線材（カタログ品）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wire: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cores: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<Quantity>,
}

/// 不確かさのどちら側を読むか。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum UncertaintySide {
    Nominal,
    Min,
    Max,
}

/// 解析ケース（`cases/<ID>.toml`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(rename = "AnalysisCase")]
pub struct Case {
    #[serde(default)]
    pub id: Id<Case>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 解析するモード（無ければ `analysis/power.toml` のモードの割合）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<Id<Mode>>,
    /// 不確かさのどちら側を読むか（既定は nominal）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<UncertaintySide>,
    /// 熱の環境条件（`analysis/thermal.toml` の `[case.<名前>]`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thermal_case: Option<String>,
    /// β 角の上書き
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub beta: Option<Quantity>,
    /// 時系列の解析で回す周回数
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orbits: Option<u32>,
}

/// 出典の種類（根拠の強さの順: placeholder が最も弱い）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Placeholder,
    Judgement,
    Analysis,
    Datasheet,
    Measured,
}

/// 出典の台帳（`sources/<ID>.toml`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(rename = "EvidenceSource")]
pub struct Source {
    #[serde(default)]
    pub id: Id<Source>,
    pub kind: SourceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// 参照先（URL・文書番号など）
    #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// 日付（`2026-10-15` のような文字列）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// 記入者
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl std::fmt::Display for SourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Placeholder => "placeholder",
            Self::Judgement => "judgement",
            Self::Analysis => "analysis",
            Self::Datasheet => "datasheet",
            Self::Measured => "measured",
        })
    }
}
impl std::str::FromStr for SourceKind {
    type Err = String;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "placeholder" => Ok(Self::Placeholder),
            "judgement" => Ok(Self::Judgement),
            "analysis" => Ok(Self::Analysis),
            "datasheet" => Ok(Self::Datasheet),
            "measured" => Ok(Self::Measured),
            _ => Err(format!("Unknown SourceKind: {text}")),
        }
    }
}
