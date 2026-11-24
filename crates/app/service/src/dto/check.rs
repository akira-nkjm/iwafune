//! 解析の判定 1 つの型（`AnalysisRunResult.checks` と概要の判定のまとまり `OverviewCheckGroup.checks` が使う葉の型）。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 解析の判定 1 つ。出力のあちこち（モードごと・ケースごとなど）にある判定を並べるので、どこにあったかを添える。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct CheckView {
    /// 判定の種類（例: `"dod"`）。同じ解析の中でも、モードごとに同じ key が並ぶことがある
    pub key: String,
    pub label: String,
    /// 英語の項目名（語彙から引いた判定だけ。null なら `label` を使う）
    pub label_en: Option<String>,
    pub verdict: sat_analysis::spec::Verdict,
    pub message: String,
    /// 英語の文（語彙から引いた判定だけ。null なら `message` を使う）
    pub message_en: Option<String>,
    /// 判定そのものの場所（出力の JSON Pointer。例: `"/cases/3/checks/1"`）。判定ごとに一意
    pub at: String,
    /// 判定の持ち主の名前（同じ階層の `label`、無ければ `id`。例: `"ダウンリンク"`）。出力の最上位なら null
    pub scope: Option<String>,
    /// 基準に対する余裕（解析の `Check::margin` そのまま。小さいほど厳しく、負なら基準を満たさない。種類は判定ごとに違い、
    /// 比べるのは同じ要求のキーに当たる判定どうしだけ。基準との比較で判定を出す判定だけが持つ）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub margin: Option<f64>,
}
