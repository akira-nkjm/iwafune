//! バジェットの API の型（`GET`・`POST /budgets/{kind}`、`fune budget`）。

use super::{FillRatio, QuantityView};
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
/// 入力が足りなければ `missing` に並べ、数えられなかった欄（`total`・`margin`・`verdict`・`rows`・`modes`・`propellant`）は空にする（解析と同じくエラーにしない）。
/// 質量の `total`・`margin`・`verdict`・`fill` は乾燥（部品だけ。推薬を含まない）の値で、推薬込みは `propellant`。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct BudgetView {
    pub kind: BudgetKind,
    /// このバジェットを見せる系（質量 → 構造・電力 → 電源・データ量 → C&DH。`query::settings::budget_system`）
    pub system: sat_analysis::spec::System,
    /// 例: `"質量"`
    pub label: String,
    /// 英語の見出し（例: `"Mass"`）
    pub label_en: Option<String>,
    /// 依頼で指定した解析ケース
    pub case: Option<String>,
    /// 上限（未設定なら null）
    pub limit: Option<QuantityView>,
    /// システムマージン（比。`si` が 0〜1 の値、`display` が `"10.0 %"`。配分が無ければ 0）
    pub system_margin: QuantityView,
    /// 成長余裕込みの合計（入力が足りなければ null。質量は乾燥の合計）
    pub total: Option<QuantityView>,
    /// 上限 ×（1 − システムマージン）− 合計
    pub margin: Option<QuantityView>,
    /// 判定（入力か上限が無ければ null）
    pub verdict: Option<Verdict>,
    /// 塗りの比（合計 / 上限。上限か合計が無ければ null）
    pub fill: Option<FillRatio>,
    /// 行（系や配分された部品）
    pub rows: Vec<BudgetRow>,
    /// 電力のモードごとの行（電力のときだけ。ほかは空）。`node` はモード ID、`allocated` はシステム余裕を
    /// 引いた使用可能量、`margin` と `verdict` は全体と同じ集計・判定。
    pub modes: Vec<BudgetRow>,
    /// 質量の推薬（質量のときだけ。ほかの種類と、部品を数えられなかったときは null）
    pub propellant: Option<BudgetPropellant>,
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
    /// 系か部品の ID（電力のモードの行はモード ID）。どの配分の行にも系にも属さない部品を
    /// まとめた「その他」の行は null（文言は CLI と Web が作る）
    pub node: Option<String>,
    /// 表示名（無ければ ID。「その他」の行は null）
    pub label: Option<String>,
    /// 英語の系の表示名
    pub label_en: Option<String>,
    /// 行の系（モードと「その他」は None）
    pub system: Option<sat_analysis::spec::System>,
    /// 成長余裕を含まない現状の値
    pub current: QuantityView,
    /// 成長余裕込み
    pub with_growth: QuantityView,
    pub allocated: Option<QuantityView>,
    /// 配分 − 成長余裕込み
    pub margin: Option<QuantityView>,
    pub verdict: Verdict,
    /// 塗りの比（成長余裕込み / 配分。配分が無ければ null）
    pub fill: Option<FillRatio>,
}

/// 質量のバジェットの推薬（`BudgetView.propellant`）。値は推進系の解析 `propulsion.propellant` の積む推薬
/// （マージン込みで使う分 + 残る分）だけから取る。推薬は成長余裕を掛けず、乾燥の部品に足して上限と比べる
/// （集計と判定は乾燥と同じ `sat_analysis::budget::budget_rollup`）。数えなかったときは理由を構造で返す（文言は CLI と Web）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum BudgetPropellant {
    /// 推薬を数えた
    Counted {
        /// 値を計算した解析（`"propulsion.propellant"`）
        analysis: String,
        /// 積む推薬
        loaded: QuantityView,
        /// 推薬込み（wet）の合計・余裕・判定・塗りの比
        wet: Box<BudgetTotal>,
    },
    /// 推進系が無い（マヌーバも推進系の部品も無い）ので数えない
    NoPropulsion,
    /// 推進系の入力が足りないので数えない（解析の `missing` と同じ形）
    Missing { missing: Vec<MissingInput> },
    /// 推進系の解析が通らなかったので数えない
    Failed {
        /// 解析の誤りの文
        message: String,
    },
}

/// 合計と上限の比較（推薬込みの合計など）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct BudgetTotal {
    /// 成長余裕込みの合計（推薬には成長余裕を掛けない）
    pub total: QuantityView,
    /// 上限 ×（1 − システムマージン）− 合計（上限が無ければ null）
    pub margin: Option<QuantityView>,
    /// 判定（上限が無ければ null）
    pub verdict: Option<Verdict>,
    /// 塗りの比（合計 / 上限。上限が無ければ null）
    pub fill: Option<FillRatio>,
}
