//! 単位の API の型。

use super::QuantityView;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 単位の一覧（`GET /units`、`fune units ls`）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct UnitsCatalog {
    pub dimensions: Vec<DimensionInfo>,
    pub units: Vec<UnitInfo>,
}

/// 名前の付いた次元。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct DimensionInfo {
    /// 例: `"AngularMomentum"`
    pub name: String,
    /// SI の基本単位での式（例: `"kg*m^2/s"`）。無次元は空文字列
    pub si_unit: String,
    /// 表示単位の候補（小さい順。GUI の単位の選択肢）
    pub display_units: Vec<String>,
}

/// 単位記号 1 つ。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct UnitInfo {
    /// 例: `"W"`
    pub symbol: String,
    /// 次元の名前（名前が無ければ指数の表記）
    pub dimension: String,
    /// SI 値 = `scale` × 値 + `offset`（対数単位では 0 dB の基準値）
    pub scale: f64,
    pub offset: f64,
    /// `scale` の数の表示（量ではないので文字列。例: `"0.001"`）
    pub scale_text: String,
    pub mapping: UnitMapping,
}

/// 値と SI 値の対応の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum UnitMapping {
    Linear,
    Affine,
    Logarithmic,
}

/// 単位の一覧の絞り込み。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct UnitsQuery {
    /// 次元の名前（例: `"Power"`）。指定するとその次元の単位だけを返す
    pub dimension: Option<String>,
}

/// 入力文字列の解析の依頼（`POST /units/parse`、`fune units parse`）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ParseQuantityRequest {
    /// 例: `"18 mN*m*s"`、`"0.62 kg"`、`"12"`（無次元）
    pub input: String,
    /// 期待する次元（例: `"AngularMomentum"`）。指定すると次元が違えばエラー
    #[serde(default)]
    pub dimension: Option<String>,
}

/// 入力文字列の解析結果。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ParsedQuantity {
    pub quantity: QuantityView,
    /// 同じ量を SI の組立単位で表したもの（例: `display` が `"0.0180 N·m·s"`。無次元は単位なし）
    pub si: QuantityView,
}

/// SI 値の一括整形の依頼（`POST /units/format`、`fune units format`）。解析の出力（SI の数値）を画面に出すのに使う。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FormatRequest {
    pub items: Vec<FormatItem>,
}

/// 整形する値 1 つ。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FormatItem {
    /// SI 値
    pub si: f64,
    /// 次元の名前（例: `"Information"`。Energy と Torque のように同じ次元でも名前で区別する）
    pub dimension: String,
    /// 単位を指定する（例: `"Mbit"`）。省略すると読みやすい単位を選ぶ
    #[serde(default)]
    pub unit: Option<String>,
}

/// 一括整形の結果（`items` と同じ順。量の表示はほかの API と同じ `QuantityView`）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FormatResponse {
    pub items: Vec<QuantityView>,
}
