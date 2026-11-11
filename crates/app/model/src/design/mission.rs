//! ミッションの入力（`mission/`）: 軌道・地上局・打ち上げ条件。serde の形がそのままファイルの形。
//!
//! 値はすべて任意（`Option`）。足りない入力は projection が `mission.orbit.altitude` のようなパスで列挙する。

use crate::enum_text::enum_text;
use crate::value::Quantity;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// `mission/` 全体。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
pub struct MissionSettings {
    /// `mission/orbit.toml`
    pub orbit: Option<OrbitSettings>,
    /// `mission/ground/<ID>.toml`
    pub ground: BTreeMap<String, GroundStationSettings>,
    /// `mission/launch.toml`
    pub launch: Option<LaunchSettings>,
}

enum_text! {
    /// 軌道の伝搬方法。
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
    #[ts(rename = "OrbitPropagatorKind")]
    pub enum PropagatorKind: "軌道の伝搬方法" {
        Kepler = "kepler",
        J2 = "j2",
        Numerical = "numerical",
    }
}

/// 軌道（`mission/orbit.toml`）。円軌道は `altitude`、一般の軌道は `semi_major_axis` 以下の要素で書く。
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
}

/// 地上局（`mission/ground/<ID>.toml`）。
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

/// 打ち上げ条件（`mission/launch.toml`）。
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
