//! マヌーバの表示（`GET /maneuvers`・`GET /design` の `maneuvers`・`fune maneuver ls`）と、
//! `fune maneuver add|set` に打った引数（`CommandText::ManeuverAdd`・`ManeuverSet`。読み方は `edit::maneuver`）。

use super::QuantityView;
use fune_model::design::{CentralBodyKind, ManeuverCategory, ManeuverMethod};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// `fune maneuver add|set <ID> --category … --method … --altitude "600 km" …` の引数（文字列のまま）。
/// 書かなかった旗（省略・null・空の文字列）は書かない。量（`"600 km"`）と真偽（`"true"`）を読むのは service。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(optional_fields = nullable)]
pub struct ManeuverText {
    pub id: String,
    /// `insertion` / `station_keeping` / `unloading` / `deorbit` / `other`
    pub category: String,
    /// `impulsive` / `finite_burn` / `low_thrust` / `propagated`
    pub method: String,
    pub label: Option<String>,
    /// スラスタの部品 ID
    pub thruster: Option<String>,
    /// 前のマヌーバの ID
    pub after: Option<String>,
    /// バイエリプティック遷移の中間の遠地点の高度（`"20000 km"`）
    pub via_altitude: Option<String>,
    /// 直接書く ΔV（`"15 m/s"`。軌道修正・軌道維持・マージンの余裕。軌道は変えず、目標の軌道・中間の遠地点・日付とは一緒に書かない）
    pub delta_v: Option<String>,
    /// 目標の中心天体（`moon`・`mars` など。書くと天体間の遷移で、高度はその天体から）
    pub body: Option<String>,
    /// 目標の円軌道の高度（`"600 km"`）
    pub altitude: Option<String>,
    pub perigee_altitude: Option<String>,
    pub apogee_altitude: Option<String>,
    /// 目標の傾斜角（`"97.8 deg"`）
    pub inclination: Option<String>,
    /// 日照中だけ噴射する（`"true"` / `"false"`）
    pub sunlit_only: Option<String>,
    /// 噴射するモード
    pub mode: Option<String>,
    /// デューティ（`"0.5"`。0 より大きく 1 以下）
    pub duty: Option<String>,
    /// 推力の向き（`tangential` / `inertial`）
    pub steering: Option<String>,
    /// 日付つきの惑星間遷移の出発の日（ISO 8601、UTC。`"2028-12-10T00:00:00Z"`。到着の日と一緒に書く。目標の天体が要る）
    pub depart: Option<String>,
    /// 日付つきの惑星間遷移の到着の日（出発より後）
    pub arrive: Option<String>,
    /// 出発の噴射を行うもの（`spacecraft` / `launcher`。書かなければ衛星。日付と一緒にだけ書ける）
    pub departure_by: Option<String>,
    /// 途中のスイングバイ（`"venus@2029-02-01T00:00:00Z"`、`,min_altitude=400 km`・`,powered=false` を続けられる。繰り返す旗。
    /// 日付と一緒にだけ書ける。GUI の編集では、書かれたものをそのまま送り返す）
    #[ts(as = "Option<Vec<String>>")]
    pub flybys: Vec<String>,
}

/// 目標の軌道の要素 1 つ。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ManeuverTargetView {
    /// 要素のキー（`altitude`・`perigee_altitude`・`apogee_altitude`・`inclination`）
    pub key: String,
    pub value: QuantityView,
}

/// マヌーバ 1 つ（実行の順）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ManeuverView {
    pub id: String,
    /// 実行の順（1 から。`Design::ordered_maneuvers`）
    pub order: u32,
    /// 表示名（無ければ ID）
    pub label: String,
    pub category: ManeuverCategory,
    pub method: ManeuverMethod,
    pub thruster: Option<String>,
    pub after: Option<String>,
    /// 目標の中心天体（天体間の遷移。書かなければ同じ天体のまわり）
    pub body: Option<CentralBodyKind>,
    pub via_altitude: Option<QuantityView>,
    /// 直接書かれた ΔV（余裕。書かなければ目標の軌道から計算する）
    pub delta_v: Option<QuantityView>,
    /// 書かれた目標の要素（宣言の順）
    pub target: Vec<ManeuverTargetView>,
    /// 書かれた制約のキー（粗い段階の解析は無視する）
    pub constraints: Vec<String>,
    /// 編集の欄の初期値（`fune maneuver set` に打つのと同じ文字列）
    pub text: ManeuverText,
    /// 欄の名前（`ManeuverText` の欄）→ 対象のパス（`maneuvers.RAISE.thruster`。診断の `DiagnosticView.path` と同じ）
    pub field_paths: BTreeMap<String, String>,
    pub file: String,
}
