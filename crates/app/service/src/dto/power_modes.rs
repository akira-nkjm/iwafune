//! 電源のモードごとの表示定義（`SystemModeView.power`）。位置・比・判定・量はサービスが返す。
use super::{AnalysisLabel, DisplaySeries, QuantityView};
use sat_analysis::spec::{System, Verdict};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 1 モードの電力の表示（`SystemModeView.id` と同じモード ID）。
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
