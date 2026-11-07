//! 値の表示用の型（部品の値・カタログ品の値・設定の値で共有する）。

use super::QuantityView;
use fune_model::resolve::{Evidence, Origin};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// 解決済みの値 1 つ。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ValueView {
    pub attr: String,
    pub value: ValueData,
    /// 表示文字列（例: `"620 g"`、`"on 1.2 W / idle 0.3 W"`）
    pub display: String,
    pub origin: Origin,
    /// 根拠の強さ
    pub evidence: Evidence,
    /// 出典 ID（`sources/<ID>.toml`）
    pub source: Option<String>,
    pub note: Option<String>,
    /// 値が書かれているファイルと行
    pub file: Option<String>,
    pub line: Option<u32>,
}

/// 値の中身（型ごと）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ValueData {
    Unset,
    Scalar {
        nominal: QuantityView,
        min: Option<QuantityView>,
        max: Option<QuantityView>,
    },
    ByState {
        /// 状態 → 値
        states: BTreeMap<String, QuantityView>,
    },
    Vector {
        x: f64,
        y: f64,
        z: f64,
        unit: String,
        /// SI 値 `[x, y, z]`
        si: [f64; 3],
        frame: Option<String>,
    },
    Quat {
        w: f64,
        x: f64,
        y: f64,
        z: f64,
    },
    Tensor {
        m: [[f64; 3]; 3],
        unit: String,
    },
    Range {
        min: QuantityView,
        max: QuantityView,
    },
    Enum {
        value: String,
    },
    Bool {
        value: bool,
    },
    Text {
        value: String,
    },
    Table {
        id: String,
    },
}
