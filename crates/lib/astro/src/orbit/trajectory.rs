//! 軌道の一般化: 「時刻の関数としての状態」と「繰り返しの長さ」だけを共通の入口にする。
//!
//! 解析の多くはケプラー要素そのものを読まず、時刻での状態・中心天体・元期・周期（繰り返しの長さ）しか使わない。
//! その入口が [`Trajectory`]（どこから来る軌道か）と [`Cycle`]（周期か、1 回限りの期間か）。
//! [`Trajectory::Elements`]（[`OrbitSpec`]）の関数は [`OrbitSpec`] の関数にそのまま渡す（数値は変わらない）。
//! どの軌道にも、1 回限りの期間を持たせられる（[`Trajectory::with_span`]。[`Trajectory::Spanned`]）。期間を持つと
//! [`Cycle`] は [`Cycle::OneOff`] になり、中身の軌道の伝搬は変えない（日付つきの遷移の巡航のように、周期的でない弧を
//! 二体の軌道で表すときに使う）。状態ベクトルの軌道（[`Trajectory::State`]）は期間が必須で、同じ [`Cycle::OneOff`]・
//! [`Trajectory::with_span`]・`span_s` の書き方を使う（期間は [`StateSpec`] の欄で、`Spanned` で包まない）。
//! ほかの source（回転座標系の軌跡・表）は、この列挙型に変種を足し、
//! 下の各メソッドに分岐を足すだけで増やせる（呼び出し側は変えない）。
//! ケプラー要素が本当に要る処理は [`Trajectory::elements`] を通して取り、無ければ対象外として扱う
//! （期間を持つ軌道は、要素で書かれていても周期的でないので `None`。状態ベクトルの軌道も `None`）。

mod rebase;

use super::track::TABLE_STEPS_PER_ORBIT;
use super::{OrbitSpec, OrbitState, OrbitTrack, StateSpec};
use crate::AstroError;
use crate::ephem::CentralBody;
use crate::ode::step_offsets;
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
/// 要素以外の source は `source` のタグ（`"state"`）で区別する（[`TrajectoryRepr`]）。
#[derive(Debug, Clone)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
// JSON の形は TrajectoryRepr（要素は OrbitSpec に期間があれば `span_s` を足したもの、状態ベクトルは `source = "state"` を足したもの）。
#[cfg_attr(feature = "ts", ts(as = "TrajectoryRepr"))]
pub enum Trajectory {
    /// ケプラー要素（今の軌道）。
    Elements(OrbitSpec),
    /// 状態ベクトル（元期・原点の天体・位置・速度・力の設定・期間）を既存の数値伝搬で伝搬する軌道。繰り返しは常に 1 回限り。
    State(StateSpec),
    /// 1 回限りの期間 `span_s` [s] を持つ軌道（[`Cycle::OneOff`]）。伝搬・中心天体・元期は `orbit` のまま。
    Spanned { orbit: Box<Trajectory>, span_s: f64 },
}

/// JSON の形。`source` が無い・`"elements"` なら要素（[`ElementsRepr`]）、`"state"` なら状態ベクトル（[`StateRepr`]）。
#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(untagged)]
// 型は TS の型の名前（`ts(as = ...)`）に使うだけで、Rust の側からは使わない
#[cfg_attr(not(feature = "ts"), allow(dead_code))]
pub enum TrajectoryRepr {
    State(StateRepr),
    Elements(ElementsRepr),
}

/// 状態ベクトルの軌道の JSON の形（[`StateSpec`] の欄に `source = "state"` を足したもの）。
#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StateRepr {
    #[cfg_attr(feature = "ts", ts(type = "\"state\""))]
    pub source: String,
    #[serde(flatten)]
    pub spec: StateSpec,
}

/// 要素の軌道の JSON の形（[`OrbitSpec`] の欄に、期間があれば `span_s` を並べる）。
#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ElementsRepr {
    #[serde(flatten)]
    pub orbit: OrbitSpec,
    /// 1 回限りの期間 [s]（周期的な軌道は書かない）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span_s: Option<f64>,
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
    /// 1 回限りの期間 `span_s` [s] を持たせる（すでにあれば置き換える）。期間は正の有限の値（検査は呼ぶ側）。
    /// 状態ベクトルの軌道は期間が自分の欄なので、その欄を置き換える。
    pub fn with_span(self, span_s: f64) -> Self {
        match self {
            Self::Spanned { orbit, .. } => Self::Spanned { orbit, span_s },
            Self::State(spec) => Self::State(StateSpec { span_s, ..spec }),
            orbit => Self::Spanned {
                orbit: Box::new(orbit),
                span_s,
            },
        }
    }

    /// 1 回限りの期間 [s]（周期的な軌道は `None`）。
    pub fn span_s(&self) -> Option<f64> {
        match self {
            Self::Elements(_) => None,
            Self::State(s) => Some(s.span_s),
            Self::Spanned { span_s, .. } => Some(*span_s),
        }
    }

    /// 元期（時刻 0 の時刻）。
    pub fn epoch(&self) -> Epoch {
        match self {
            Self::Elements(o) => o.epoch,
            Self::State(s) => s.epoch,
            Self::Spanned { orbit, .. } => orbit.epoch(),
        }
    }

    /// 中心天体（位置・速度の原点、直下点・日陰・遮りの基準）。
    pub fn central(&self) -> CentralBody {
        match self {
            Self::Elements(o) => o.central,
            Self::State(s) => s.central,
            Self::Spanned { orbit, .. } => orbit.central(),
        }
    }

    /// 元期から `t_s` 秒後の状態。多数の時刻が要るときは [`Trajectory::states_at`]。
    pub fn state_at(&self, t_s: f64) -> Result<OrbitState, AstroError> {
        match self {
            Self::Elements(o) => o.state_at(t_s),
            Self::State(_) => self.states_at(&[t_s])?.pop().ok_or_else(|| {
                AstroError::InvalidInput("Numerical propagation returned no result".into())
            }),
            Self::Spanned { orbit, .. } => orbit.state_at(t_s),
        }
    }

    /// 昇順の時刻列 `times_s`（元期からの秒）での状態をまとめて求める。
    pub fn states_at(&self, times_s: &[f64]) -> Result<Vec<OrbitState>, AstroError> {
        match self {
            Self::Elements(o) => o.states_at(times_s),
            Self::State(s) => s.states_at(times_s),
            Self::Spanned { orbit, .. } => orbit.states_at(times_s),
        }
    }

    /// 元期から `span_s` 秒までの任意の時刻の状態を求める道具（[`OrbitSpec::track`]）。
    /// 状態ベクトルの軌道は、1 回の積分で格子（刻み `max_grid_s` と [`StateSpec::table_step_s`] の小さい方）の表を作り、
    /// 格子の間をエルミート補間する（数値伝搬の要素の軌道と同じ表）。
    pub fn track(&self, span_s: f64, max_grid_s: f64) -> Result<OrbitTrack, AstroError> {
        match self {
            Self::Elements(o) => o.track(span_s, max_grid_s),
            Self::State(s) => {
                let t = step_offsets(span_s, max_grid_s.min(s.table_step_s()));
                let states = s.states_at(&t)?;
                Ok(OrbitTrack::from_table(t, states))
            }
            Self::Spanned { orbit, .. } => orbit.track(span_s, max_grid_s),
        }
    }

    /// 任意の時刻の表（[`Trajectory::track`]）の格子の刻み [s] の目安。繰り返しの 1 単位（周期、または期間）の
    /// [`TABLE_STEPS_PER_ORBIT`] 分の 1。状態ベクトルの軌道は近点での 1 周の時間から（[`StateSpec::table_step_s`]）。
    pub fn table_step_s(&self) -> f64 {
        match self {
            Self::State(s) => s.table_step_s(),
            _ => self.cycle().length_s() / TABLE_STEPS_PER_ORBIT,
        }
    }

    /// 繰り返しの長さ。ケプラー要素は周期的（Kepler 周期）、期間を持つ軌道・状態ベクトルの軌道は 1 回限り。
    pub fn cycle(&self) -> Cycle {
        match self {
            Self::Elements(o) => o.cycle(),
            Self::State(s) => Cycle::OneOff { span_s: s.span_s },
            Self::Spanned { span_s, .. } => Cycle::OneOff { span_s: *span_s },
        }
    }

    /// 高度 [m] = 長半径 − 中心天体の赤道半径（[`OrbitSpec::altitude_m`]。楕円では平均の高さ）。
    /// 期間を持つ軌道は中身の二体の軌道の値。長半径を持たない軌道（状態ベクトルの軌道）は `None`。
    pub fn altitude_m(&self) -> Option<f64> {
        match self {
            Self::Elements(o) => Some(o.altitude_m()),
            Self::State(_) => None,
            Self::Spanned { orbit, .. } => orbit.altitude_m(),
        }
    }

    /// ケプラー要素そのもの。周期的な、要素で書かれた軌道だけが `Some`（β の当て込み・高度・減衰など、要素が要る処理の入口）。
    pub fn elements(&self) -> Option<&OrbitSpec> {
        match self {
            Self::Elements(o) => Some(o),
            Self::State(_) | Self::Spanned { .. } => None,
        }
    }

    /// [`Trajectory::elements`] の可変版（元期や要素を差し替える、要素の軌道だけの処理用）。
    pub fn elements_mut(&mut self) -> Option<&mut OrbitSpec> {
        match self {
            Self::Elements(o) => Some(o),
            Self::State(_) | Self::Spanned { .. } => None,
        }
    }
}

impl Serialize for Trajectory {
    /// `Elements` は [`OrbitSpec`] と同じ形（タグ無し）。期間を持つ軌道は、その欄に `span_s` を足した形。
    /// `State` は [`StateSpec`] の欄に `source = "state"` を足した形。
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Elements(o) => o.serialize(s),
            Self::State(spec) => StateRepr {
                source: "state".to_owned(),
                spec: spec.clone(),
            }
            .serialize(s),
            Self::Spanned { orbit, span_s } => match orbit.as_ref() {
                Self::Elements(o) => ElementsRepr {
                    orbit: o.clone(),
                    span_s: Some(*span_s),
                }
                .serialize(s),
                // 期間は 1 段だけ（`with_span` が置き換える）。入れ子は作らない
                Self::State(_) | Self::Spanned { .. } => {
                    Err(serde::ser::Error::custom("Nested trajectory span"))
                }
            },
        }
    }
}

impl<'de> Deserialize<'de> for Trajectory {
    /// `source` が無い、または `"elements"` なら [`OrbitSpec`] として、`"state"` なら [`StateSpec`] として読む。
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let mut value = serde_json::Value::deserialize(d)?;
        let source = value.as_object_mut().and_then(|map| map.remove("source"));
        if source.as_ref().and_then(serde_json::Value::as_str) == Some("state") {
            return StateSpec::deserialize(value)
                .map(Self::State)
                .map_err(D::Error::custom);
        }
        let span = value.as_object_mut().and_then(|map| map.remove("span_s"));
        let span_s = match span {
            None | Some(serde_json::Value::Null) => None,
            Some(v) => Some(v.as_f64().ok_or_else(|| {
                D::Error::custom("The span of an orbit (span_s) must be a number of seconds")
            })?),
        };
        match source.as_ref().map(serde_json::Value::as_str) {
            None | Some(Some("elements")) => OrbitSpec::deserialize(value)
                .map(|o| match span_s {
                    Some(span_s) => Self::Elements(o).with_span(span_s),
                    None => Self::Elements(o),
                })
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
