//! ミッション幾何: 探査機の太陽距離・地球距離・太陽–地球–探査機の角（SEP）・太陽–探査機–地球の角（SPE）・片道の光の伝搬時間。
//! **これらの距離と角度を時刻の列で出すのはここだけ**（地球からの距離の範囲 `ground::earth_range`・太陽距離の極値
//! `sat_analysis::orbit::sun_distance`・回線バジェットの光の伝搬時間・解析 `orbit.geometry` はこの上に載っている）。
//!
//! 入口は [`Trajectory`]（`states_at`・`central`・`epoch` だけを使い、軌道の source には依存しない）。位置はすべて
//! 地心の ECI（J2000 平均赤道）に直して比べる: 探査機 = 中心天体の地心位置 + 中心天体からの相対位置、
//! 天体の位置は [`CentralBody::position_eci_m`]（地球は原点なので 0）。
//!
//! # 定義
//! - **天体 B までの距離** = 探査機から B の中心まで。中心天体が B のときは、中心天体の位置を足し引きせず軌道半径 `|r|` そのもの
//!   （太陽中心の軌道の太陽距離、地球を回る衛星の地心距離。暦の桁落ちを避け、0 の天体位置からも安全）。
//! - **SEP**（太陽–地球–探査機の角）= 地球の中心で、太陽の向きと探査機の向きがなす角。合（SEP ≈ 0°）と衝（≈ 180°）の目安。
//!   地球を回る衛星では地心から見た太陽と衛星の角で、1 周回ごとに 0°〜180° を巡る（合の角としては意味を持たない）。
//! - **SPE**（太陽–探査機–地球の角）= 探査機で、太陽の向きと地球の向きがなす角（アンテナ・パネルの幾何に使う位相角）。
//! - 一般の天体 B（[`body_views_at`]）では、SEP と SPE の「地球」を B に置き換えた角（[`BodyView`]）。B が太陽のときは角が決まらず `None`。
//! - 光の伝搬時間 = 距離 / 光速（片道。[`light_time_s`]）。
//! - 三角形（太陽・天体・探査機）の内角は、太陽での角を `θ_S` として `SEP + SPE + θ_S = π`。距離は余弦定理を満たす（テストで確かめる）。
//!
//! 極値は標本の最小・最大（[`extreme_indices`]）と、標本の前後の区間で黄金分割（`math::golden_section_min`）して詰めたもの
//! （[`extremes_by`]）の 2 つ。詰める区間では単峰とみなす（軌道の周回による細かい変動が標本の刻みより速い成分は、詰めても拾えない）。
//! 弧の連鎖の軌道（スイングバイのある巡航）は、境で位置が連続で速度が跳ぶので、距離・角は連続だが傾きが折れる。
//! 詰める区間は境で分け、境そのものも極値の候補にする（[`extremes_by_split`]、[`breakpoints_from`]）。

mod below;
mod extremes;

pub use below::time_below_s;
pub use extremes::{
    Extremes, Extremum, ExtremumKind, extreme_indices, extremes_by, extremes_by_split,
};

use crate::AstroError;
use crate::constants::C_LIGHT;
use crate::ephem::CentralBody;
use crate::math::Vec3;
use crate::orbit::{OrbitState, OrbitTrack, Trajectory};
use crate::time::Epoch;

/// 片道の光の伝搬時間 [s]（距離 [m] / 光速）。**距離 → 光の伝搬時間の換算はここだけ。**
pub fn light_time_s(distance_m: f64) -> f64 {
    distance_m / C_LIGHT
}

/// 探査機から見た天体 1 つの幾何。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyView {
    /// 探査機から天体の中心までの距離 [m]
    pub distance_m: f64,
    /// 片道の光の伝搬時間 [s]
    pub light_time_s: f64,
    /// 太陽との角。天体が太陽のときは決まらず `None`
    pub angles: Option<ViewAngles>,
}

/// 天体 B と太陽と探査機の三角形の、B と探査機での角。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewAngles {
    /// B の中心での、太陽と探査機の角 [rad]（B = 地球なら SEP）
    pub at_body_rad: f64,
    /// 探査機での、太陽と B の角 [rad]（B = 地球なら SPE）
    pub at_probe_rad: f64,
}

/// 時刻 1 つの太陽・地球の幾何（[`geometry_at`]）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeometrySample {
    /// 標本の始め（`start`）からの経過 [s]
    pub t_s: f64,
    pub epoch: Epoch,
    /// 太陽の中心までの距離 [m]
    pub sun_distance_m: f64,
    /// 地球の中心までの距離 [m]
    pub earth_distance_m: f64,
    /// 太陽–地球–探査機の角 [rad]
    pub sep_rad: f64,
    /// 太陽–探査機–地球の角 [rad]
    pub spe_rad: f64,
    /// 地球までの片道の光の伝搬時間 [s]
    pub light_time_s: f64,
}

/// 幾何の量の種類（極値を取る量の選択）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Metric {
    SunDistance,
    EarthDistance,
    Sep,
    Spe,
    LightTime,
}

impl Metric {
    /// 標本の中のこの量の値（SI・rad）。
    pub fn of(self, s: &GeometrySample) -> f64 {
        match self {
            Self::SunDistance => s.sun_distance_m,
            Self::EarthDistance => s.earth_distance_m,
            Self::Sep => s.sep_rad,
            Self::Spe => s.spe_rad,
            Self::LightTime => s.light_time_s,
        }
    }
}

/// 1 時刻の位置（地心 ECI）。
struct Positions {
    /// 探査機
    probe: Vec3,
    /// 太陽
    sun: Vec3,
}

/// 時刻 `epoch` での軌道の状態 `state`（中心天体からの相対）を、地心の位置にする（`(中心天体からの相対位置, 地心の位置)`）。
fn positions(orbit: &Trajectory, epoch: Epoch, state: &OrbitState) -> (Vec3, Positions) {
    let probe = orbit.central().position_eci_m(epoch) + state.r_eci;
    let sun = CentralBody::Sun.position_eci_m(epoch);
    (state.r_eci, Positions { probe, sun })
}

/// `start` から `times_s`（昇順）の時刻での軌道の状態を、地心の位置にして返す（`(元期, 中心天体からの相対位置, 地心の位置)`）。
fn probe_states(
    orbit: &Trajectory,
    start: Epoch,
    times_s: &[f64],
) -> Result<Vec<(Epoch, Vec3, Positions)>, AstroError> {
    let states = orbit.states_at(&offsets(orbit, start, times_s))?;
    Ok(times_s
        .iter()
        .zip(&states)
        .map(|(&t, s)| {
            let epoch = start.plus_seconds(t);
            let (r, pos) = positions(orbit, epoch, s);
            (epoch, r, pos)
        })
        .collect())
}

/// 探査機（中心天体からの相対位置 `r`、地心位置 `pos.probe`）から見た天体 `body` の幾何。
fn view(
    central: CentralBody,
    epoch: Epoch,
    r: Vec3,
    pos: &Positions,
    body: CentralBody,
) -> Result<BodyView, AstroError> {
    let body_pos = body.position_eci_m(epoch);
    // 探査機から天体への向き。中心天体なら軌道半径そのもの（位置を足し引きしない）
    let to_body = if body == central {
        -r
    } else {
        body_pos - pos.probe
    };
    let distance_m = to_body.norm();
    let angles = if body == CentralBody::Sun {
        None
    } else {
        let (body_to_sun, body_to_probe, probe_to_sun) =
            (pos.sun - body_pos, -to_body, pos.sun - pos.probe);
        if [body_to_sun, body_to_probe, probe_to_sun, to_body]
            .iter()
            .any(|v| v.norm() == 0.0)
        {
            return Err(AstroError::InvalidInput(
                "The probe coincides with the centre of the body or the Sun, so the angles are undefined".into(),
            ));
        }
        Some(ViewAngles {
            at_body_rad: body_to_sun.angle_to(body_to_probe),
            at_probe_rad: probe_to_sun.angle_to(to_body),
        })
    };
    Ok(BodyView {
        distance_m,
        light_time_s: light_time_s(distance_m),
        angles,
    })
}

/// 天体 `body` の幾何を `start` から `times_s`（昇順・`start` からの経過 [s]）の時刻ごとに求める。
/// `start` は軌道の元期と違っていてよい（軌道の元期からの経過に直して伝搬する）。
pub fn body_views_at(
    orbit: &Trajectory,
    start: Epoch,
    times_s: &[f64],
    body: CentralBody,
) -> Result<Vec<BodyView>, AstroError> {
    probe_states(orbit, start, times_s)?
        .into_iter()
        .map(|(epoch, r, pos)| view(orbit.central(), epoch, r, &pos, body))
        .collect()
}

/// 天体 `body` の中心までの距離 [m]（探査機の状態から。[`body_views_at`] の距離と同じ式）。
fn distance_of(orbit: &Trajectory, epoch: Epoch, state: &OrbitState, body: CentralBody) -> f64 {
    let (r, pos) = positions(orbit, epoch, state);
    if body == orbit.central() {
        r.norm()
    } else {
        (body.position_eci_m(epoch) - pos.probe).norm()
    }
}

/// 天体 `body` の中心までの距離 [m] だけを時刻ごとに求める（角が要らないときの軽い入口。値は [`body_views_at`] と同じ）。
pub fn body_distances_m(
    orbit: &Trajectory,
    start: Epoch,
    times_s: &[f64],
    body: CentralBody,
) -> Result<Vec<f64>, AstroError> {
    let states = orbit.states_at(&offsets(orbit, start, times_s))?;
    Ok(times_s
        .iter()
        .zip(&states)
        .map(|(&t, s)| distance_of(orbit, start.plus_seconds(t), s, body))
        .collect())
}

/// `start` からの経過 `times_s` を、軌道の元期からの経過にする。
fn offsets(orbit: &Trajectory, start: Epoch, times_s: &[f64]) -> Vec<f64> {
    let offset = start.seconds_since(orbit.epoch());
    times_s.iter().map(|t| t + offset).collect()
}

/// 太陽・地球の幾何を時刻ごとに求める（時刻の取り方は [`body_views_at`] と同じ）。
pub fn geometry_at(
    orbit: &Trajectory,
    start: Epoch,
    times_s: &[f64],
) -> Result<Vec<GeometrySample>, AstroError> {
    let states = orbit.states_at(&offsets(orbit, start, times_s))?;
    times_s
        .iter()
        .zip(&states)
        .map(|(&t_s, s)| sample_of(orbit, start, t_s, s))
        .collect()
}

/// 時刻 `start + t_s` の探査機の状態 `state` から太陽・地球の幾何の標本 1 つを作る。
fn sample_of(
    orbit: &Trajectory,
    start: Epoch,
    t_s: f64,
    state: &OrbitState,
) -> Result<GeometrySample, AstroError> {
    let central = orbit.central();
    let epoch = start.plus_seconds(t_s);
    let (r, pos) = positions(orbit, epoch, state);
    let sun = view(central, epoch, r, &pos, CentralBody::Sun)?;
    let earth = view(central, epoch, r, &pos, CentralBody::Earth)?;
    let angles = earth
        .angles
        .ok_or_else(|| AstroError::InvalidInput("The Earth angles are undefined".into()))?;
    Ok(GeometrySample {
        t_s,
        epoch,
        sun_distance_m: sun.distance_m,
        earth_distance_m: earth.distance_m,
        sep_rad: angles.at_body_rad,
        spe_rad: angles.at_probe_rad,
        light_time_s: earth.light_time_s,
    })
}

/// 任意の時刻の探査機の状態を何度も求める道具（極値を詰める評価のように、1 時刻ずつ多数回の評価をするとき用）。
/// 軌道の表を 1 回だけ作る（[`Trajectory::track`]。解析的な軌道は表を使わず、[`Trajectory::states_at`] と同じ値）ので、
/// 数値伝搬の軌道で時刻ごとに元期から積分し直さない。`start` から `last_s` [s] までで使える。
pub struct ProbeTrack<'a> {
    orbit: &'a Trajectory,
    start: Epoch,
    offset_s: f64,
    track: OrbitTrack,
}

impl<'a> ProbeTrack<'a> {
    /// `start` から `last_s` [s] までの道具を作る。
    pub fn new(orbit: &'a Trajectory, start: Epoch, last_s: f64) -> Result<Self, AstroError> {
        let offset_s = start.seconds_since(orbit.epoch());
        let track = orbit.track(offset_s + last_s, orbit.table_step_s())?;
        Ok(Self {
            orbit,
            start,
            offset_s,
            track,
        })
    }

    /// `start` から `t_s` 後の太陽・地球の幾何（[`geometry_at`] の 1 時刻と同じ量）。
    pub fn geometry(&self, t_s: f64) -> Result<GeometrySample, AstroError> {
        sample_of(
            self.orbit,
            self.start,
            t_s,
            &self.track.state_at(t_s + self.offset_s),
        )
    }

    /// `start` から `t_s` 後の天体 `body` の中心までの距離 [m]（[`body_distances_m`] の 1 時刻と同じ量）。
    pub fn distance_m(&self, t_s: f64, body: CentralBody) -> f64 {
        distance_of(
            self.orbit,
            self.start.plus_seconds(t_s),
            &self.track.state_at(t_s + self.offset_s),
            body,
        )
    }
}

/// 標本 `samples` の中の量 `metric` の最小・最大を、標本の前後の区間で詰めて求める。
/// 詰める評価は [`ProbeTrack`]（表を 1 回だけ作る）で 1 時刻ずつ行う。標本が空なら `None`。
pub fn refined_extremes(
    orbit: &Trajectory,
    start: Epoch,
    samples: &[GeometrySample],
    metric: Metric,
) -> Result<Option<Extremes>, AstroError> {
    let times: Vec<f64> = samples.iter().map(|s| s.t_s).collect();
    let values: Vec<f64> = samples.iter().map(|s| metric.of(s)).collect();
    let Some(last_s) = times.iter().copied().reduce(f64::max) else {
        return Ok(None);
    };
    let probe = ProbeTrack::new(orbit, start, last_s)?;
    let breaks = breakpoints_from(orbit, start);
    extremes_by_split(&times, &values, &breaks, |t| {
        Ok(metric.of(&probe.geometry(t)?))
    })
}

/// 時刻列 `times`（昇順）に、範囲の中の折れ目 `breaks`（昇順）を加えた昇順の列。標本の最小・最大を折れ目で取りこぼさないために使う
/// （折れ目は距離の傾きが跳ぶ点で、標本の間にあると極値を見逃す）。折れ目が無ければ `times` のまま。
pub fn times_with_breaks(times: &[f64], breaks: &[f64]) -> Vec<f64> {
    let (Some(first), Some(last)) = (times.first().copied(), times.last().copied()) else {
        return Vec::new();
    };
    let mut all: Vec<f64> = times.to_vec();
    all.extend(breaks.iter().copied().filter(|b| *b > first && *b < last));
    all.sort_by(f64::total_cmp);
    all.dedup();
    all
}

/// 軌道の折れ目（弧の連鎖の境。[`Trajectory::breakpoints_s`]）を、`start` からの経過 [s] にしたもの（昇順。折れ目の無い軌道は空）。
/// 極値を詰める区間はこの折れ目で分ける（[`extremes_by_split`]）。**軌道の折れ目を標本の時刻の座標に直すのはここだけ**。
pub fn breakpoints_from(orbit: &Trajectory, start: Epoch) -> Vec<f64> {
    let offset = start.seconds_since(orbit.epoch());
    orbit
        .breakpoints_s()
        .into_iter()
        .map(|b| b - offset)
        .collect()
}

#[cfg(test)]
mod chain_tests;
#[cfg(test)]
mod tests;
