//! 電源ページの表示定義。位置・比・判定・量はサービスが返す。
use super::{AnalysisLabel, DisplaySeries, QuantityView};
use sat_analysis::spec::{System, Verdict};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// ケースに対する電源ページ全体の表示。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
pub struct PowerPageView {
    /// 全解析を統合したモードごとの表示。
    pub modes: Vec<PowerModeView>,
    /// データを見る・不足入力・設定に使う元の解析と表示名（電源の解析 4 つ。並びは負荷・軌道発電・電力収支・サイジング）。
    pub analyses: Vec<PowerAnalysisView>,
}

/// 選択するモードと、そのモードの表示。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PowerModeView {
    pub id: String,
    pub label: String,
    pub verdict: Option<Verdict>,
    pub sunlit: Option<PowerBudgetView>,
    pub eclipse: Option<PowerBudgetView>,
    pub charts: Vec<PowerChartView>,
}

/// 日照中（または蝕中）の電力の行き先を 1 本の積み上げ棒にした表示。
/// 幅 `fraction` は、同じ照明条件（日照・蝕）の全モードで共通の最大に対する割合（0〜1）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PowerBudgetView {
    /// 供給（日照: 太陽電池の日照中の平均発電、蝕: 電池からの放電）
    pub supply: QuantityView,
    /// 「余裕の少ない順」に並べるときの値（小さいほど余裕が少ない）。日照は余り − 不足 [W]、蝕は −放電 [W]
    pub order: f64,
    pub segments: Vec<PowerBudgetSegment>,
}

/// 棒の 1 区間。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PowerBudgetSegment {
    pub kind: PowerBudgetKind,
    /// 負荷のとき、その系（色に使う）
    pub system: Option<System>,
    pub label: AnalysisLabel,
    pub value: QuantityView,
    pub fraction: f64,
}

/// 区間の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum PowerBudgetKind {
    /// 系の消費
    Load,
    /// 電池への充電
    Charge,
    /// 余り（シャントで捨てる分）
    Shunt,
    /// 不足（日照中なのに電池から出す分）
    Deficit,
}

/// 判定 1 つ。メーターは「限界・必要値に対する割合」で、位置はすべて 0〜100（サービスが計算する）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PowerVerdictView {
    pub key: String,
    pub label: AnalysisLabel,
    pub value: Option<QuantityView>,
    pub limit: QuantityView,
    pub verdict: Option<Verdict>,
    pub kind: PowerLimitKind,
    /// 「上限の 143 %」「必要の 59 %」「8.70 Wh 不足」など
    pub note: AnalysisLabel,
    /// メーターの塗りの始まりと終わり
    pub fill_start: f64,
    pub fill_end: f64,
    /// 限界・必要値の線の位置
    pub limit_at: f64,
}

/// 限界の向き。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum PowerLimitKind {
    /// 限界以下なら OK（DoD）
    AtMost,
    /// 必要値以上なら OK（面積・容量・収支）
    AtLeast,
}

/// 共通の横軸・同期カーソルを使うグラフの定義。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PowerChartView {
    pub id: String,
    pub label: AnalysisLabel,
    pub series: DisplaySeries,
    pub domain: [f64; 2],
    pub show_time_axis: bool,
    pub bands: Vec<PowerChartBand>,
    /// 表示単位での許容線。
    pub reference: Option<f64>,
    pub reference_label: Option<AnalysisLabel>,
}

/// 時間軸上の蝕・モードの帯。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PowerChartBand {
    pub start: f64,
    pub end: f64,
    pub eclipse: bool,
    pub mode_index: usize,
    pub label: String,
}

/// 電源ページが収集した元の解析結果と定義。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PowerAnalysisView {
    pub entry: super::AnalysisEntry,
    /// 解析の結果。概要の解析の表の行（`overview`）には電源の棒グラフと隠す表が入っている（`power_page::overview`）
    pub result: super::AnalysisRunResult,
}

/// 電源ページの問い合わせ（`GET /power_page?case=…`、`POST /power_page { case, ops }`）。省略時のケース解決は
/// 既存の解析入口と同じ。バジェット（`BudgetRequest`）と同じ形でケースと下書きの `ops` を受ける。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct PowerPageQuery {
    /// 解析ケースの ID（`cases/<ID>.toml`）。省略時は各解析と同じ規則で決める
    pub case: Option<String>,
    /// 保存していない編集。書き込まずに当てた設計で組み立てる（空なら保存済みの設計。`GET` と同じ）
    pub ops: Vec<fune_ops::Op>,
}
