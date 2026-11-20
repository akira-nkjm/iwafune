//! 系の概要の「解析の表」の型（1 行 = 1 解析）。どの系の概要でも同じ形で、行に出す主な結果・判定のまとまり・
//! 開いたときの小さな棒グラフ・「データを見る」で隠す表は `fune_service::analysis::overview` と系ごとの組み立て
//! （電源は `analysis::overview::power`）が決める。Web は並べて描くだけ。

use super::{AnalysisLabel, CheckView, KpiView, QuantityView};
use sat_analysis::spec::Verdict;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 解析の表の 1 行に添える概要（解析の一覧の行 `AnalysisEntry` と結果 `AnalysisRunResult` の横に置く）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
pub struct AnalysisOverview {
    /// 行に出す主な結果（結果の `kpis` を定義の順に `analysis::overview::MAIN_KPIS` 個まで）。全部は結果の `kpis`
    pub main: Vec<KpiView>,
    /// 判定の文を持ち主（ケース・モードなど。`CheckView.scope`）ごとにまとめたもの（出てきた順）
    pub checks: Vec<OverviewCheckGroup>,
    /// 開いたときに出す小さな棒グラフ（解析ごとに service が選ぶ。無ければ null）
    pub chart: Option<OverviewChart>,
    /// 「データを見る」で出さない表（同じ表がほかの解析の行にあるもの）。出力の中の位置のキーの列（`["mode_loads", "modes"]`）
    pub hidden_tables: Vec<Vec<String>>,
}

/// 同じ持ち主の判定の文のまとまり。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct OverviewCheckGroup {
    /// 持ち主の名前（電力収支のケース・モードの並びなど。出力の最上位の判定は null）
    pub scope: Option<String>,
    pub checks: Vec<CheckView>,
}

/// 小さな棒グラフ（1 本 = 1 行）。塗りと線の位置はすべて 0〜1 で、service が計算する。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct OverviewChart {
    /// グラフの題（「モードごとの消費電力」など）
    pub label: AnalysisLabel,
    pub bars: Vec<OverviewBar>,
}

/// 棒 1 本。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct OverviewBar {
    /// グラフの中で一意なキー
    pub key: String,
    /// 行の名前（モード・ケース・量）
    pub label: AnalysisLabel,
    /// 値（出せなければ null）
    pub value: Option<QuantityView>,
    /// 塗りの長さ（0〜1）
    pub fill: f64,
    /// 比べる値（上限・必要値。無ければ null）
    pub reference: Option<OverviewReference>,
    /// 判定（判定の対象でなければ null）
    pub verdict: Option<Verdict>,
    /// 補足（「上限の 47.0 %」など。無ければ null）
    pub note: Option<AnalysisLabel>,
}

/// 棒と比べる値と、その線の位置。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct OverviewReference {
    /// 「上限」「必要」など
    pub label: AnalysisLabel,
    pub value: QuantityView,
    /// 線の位置（0〜1）
    pub at: f64,
}
