//! 天体の型と定数（暦の位置計算には依存しない）。
use crate::constants::*;
use serde::{Deserialize, Serialize};
/// 天体。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Body {
    Sun,
    Moon,
    Mercury,
    Venus,
    Mars,
    Jupiter,
    Saturn,
    Uranus,
    Neptune,
}

impl Body {
    /// 重力定数 μ [m³/s²]。
    pub fn gm(self) -> f64 {
        match self {
            Body::Sun => GM_SUN,
            Body::Moon => GM_MOON,
            Body::Mercury => GM_MERCURY,
            Body::Venus => GM_VENUS,
            Body::Mars => GM_MARS,
            Body::Jupiter => GM_JUPITER,
            Body::Saturn => GM_SATURN,
            Body::Uranus => GM_URANUS,
            Body::Neptune => GM_NEPTUNE,
        }
    }
}

/// 軌道の中心天体。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum CentralBody {
    #[default]
    Earth,
    Moon,
    Sun,
    Mercury,
    Venus,
    Mars,
    Jupiter,
    Saturn,
    Uranus,
    Neptune,
}

impl CentralBody {
    /// 暦の天体（地球は暦の原点なので `None`）。
    pub fn as_body(self) -> Option<Body> {
        match self {
            CentralBody::Earth => None,
            CentralBody::Moon => Some(Body::Moon),
            CentralBody::Sun => Some(Body::Sun),
            CentralBody::Mercury => Some(Body::Mercury),
            CentralBody::Venus => Some(Body::Venus),
            CentralBody::Mars => Some(Body::Mars),
            CentralBody::Jupiter => Some(Body::Jupiter),
            CentralBody::Saturn => Some(Body::Saturn),
            CentralBody::Uranus => Some(Body::Uranus),
            CentralBody::Neptune => Some(Body::Neptune),
        }
    }

    /// 重力定数 μ [m³/s²]。
    pub fn gm(self) -> f64 {
        self.as_body().map_or(MU_EARTH, Body::gm)
    }

    /// 赤道半径 [m]（J2 の基準半径。高度の基準にも使う）。
    pub fn radius_m(self) -> f64 {
        match self {
            CentralBody::Earth => R_EARTH,
            CentralBody::Moon => R_MOON,
            CentralBody::Sun => R_SUN,
            CentralBody::Mercury => R_MERCURY,
            CentralBody::Venus => R_VENUS,
            CentralBody::Mars => R_MARS,
            CentralBody::Jupiter => R_JUPITER,
            CentralBody::Saturn => R_SATURN,
            CentralBody::Uranus => R_URANUS,
            CentralBody::Neptune => R_NEPTUNE,
        }
    }

    /// 親の天体（この天体が回っている天体）。月は地球、地球と惑星は太陽、太陽は `None`。
    /// パッチドコニックの遷移（`transfer::patched_conic`）が親子・兄弟の関係を引く。
    pub fn parent(self) -> Option<CentralBody> {
        match self {
            CentralBody::Sun => None,
            CentralBody::Moon => Some(CentralBody::Earth),
            _ => Some(CentralBody::Sun),
        }
    }

    /// 設計で使う太陽距離の慣例値 [m]: 地球と月（地球と同じ日心距離）は 1 AU。それ以外は `None`
    /// （日心距離の変化が大きいので、解析が太陽距離か太陽光の強さを入力として求める）。
    pub fn nominal_sun_distance_m(self) -> Option<f64> {
        match self {
            CentralBody::Earth | CentralBody::Moon => Some(AU),
            _ => None,
        }
    }

    /// J2（[`CentralBody::radius_m`] を基準半径とする）。
    pub fn j2(self) -> f64 {
        match self {
            CentralBody::Earth => J2_EARTH,
            CentralBody::Moon => J2_MOON,
            CentralBody::Sun => J2_SUN,
            CentralBody::Mercury => J2_MERCURY,
            CentralBody::Venus => J2_VENUS,
            CentralBody::Mars => J2_MARS,
            CentralBody::Jupiter => J2_JUPITER,
            CentralBody::Saturn => J2_SATURN,
            CentralBody::Uranus => J2_URANUS,
            CentralBody::Neptune => J2_NEPTUNE,
        }
    }
}
