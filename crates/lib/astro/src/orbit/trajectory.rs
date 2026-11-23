//! 軌道の一般化: 「時刻の関数としての状態」と「繰り返しの長さ」だけを共通の入口にする。
//!
//! 解析の多くはケプラー要素そのものを読まず、時刻での状態・中心天体・元期・周期（繰り返しの長さ）しか使わない。
//! その入口が [`Trajectory`]（どこから来る軌道か）と [`Cycle`]（周期か、1 回限りの期間か）。
//! 今は [`Trajectory::Elements`]（[`OrbitSpec`]）だけで、関数はすべて [`OrbitSpec`] の関数にそのまま渡す
//! （数値は変わらない）。ほかの source（状態ベクトル・回転座標系の軌跡・表）は、この列挙型に変種を足し、
//! 下の各メソッドに分岐を足すだけで増やせる（呼び出し側は変えない）。
//! ケプラー要素が本当に要る処理は [`Trajectory::elements`] を通して取り、無ければ対象外として扱う。

use super::{OrbitSpec, OrbitState, OrbitTrack};
use crate::AstroError;
use crate::ephem::CentralBody;
use crate::time::Epoch;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// 軌道の繰り返しの長さ。周期の置き換え。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cycle {
    /// 周期的な軌道（1 周回の長さ [s]）。
    Periodic { period_s: f64 },
    /// 繰り返さない軌道（飛行時間や指定した期間 [s]）。
    OneOff { span_s: f64 },
}

impl Cycle {
    /// 繰り返しの 1 単位の長さ [s]（周期、または 1 回限りの期間）。
    pub fn length_s(&self) -> f64 {
        match *self {
            Self::Periodic { period_s } => period_s,
            Self::OneOff { span_s } => span_s,
        }
    }

    /// 周期的なら周期 [s]。1 回限りなら `None`。
    pub fn period_s(&self) -> Option<f64> {
        match *self {
            Self::Periodic { period_s } => Some(period_s),
            Self::OneOff { .. } => None,
        }
    }
}

/// 解析に渡す軌道。時刻の関数としての状態と中心天体・元期・[`Cycle`] を共通の入口として持つ。
///
/// JSON では、`source` が無ければ [`Trajectory::Elements`] と読み、`Elements` は [`OrbitSpec`] と同じ形（タグ無し）で
/// 書く。このため、軌道を [`OrbitSpec`] で書いていた既存の入力はそのまま読め、書き出しも変わらない。
/// 将来の source は `source` のタグで区別する。
#[derive(Debug, Clone)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
// 今は Elements だけで、JSON の形は OrbitSpec と同じ。変種が増えるときにこの指定を外す。
#[cfg_attr(feature = "ts", ts(as = "OrbitSpec"))]
pub enum Trajectory {
    /// ケプラー要素（今の軌道）。
    Elements(OrbitSpec),
}

impl From<OrbitSpec> for Trajectory {
    fn from(spec: OrbitSpec) -> Self {
        Self::Elements(spec)
    }
}

impl OrbitSpec {
    /// 繰り返しの長さ。要素で書かれた軌道は周期的（Kepler 周期）。**周期から [`Cycle`] を作るのはここだけ**
    /// （[`Trajectory::cycle`] もこれを使う）。
    pub fn cycle(&self) -> Cycle {
        Cycle::Periodic {
            period_s: self.period_s(),
        }
    }
}

impl Trajectory {
    /// 元期（時刻 0 の時刻）。
    pub fn epoch(&self) -> Epoch {
        match self {
            Self::Elements(o) => o.epoch,
        }
    }

    /// 中心天体（位置・速度の原点、直下点・日陰・遮りの基準）。
    pub fn central(&self) -> CentralBody {
        match self {
            Self::Elements(o) => o.central,
        }
    }

    /// 元期から `t_s` 秒後の状態。多数の時刻が要るときは [`Trajectory::states_at`]。
    pub fn state_at(&self, t_s: f64) -> Result<OrbitState, AstroError> {
        match self {
            Self::Elements(o) => o.state_at(t_s),
        }
    }

    /// 昇順の時刻列 `times_s`（元期からの秒）での状態をまとめて求める。
    pub fn states_at(&self, times_s: &[f64]) -> Result<Vec<OrbitState>, AstroError> {
        match self {
            Self::Elements(o) => o.states_at(times_s),
        }
    }

    /// 元期から `span_s` 秒までの任意の時刻の状態を求める道具（[`OrbitSpec::track`]）。
    pub fn track(&self, span_s: f64, max_grid_s: f64) -> Result<OrbitTrack, AstroError> {
        match self {
            Self::Elements(o) => o.track(span_s, max_grid_s),
        }
    }

    /// 繰り返しの長さ。ケプラー要素は周期的（Kepler 周期）。
    pub fn cycle(&self) -> Cycle {
        match self {
            Self::Elements(o) => o.cycle(),
        }
    }

    /// ケプラー要素そのもの。要素で書かれた軌道だけが `Some`（β の当て込み・高度・減衰など、要素が要る処理の入口）。
    pub fn elements(&self) -> Option<&OrbitSpec> {
        match self {
            Self::Elements(o) => Some(o),
        }
    }

    /// [`Trajectory::elements`] の可変版（元期や要素を差し替える、要素の軌道だけの処理用）。
    pub fn elements_mut(&mut self) -> Option<&mut OrbitSpec> {
        match self {
            Self::Elements(o) => Some(o),
        }
    }
}

impl Serialize for Trajectory {
    /// `Elements` は [`OrbitSpec`] と同じ形（タグ無し）。
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Elements(o) => o.serialize(s),
        }
    }
}

impl<'de> Deserialize<'de> for Trajectory {
    /// `source` が無い、または `"elements"` なら [`OrbitSpec`] として読む。
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let mut value = serde_json::Value::deserialize(d)?;
        let source = match value.as_object_mut() {
            Some(map) => map.remove("source"),
            None => None,
        };
        match source.as_ref().map(serde_json::Value::as_str) {
            None | Some(Some("elements")) => OrbitSpec::deserialize(value)
                .map(Self::Elements)
                .map_err(D::Error::custom),
            Some(_) => Err(D::Error::custom(format!(
                "Unknown orbit source {}",
                source.unwrap_or_default()
            ))),
        }
    }
}

#[cfg(test)]
mod tests;
