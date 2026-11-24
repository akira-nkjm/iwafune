//! [`Trajectory`] の JSON の形（読み書き）。**軌道の source ごとの JSON の綴りを決めるのはここだけ**。
//!
//! `source` が無い・`"elements"` なら要素（[`ElementsRepr`]。[`OrbitSpec`] と同じ形で、期間があれば `span_s` を足す）、
//! `"state"` なら状態ベクトル（[`StateRepr`]）、`"libration"` なら回転座標系の軌跡（[`LibrationRepr`]）、
//! `"arcs"` なら弧の連鎖（[`ArcsRepr`]。1 本の弧は `span_s` を持つ要素の軌道そのままの形で、2 本以上のときだけこの形になる）。

use super::{LIBRATION_SOURCE, Trajectory};
use crate::orbit::{OrbitSpec, StateSpec, SynodicSpec};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// `source` の値: 弧の連鎖（[`Trajectory::Arcs`]）。
const ARCS_SOURCE: &str = "arcs";

/// JSON の形。`source` が無い・`"elements"` なら要素（[`ElementsRepr`]）、`"state"` なら状態ベクトル（[`StateRepr`]）、
/// `"arcs"` なら弧の連鎖（[`ArcsRepr`]）。
#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(untagged)]
// 型は TS の型の名前（`ts(as = ...)`）に使うだけで、Rust の側からは使わない
#[cfg_attr(not(feature = "ts"), allow(dead_code))]
pub enum TrajectoryRepr {
    Libration(LibrationRepr),
    State(StateRepr),
    Arcs(ArcsRepr),
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

/// 回転座標系の軌跡の JSON の形（[`SynodicSpec`] の欄に `source = "libration"` を足したもの）。
#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LibrationRepr {
    #[cfg_attr(feature = "ts", ts(type = "\"libration\""))]
    pub source: String,
    #[serde(flatten)]
    pub spec: SynodicSpec,
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

/// 弧の連鎖の JSON の形（時間の順の弧。各弧は `span_s` を持つ要素の軌道で、元期は弧の始め。隣り合う弧は前の弧の終わりで接する）。
#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ArcsRepr {
    #[cfg_attr(feature = "ts", ts(type = "\"arcs\""))]
    pub source: String,
    pub arcs: Vec<ElementsRepr>,
}

/// 弧 1 本（期間を持つ要素の軌道）の JSON の形。要素の軌道でない弧は書けない。
fn arc_repr(arc: &Trajectory) -> Option<ElementsRepr> {
    match arc {
        Trajectory::Spanned { orbit, span_s } => match orbit.as_ref() {
            Trajectory::Elements(o) => Some(ElementsRepr {
                orbit: o.clone(),
                span_s: Some(*span_s),
            }),
            _ => None,
        },
        _ => None,
    }
}

impl Serialize for Trajectory {
    /// `Elements` は [`OrbitSpec`] と同じ形（タグ無し）。期間を持つ軌道は、その欄に `span_s` を足した形。
    /// `State` は [`StateSpec`] の欄に `source = "state"` を足した形。弧の連鎖は [`ArcsRepr`]（要素の弧だけ）。
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Elements(o) => o.serialize(s),
            Self::State(spec) => StateRepr {
                source: "state".to_owned(),
                spec: spec.clone(),
            }
            .serialize(s),
            Self::Synodic(spec) => LibrationRepr {
                source: LIBRATION_SOURCE.to_owned(),
                spec: spec.clone(),
            }
            .serialize(s),
            // 期間は 1 段だけ（`with_span` が置き換える）。入れ子は作らない
            Self::Spanned { .. } => arc_repr(self)
                .ok_or_else(|| serde::ser::Error::custom("Nested trajectory span"))?
                .serialize(s),
            Self::Arcs(chain) => ArcsRepr {
                source: ARCS_SOURCE.to_owned(),
                arcs: chain
                    .arcs()
                    .iter()
                    .map(|a| {
                        arc_repr(a).ok_or_else(|| {
                            serde::ser::Error::custom("Only Kepler arcs can be written in a chain")
                        })
                    })
                    .collect::<Result<_, _>>()?,
            }
            .serialize(s),
        }
    }
}

/// 期間つきの要素の軌道 1 本（`span_s` が無ければ周期的な軌道）。
fn elements_of(repr: ElementsRepr) -> Trajectory {
    match repr.span_s {
        Some(span_s) => Trajectory::Elements(repr.orbit).with_span(span_s),
        None => Trajectory::Elements(repr.orbit),
    }
}

impl<'de> Deserialize<'de> for Trajectory {
    /// `source` が無い、または `"elements"` なら [`OrbitSpec`] として、`"state"` なら [`StateSpec`] として、
    /// `"libration"` なら [`SynodicSpec`] として、`"arcs"` なら弧の連鎖として読む（連鎖は弧のつながりを検査する）。
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let mut value = serde_json::Value::deserialize(d)?;
        let source = value.as_object_mut().and_then(|map| map.remove("source"));
        let tag = source.as_ref().and_then(serde_json::Value::as_str);
        if tag == Some("state") {
            return StateSpec::deserialize(value)
                .map(Self::State)
                .map_err(D::Error::custom);
        }
        if tag == Some(LIBRATION_SOURCE) {
            return SynodicSpec::deserialize(value)
                .map(Self::Synodic)
                .map_err(D::Error::custom);
        }
        if tag == Some(ARCS_SOURCE) {
            return chain_of(value).map_err(D::Error::custom);
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

/// `source = "arcs"` の値（`source` を除いたもの）から連鎖を作る。どの弧にも `span_s` が要る。
fn chain_of(value: serde_json::Value) -> Result<Trajectory, String> {
    #[derive(Deserialize)]
    struct Arcs {
        arcs: Vec<ElementsRepr>,
    }
    let parsed: Arcs = serde_json::from_value(value).map_err(|e| e.to_string())?;
    if parsed.arcs.iter().any(|a| a.span_s.is_none()) {
        return Err("Every arc of a chain needs span_s".to_owned());
    }
    Trajectory::chain(parsed.arcs.into_iter().map(elements_of).collect()).map_err(|e| e.to_string())
}
