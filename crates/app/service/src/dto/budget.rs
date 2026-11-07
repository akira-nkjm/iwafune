//! バジェットの API の型（`GET`・`POST /budgets/{kind}`、`fune budget`）。

use super::QuantityView;
use fune_model::design::BudgetKind;
use fune_ops::Op;
use sat_analysis::spec::Verdict;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// バジェットの依頼（`POST /budgets/{kind}`、`fune budget <kind> --ops`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct BudgetRequest {
    /// 保存していない編集。書き込まずに当てた設計で数える（空なら保存済みの設計。`GET` と同じ）
    pub ops: Vec<Op>,
}

/// 1 種類のバジェット（配分 `budget/allocation.toml` と現状の比較）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct BudgetView {
    pub kind: BudgetKind,
    /// 例: `"質量"`
    pub label: String,
    /// 英語の見出し（例: `"Mass"`）
    pub label_en: Option<String>,
    /// 上限（配分が無ければ null）
    pub limit: Option<QuantityView>,
    pub system_margin: f64,
    /// `system_margin` の表示（`"10.0 %"`）
    pub system_margin_display: QuantityView,
    /// 成長余裕込みの合計
    pub total: QuantityView,
    /// 上限 ×（1 − システムマージン）− 合計
    pub margin: Option<QuantityView>,
    pub verdict: Verdict,
    /// 行（サブシステムや組立品）
    pub rows: Vec<BudgetRow>,
    /// 電力のモードごとの行（電力のときだけ。ほかは空）。`node` はモード ID、`allocated` はシステム余裕を
    /// 引いた使用可能量、`margin` と `verdict` は全体と同じ集計・判定。
    pub modes: Vec<BudgetRow>,
    /// 値を計算した解析（例: `"structure.mass_properties"`）
    pub analysis: String,
    /// 計算に使ったモード（電力・データのとき）
    pub mode: Option<String>,
    pub notes: Vec<String>,
}

/// バジェットの 1 行。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct BudgetRow {
    /// Group か Part の ID
    pub node: String,
    pub label: String,
    /// 成長余裕を含まない現状の値
    pub current: QuantityView,
    /// 成長余裕込み
    pub with_growth: QuantityView,
    pub allocated: Option<QuantityView>,
    /// 配分 − 成長余裕込み
    pub margin: Option<QuantityView>,
    pub verdict: Verdict,
}
