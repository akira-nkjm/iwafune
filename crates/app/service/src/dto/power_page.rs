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
    /// データを見る・不足入力・設定に使う元の解析と表示名。
    pub analyses: Vec<PowerAnalysisView>,
    /// 役割別の部品と表示属性。
    pub parts: Vec<PowerPartsGroup>,
}

/// 選択するモードと、そのモードの表示。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PowerModeView {
    pub id: String,
    pub label: String,
    pub verdict: Option<Verdict>,
    pub sunlit: Option<PowerBudgetView>,
    pub eclipse: Option<PowerBudgetView>,
    pub bars: Vec<PowerBulletView>,
    pub charts: Vec<PowerChartView>,
}

/// 日照中（または蝕中）の電力の行き先を 1 本の積み上げ棒にした表示。
/// 幅 `fraction` は、同じ照明条件（日照・蝕）の全モードで共通の最大に対する割合（0〜1）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PowerBudgetView {
    /// 供給（日照: 太陽電池の日照中の平均発電、蝕: 電池からの放電）
    pub supply: QuantityView,
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

/// 許容帯と設計値・限界のマーカー（位置は 0〜100）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PowerBulletView {
    pub key: String,
    pub label: AnalysisLabel,
    pub value: Option<QuantityView>,
    pub limit: QuantityView,
    pub verdict: Option<Verdict>,
    pub band_start: f64,
    pub band_end: f64,
    pub marker: Option<f64>,
    pub limit_marker: f64,
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

/// 部品タブの役割と属性の定義。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PowerPartsGroup {
    pub key: String,
    pub label: AnalysisLabel,
    pub parts: Vec<super::PartRow>,
    pub attributes: Vec<String>,
}

/// 電源ページが収集した元の解析結果と定義。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PowerAnalysisView {
    pub entry: super::AnalysisEntry,
    pub result: super::AnalysisRunResult,
}

/// 電源ページの問い合わせ。省略時のケース解決は既存の解析入口に委ねる。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct PowerPageQuery {
    pub case: Option<String>,
}
