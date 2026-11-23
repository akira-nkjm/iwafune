//! 運用モード（`modes/<ID>.toml`。docs/model.md §7）。部品 → 状態の表。
//!
//! serde の形がそのままファイルの形（`id` はファイル名から決まるので書かない）。

use super::part::Part;
use super::timing::{ModeWhen, StartRef};
use crate::id::Id;
use crate::value::Quantity;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

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
    /// 最長継続時間。`power.energy_balance` は基本のモードの上でこの時間だけ続けたときで判定する。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_duration: Option<Quantity>,
    /// 最長継続時間の評価でこのモードを使う時間帯（省略は `any`）。`start` が無ければ、窓がこの時間帯に入る範囲で開始をずらす。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<ModeWhen>,
    /// 最長継続時間の評価の開始の指定（食の出入り + オフセット）。あればその開始だけで評価する。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<StartRef>,
    /// 蝕の間だけの上書き。
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub eclipse: BTreeMap<Id<Part>, StateAssign>,
}

/// 基本の状態と、ほかの状態で過ごす時間の割合（残りは基本の状態）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct StateAssign {
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_state: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub duty: BTreeMap<String, f64>,
}

impl StateAssign {
    /// 時間割合なしの割り当て。
    pub fn new(state: impl Into<String>) -> Self {
        Self {
            state: state.into(),
            active: None,
            spare_state: None,
            duty: BTreeMap::new(),
        }
    }
}
