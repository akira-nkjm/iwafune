//! 配分とマージン方針（`budget/allocation.toml`・`budget/policy.toml`。docs/model.md §8）。
//!
//! serde の形がそのままファイルの形。

use super::defs::BudgetKind;
use super::part::Maturity;
use crate::value::Quantity;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// 配分とマージン方針。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
pub struct Budget {
    pub allocation: Allocations,
    pub policy: BudgetPolicy,
}

/// バジェットの種類ごとの配分（`budget/allocation.toml` の `[mass]`・`[power]`・`[data]`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Allocations {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mass: Option<Allocation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power: Option<Allocation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Allocation>,
}

impl Allocations {
    /// 種類で引く。
    pub fn get(&self, kind: BudgetKind) -> Option<&Allocation> {
        match kind {
            BudgetKind::Mass => self.mass.as_ref(),
            BudgetKind::Power => self.power.as_ref(),
            BudgetKind::Data => self.data.as_ref(),
        }
    }

    /// 書かれている配分を種類と組で列挙する。
    pub fn iter(&self) -> impl Iterator<Item = (BudgetKind, &Allocation)> {
        [
            (BudgetKind::Mass, self.mass.as_ref()),
            (BudgetKind::Power, self.power.as_ref()),
            (BudgetKind::Data, self.data.as_ref()),
        ]
        .into_iter()
        .filter_map(|(kind, a)| a.map(|a| (kind, a)))
    }
}

/// 1 種類の配分。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Allocation {
    /// 上限（ロケット側の上限・軌道平均の発電見込みなど）
    pub limit: Quantity,
    /// システムマージン（上限に対する割合）
    #[serde(default)]
    pub system_margin: f64,
    /// Group か Part の ID → 配分
    #[serde(default)]
    pub alloc: BTreeMap<String, Quantity>,
}

/// 成熟度ごとの係数。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct MaturityFactors {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub off_the_shelf: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new: Option<f64>,
}

impl MaturityFactors {
    /// 成熟度で引く。
    pub fn get(&self, maturity: Maturity) -> Option<f64> {
        match maturity {
            Maturity::OffTheShelf => self.off_the_shelf,
            Maturity::Modified => self.modified,
            Maturity::New => self.new,
        }
    }
}

/// ハーネスの見込み。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct HarnessAllowance {
    /// `harness/` が無い間、サブシステム質量に対して見込む割合
    #[serde(default)]
    pub allowance: f64,
}

/// マージン方針（`budget/policy.toml`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BudgetPolicy {
    /// 成熟度ごとの成長余裕（質量・電力に掛ける）
    #[serde(default)]
    pub growth: MaturityFactors,
    /// 不確かさを書いていない値に当てる既定の幅（±）
    #[serde(default)]
    pub uncertainty: MaturityFactors,
    #[serde(default)]
    pub harness: HarnessAllowance,
}
