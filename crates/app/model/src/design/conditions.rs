//! 前提条件の入力（`conditions/`）: 軌道・地上局・打ち上げ条件。serde の形がそのままファイルの形。
//!
//! 値はすべて任意（`Option`）。足りない入力は projection が `conditions.orbit.altitude` のようなパスで列挙する。

use crate::enum_text::enum_text;
use crate::value::Quantity;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// `conditions/` 全体。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
pub struct ConditionsSettings {
    /// `conditions/orbit.toml`
    pub orbit: Option<OrbitSettings>,
    /// `conditions/ground/<ID>.toml`
    pub ground: BTreeMap<String, GroundStationSettings>,
    /// `conditions/launch.toml`
    pub launch: Option<LaunchSettings>,
}

enum_text! {
    /// 軌道の伝搬方法。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
    #[ts(rename = "OrbitPropagatorKind")]
    pub enum PropagatorKind: "orbit propagation method" {
        Kepler = "kepler",
        J2 = "j2",
        Numerical = "numerical",
    }
}

enum_text! {
    /// 軌道の中心天体（`sat_astro::ephem::CentralBody` に射影する。μ・半径・環境の値は sat-astro と解析が持つ）。
    /// マヌーバの目標の天体（`target.body`）にも使う。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
    pub enum CentralBodyKind: "central body" {
        Earth = "earth",
        Moon = "moon",
        Sun = "sun",
        Mercury = "mercury",
        Venus = "venus",
        Mars = "mars",
        Jupiter = "jupiter",
        Saturn = "saturn",
        Uranus = "uranus",
        Neptune = "neptune",
    }
}

/// 軌道（`conditions/orbit.toml`）。円軌道は `altitude`、一般の軌道は `semi_major_axis` 以下の要素で書く。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct OrbitSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// 元期（ISO 8601、UTC。例: `"2027-04-01T00:00:00Z"`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epoch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub propagator: Option<PropagatorKind>,
    /// 中心天体（省略時は地球）。高度はこの天体の赤道半径から数える
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub central: Option<CentralBodyKind>,
    /// 円軌道の高度
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub altitude: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semi_major_axis: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eccentricity: Option<f64>,
    /// 傾斜角（太陽同期なら省略でき、高度から求める）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inclination: Option<Quantity>,
    /// 太陽同期軌道か
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sun_synchronous: Option<bool>,
    /// 降交点の地方時（`"10:30"`）。RAAN の代わりに書ける
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ltdn: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raan: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arg_perigee: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_anomaly: Option<Quantity>,
    /// ミッション期間（時間）。
    #[serde(default)]
    pub duration: Option<Quantity>,
    /// 軌道の季節解析の設定。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eclipse_season: Option<EclipseSeasonSettings>,
    /// 大気抵抗による高度低下と軌道維持の設定（`[decay]`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decay: Option<OrbitDecaySettings>,
}

/// 季節解析の設定（刻みの既定値は解析が持つ）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct EclipseSeasonSettings {
    /// 季節の標本の間隔（時間）。
    #[serde(default)]
    pub step: Option<Quantity>,
}

/// 高度低下と軌道維持の設定（`conditions/orbit.toml` の `[decay]`）。どれも任意で、既定値は解析が持つ。
/// 抗力係数は `analysis.aocs.drag_coefficient`、質量は部品、外形は構体の `size` から取る。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct OrbitDecaySettings {
    /// 積分の期間の上限（時間）。
    #[serde(default)]
    pub span: Option<Quantity>,
    /// 再突入とみなす高度（長さ）。
    #[serde(default)]
    pub reentry_altitude: Option<Quantity>,
    /// 抗力の面積（面積。無ければ構体の外形の箱を向きでならした面積）。
    #[serde(default)]
    pub drag_area: Option<Quantity>,
    /// 軌道維持の高度の帯（長さ。無ければ抗力を連続して打ち消す）。
    #[serde(default)]
    pub band: Option<Quantity>,
    /// 寿命の上限（時間。廃棄の要求）。
    #[serde(default)]
    pub max_lifetime: Option<Quantity>,
    /// 以前の軌道維持の推薬（質量）。**使わない**（軌道維持の推薬は推進系の解析 `propulsion.propellant` の使える推薬）。
    /// 古い設計を読めるように残し、書かれていれば `check` が外すよう warning（`setting.moved`）を出す。新しく書かない。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub propellant: Option<Quantity>,
}

/// 地上局（`conditions/ground/<ID>.toml`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct GroundStationSettings {
    /// ファイル名から決まる
    #[serde(default)]
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latitude: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub longitude: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub altitude: Option<Quantity>,
    /// 運用できる最低仰角
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_elevation: Option<Quantity>,
    /// 帯域ごとの地上側の性能（`[link.uhf]`・`[link.s_band]`）
    #[serde(default, rename = "link", skip_serializing_if = "BTreeMap::is_empty")]
    pub links: BTreeMap<String, GroundLinkSettings>,
}

/// 地上局の 1 つの帯域の性能。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct GroundLinkSettings {
    /// アンテナ利得（dBi）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub antenna_gain: Option<Quantity>,
    /// 受信系の雑音温度
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub noise_temperature: Option<Quantity>,
    /// G/T（dB/K。sat-units は `dB/K` を式にできないので単位は `dB` と書く）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub g_over_t: Option<Quantity>,
    /// アップリンクの送信電力
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tx_power: Option<Quantity>,
    /// 給電線などの損失（dB）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_loss: Option<Quantity>,
}

/// 打ち上げ条件（`conditions/launch.toml`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct LaunchSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vehicle: Option<String>,
    /// 準静的荷重（機軸方向。単位は `g0`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub load_axial: Option<Quantity>,
    /// 準静的荷重（機軸に直交する方向）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub load_lateral: Option<Quantity>,
    /// 要求される最低固有振動数（機軸方向）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_frequency_axial: Option<Quantity>,
    /// 要求される最低固有振動数（直交方向）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_frequency_lateral: Option<Quantity>,
    /// 安全率（降伏）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safety_factor_yield: Option<f64>,
    /// 安全率（終局）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safety_factor_ultimate: Option<f64>,
}

enum_text! {
    /// 前提条件の解析の話題。解析ライブラリとの綴りは projection で照合する。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
    pub enum ConditionTopic: "condition topic" {
        Orbit = "orbit",
    }
}

impl ConditionTopic {
    /// 話題の設定ファイル全体のパス（その中に解析ごとの表を置く）。
    pub fn setting_path(self) -> crate::path::SettingPath {
        crate::path::SettingPath::conditions(&[self.as_str()])
    }

    /// 日本語の表示名。
    pub fn label(self) -> &'static str {
        match self {
            Self::Orbit => "軌道",
        }
    }

    /// 英語の表示名。
    pub fn label_en(self) -> &'static str {
        match self {
            Self::Orbit => "Orbit",
        }
    }
}
