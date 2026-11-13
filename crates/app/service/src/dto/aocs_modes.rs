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
    pub mtq_unload_time: Option<QuantityView>,
    pub rw: AocsAnalysisState,
    pub unloading: AocsAnalysisState,
    pub sensor_fov: AocsAnalysisState,
}

/// 解析の状態。入力不足や失敗を OK やゼロで埋めない。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AocsAnalysisState {
    pub status: super::AnalysisRunStatus,
    pub verdict: Option<sat_analysis::spec::Verdict>,
    pub missing: Vec<fune_projection::MissingInput>,
    pub error: Option<String>,
}

/// 姿勢則の設定と実効軸。編集は全て SetText を使う。
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
    pub role: Role,
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
