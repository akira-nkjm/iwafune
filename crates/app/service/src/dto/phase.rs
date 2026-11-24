//! フェーズの表示（`GET /phases`・`GET /design` の `phases`・`fune phase ls|show`）と、
//! `fune phase add|set` に打った引数（`CommandText::PhaseAdd`・`PhaseSet`。読み方は `edit::phase`）。

use super::QuantityView;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// `fune phase add|set <ID> --label … --order 1 --start … --end … --duration "30 d" --modes SAFE,NOMINAL` の引数（文字列のまま）。
/// 書かなかった旗（省略・null・空の文字列）は書かない。量（`"30 d"`）と整数（`"1"`）を読むのは service。
/// 軌道（`[orbit]`）と姿勢則（`[law.<モード>]`）の欄はここには無い（`fune phase set-field`。`set` は今のまま残す）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(optional_fields = nullable)]
pub struct PhaseText {
    pub id: String,
    pub label: Option<String>,
    /// 並び順（整数。省略は 0）
    pub order: Option<String>,
    /// 始まる条件（自由文）
    pub start: Option<String>,
    /// 終わる条件（自由文）
    pub end: Option<String>,
    /// 期間（`"30 d"`）
    pub duration: Option<String>,
    /// 使うモード（カンマ区切り）
    pub modes: Option<String>,
    /// 解析ケースがこのフェーズで使う形態の ID。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configuration: Option<String>,
}

/// 軌道・姿勢則の上書きの欄 1 つ。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PhaseFieldView {
    /// `fune phase set-field` のパス（`orbit.altitude`・`law.NOMINAL.law`）
    pub path: String,
    /// 表（`orbit`・`law`）の中のキー（`altitude`・`NOMINAL.law`。表示用）
    pub key: String,
    /// 値の表示（量なら単位付きに整形）
    pub display: String,
    /// 値（設定の表と同じ形。設定の表の編集の欄がそのまま使う）
    pub value: super::ValueData,
    /// 編集の欄の初期値（`fune phase set-field` に打つのと同じ文字列）
    pub text: String,
    /// 選んで入れる欄の選択肢（軌道の `source`・`maneuver`。空なら自由入力。欄の一覧は API が出どころごとに返す）
    pub choices: Vec<fune_projection::Choice>,
}

/// フェーズが姿勢則を上書きしているモード 1 つ（`[law.<モード>]`）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PhaseLawView {
    pub mode: String,
    /// 書かれた欄（`law.<モード>.<キー>`）
    pub fields: Vec<PhaseFieldView>,
    /// 姿勢則の種類・軸の欄・実効軸（AOCS のモードの姿勢則と同じ形。欄のパスは `fune phase set-field` のパス）
    pub attitude: super::AocsAttitudeView,
}

/// フェーズ 1 つ（`(order, ID)` の順）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PhaseView {
    pub id: String,
    /// 並びの位置（1 から。`Design::ordered_phases`）
    pub position: u32,
    /// 並び順の値（ファイルの `order`）
    pub order: i64,
    /// 表示名（無ければ ID）
    pub label: String,
    pub start: Option<String>,
    pub end: Option<String>,
    pub duration: Option<QuantityView>,
    /// 使うモード
    pub modes: Vec<String>,
    /// 軌道を上書きしているか（`[orbit]`）
    pub orbit_overridden: bool,
    /// 上書きの軌道の書かれた欄（`[orbit]`。`orbit_overridden` のときだけ。出どころが使う欄（`source` と、その出どころの欄）は未入力でも並べる）
    pub orbit: Vec<PhaseFieldView>,
    /// 上書きの軌道が巡航の軌道を取る遷移のマヌーバ（`[orbit]` が `source = "transfer"` のとき）。一覧の「軌道は <マヌーバ> から」の表示用
    pub orbit_from: Option<String>,
    /// 姿勢則の上書き（モードの ID 順。`[law.<モード>]`）
    pub laws: Vec<PhaseLawView>,
    /// このフェーズを指している解析ケース
    pub cases: Vec<String>,
    /// 編集の欄の初期値（`fune phase set` に打つのと同じ文字列）
    pub text: PhaseText,
    /// 欄の名前（`PhaseText` の欄）→ 対象のパス（`phases.LEOP.duration`。診断の `DiagnosticView.path` と同じ）
    pub field_paths: BTreeMap<String, String>,
    pub file: String,
}
