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
    pub sunlit: Option<PowerFlowView>,
    pub eclipse: Option<PowerFlowView>,
    pub bars: Vec<PowerBulletView>,
    pub charts: Vec<PowerChartView>,
}

/// 流れ図（座標は表示用の固定座標系）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PowerFlowView {
    pub height: f64,
    pub nodes: Vec<PowerFlowNode>,
    pub edges: Vec<PowerFlowEdge>,
}

/// 流れ図の端点と量。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PowerFlowNode {
    pub id: String,
    pub label: AnalysisLabel,
    pub system: Option<System>,
    pub x: f64,
    pub y: f64,
    pub value: Option<QuantityView>,
}

/// 流れ図の矢印。太さは同じ図内で電力に比例する。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PowerFlowEdge {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub width: f64,
    pub system: Option<System>,
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
