//! AOCS のモードカード。集計・解析済みの値と既存の設定編集のパス。
use super::QuantityView;
use fune_model::role::Role;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 1 モードの姿勢・機器・解析。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AocsModeView {
    pub attitude: AocsAttitudeView,
    pub devices: Vec<AocsDeviceGroup>,
    pub eclipse_devices: Vec<AocsDeviceGroup>,
    pub momentum_per_orbit: Option<QuantityView>,
    pub unloads_per_day: Option<QuantityView>,
    /// アンロード時間（解析が出した手段ごと。どちらも無ければ空）
    pub unload_times: Vec<AocsUnloadTime>,
    pub rw: AocsAnalysisState,
    pub unloading: AocsAnalysisState,
    pub sensor_fov: AocsAnalysisState,
}

/// アンロードの手段。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum AocsUnloadMeans {
    /// 磁気トルカ
    Mtq,
    /// スラスタ（RCS。最悪の向き）
    Rcs,
}

/// 1 つの手段でのアンロード時間。表示名は API が返す。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AocsUnloadTime {
    pub means: AocsUnloadMeans,
    pub label: String,
    pub label_en: String,
    pub time: QuantityView,
}

/// 解析の状態。入力不足や失敗を OK やゼロで埋めない。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AocsAnalysisState {
    pub status: super::AnalysisRunStatus,
    pub verdict: Option<sat_analysis::spec::Verdict>,
    pub missing: Vec<fune_projection::MissingInput>,
    pub error: Option<String>,
}

/// 姿勢則の設定と実効軸。編集は全て SetText を使う（フェーズの上書きの欄は `fune phase set-field` と同じ `POST /ops/from_command` の `phase_field`）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AocsAttitudeView {
    pub law: Option<fune_model::design::AttitudeLawKind>,
    pub fields: Vec<AocsLawField>,
    pub axes: Vec<AocsAxisView>,
}

/// 設定欄のパスと、CLI と同じ書式の現在値。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AocsLawField {
    pub field: String,
    pub path: String,
    pub text: String,
}

/// 主軸・副軸と向ける先。ベクトルの書式も Rust で作る。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AocsAxisView {
    pub body: String,
    pub target: String,
    pub target_en: String,
}

/// 役割ごとの機器。台数と状態は counted_states で評価済み。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AocsDeviceGroup {
    /// 役割（役割の無い種別の部品をまとめた「その他」は None）
    pub role: Option<Role>,
    pub label: String,
    pub label_en: Option<String>,
    pub active_count: u32,
    pub total_count: u32,
    pub devices: Vec<AocsDeviceView>,
}

/// 部品の稼働状態（不明は None）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AocsDeviceView {
    pub part: String,
    pub active: Option<bool>,
    pub states: Vec<AocsDeviceState>,
}

/// 状態とその台数・時間割合。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AocsDeviceState {
    pub state: String,
    pub count: u32,
    pub fraction: QuantityView,
}
