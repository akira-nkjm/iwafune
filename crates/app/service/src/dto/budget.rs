//! バジェットの API の型（`GET`・`POST /budgets/{kind}`、`fune budget`）。

use super::QuantityView;
use fune_model::design::BudgetKind;
use fune_ops::Op;
use fune_projection::MissingInput;
use sat_analysis::spec::Verdict;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// バジェットの依頼（`POST /budgets/{kind}`、`fune budget <kind> --case <ID> --ops <file>`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct BudgetRequest {
    /// 解析ケースの ID（`cases/<ID>.toml`）。電力はケースのモードで、データ量はケースのモードだけで数える。
    /// 省略すると既定（電力は合計が最大のモード、データ量は `analysis.power.mode_share` の割合）。`GET` は省略と同じ
    pub case: Option<String>,
    /// 保存していない編集。書き込まずに当てた設計で数える（空なら保存済みの設計。`GET` と同じ）
    pub ops: Vec<Op>,
}

/// 1 種類のバジェット（配分 `budget/allocation.toml` と現状の比較）。
/// 入力が足りなければ `missing` に並べ、数えられなかった欄（`total`・`margin`・`verdict`・`rows`・`modes`）は空にする（解析と同じくエラーにしない）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct BudgetView {
    pub kind: BudgetKind,
    /// 例: `"質量"`
    pub label: String,
    /// 英語の見出し（例: `"Mass"`）
    pub label_en: Option<String>,
    /// 依頼で指定した解析ケース
    pub case: Option<String>,
    /// 上限（配分が無ければ null）
    pub limit: Option<QuantityView>,
    /// システムマージン（比。`si` が 0〜1 の値、`display` が `"10.0 %"`。配分が無ければ 0）
    pub system_margin: QuantityView,
    /// 成長余裕込みの合計（入力が足りなければ null）
    pub total: Option<QuantityView>,
    /// 上限 ×（1 − システムマージン）− 合計
    pub margin: Option<QuantityView>,
    /// 判定（入力が足りなければ null）
    pub verdict: Option<Verdict>,
    /// 行（サブシステムや組立品）
    pub rows: Vec<BudgetRow>,
    /// 電力のモードごとの行（電力のときだけ。ほかは空）。`node` はモード ID、`allocated` はシステム余裕を
    /// 引いた使用可能量、`margin` と `verdict` は全体と同じ集計・判定。
    pub modes: Vec<BudgetRow>,
    /// 値を計算した解析（例: `"structure.mass_properties"`）
    pub analysis: String,
    /// 計算に使ったモード（電力・データのとき）
    pub mode: Option<String>,
    /// 足りない入力（解析の `missing` と同じ形）。空でなければ合計と行は無い
    pub missing: Vec<MissingInput>,
    pub notes: Vec<String>,
}

/// バジェットの 1 行。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct BudgetRow {
    /// Group か Part の ID（電力のモードの行はモード ID）。どの配分の行にも最上位の Group にも属さない部品を
    /// まとめた「その他」の行は null（文言は CLI と Web が作る）
    pub node: Option<String>,
    /// 表示名（無ければ ID。「その他」の行は null）
    pub label: Option<String>,
    /// 成長余裕を含まない現状の値
    pub current: QuantityView,
    /// 成長余裕込み
    pub with_growth: QuantityView,
    pub allocated: Option<QuantityView>,
    /// 配分 − 成長余裕込み
    pub margin: Option<QuantityView>,
    pub verdict: Verdict,
}
