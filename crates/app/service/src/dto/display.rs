//! 解析表示の葉の型（表示名・時系列）。
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 解析の出力の語彙の表示名（日本語と英語。`fune_service::analysis::labels` が引く）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct AnalysisLabel {
    pub label: String,
    pub label_en: String,
}

/// service が表示情報を添えた時系列チャンネル。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
pub struct DisplayChannel {
    /// 解析が返す SI の系列。
    #[serde(flatten)]
    pub channel: sat_analysis::spec::Channel,
    /// 表示名（語彙から引いたもの。`kind` と `subject` から作る）。
    pub label: String,
    /// 英語の表示名。
    pub label_en: String,
    /// 表示単位。
    pub display_unit: String,
    /// SI 値に掛ける表示倍率。
    pub display_scale: f64,
}

/// run result の JSON に含まれる表示用時系列。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
pub struct DisplaySeries {
    /// 元期からの経過秒。
    pub t: Vec<f64>,
    /// 表示情報を持つ系列。
    pub channels: Vec<DisplayChannel>,
}
