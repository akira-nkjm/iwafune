//! 解析の所属を表す葉の型（系と前提条件の話題）。

use serde::{Deserialize, Serialize};

/// 解析が属する系。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum System {
    Aocs,
    Cdh,
    Comm,
    Payload,
    Power,
    Propulsion,
    Structure,
    Thermal,
}

/// 前提条件の解析の話題。綴り・表示名は model と projection の対応で一致を確かめる。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ConditionTopic {
    Orbit,
}

/// 解析が属する区分。部品・バジェットの系とは独立。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum AnalysisArea {
    System(System),
    Condition(ConditionTopic),
}
