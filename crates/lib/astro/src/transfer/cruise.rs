//! 日付つきの遷移（[`DatedTransfer`]）の巡航の多体の確認。Lambert の解（太陽の点質量だけの円錐曲線）の出発の状態から、
//! 太陽と惑星の点質量の重力で数値伝播して、到着の日の到着天体とのずれ・最接近・到着の超過速度の差を求める。
//!
//! **伝播は新しく作らない。** 既存の数値伝播（[`crate::orbit::propagate_states`]。中心天体は太陽）・力モデル
//! （[`crate::force`] の第三体。[`ForceConfig::heliocentric_point_masses`]）・積分器（[`crate::ode::Dop853`]）・
//! 暦（[`crate::ephem`]）を使い、ここにあるのは「遷移の状態から力モデルを選ぶ」ことと、到着側の量の取り出しだけ。
//! 最接近の探索は [`crate::orbit::closest_approach`]。
//!
//! 単純化（結果の注記にも出す）:
//! - 出発天体の中心から Lambert の速度で始める。**出発と到着の天体の重力は入れない**（パッチドコニックは両端の影響圏の中を
//!   双曲線の ΔV で扱っており、その中は伝播しない。到着天体の点質量を入れると、中心を狙う軌道は特異になる）。
//!   ほかの惑星（地球が端でなければ地球も）は点質量の第三体として入れる。地球は月を含めず地球の重力定数だけ。
//! - 影響圏の中のずれは見ない。ずれは「天体の中心を狙った日心の軌道が、ほかの惑星の重力で到着の日にどれだけ外れるか」。
//! - 日付が DE440 の期間でなければ暦は解析式で、ずれの大きさは暦の誤差（火星で 10⁵ km 級）と同程度になり、判定に使えない。

use super::dated::{DatedTransfer, LambertArc};
use crate::AstroError;
use crate::constants::SECONDS_PER_DAY;
use crate::ephem::{CentralBody, EphemerisSource, covers, eci_to_ecliptic_j2000};
use crate::force::ForceConfig;
use crate::math::Vec3;
use crate::ode::{Dop853, step_offsets};
use crate::orbit::{ClosestApproach, HELIOCENTRIC_INTEGRATOR, closest_approach, propagate_states};
use crate::state::OrbitState;
use crate::time::Epoch;

/// 巡航の積分器: 精度は既定の数値伝播と同じで、刻みの上限だけを 1 日にする（惑星間は 600 s の上限が要らない）。
pub const CRUISE_INTEGRATOR: Dop853 = HELIOCENTRIC_INTEGRATOR;

/// 距離を測る粗い格子の間隔 [s]。
const COARSE_STEP_S: f64 = 21_600.0;

/// 到着の日の後も伝播する期間 [s]。到着の日に最接近の手前でも、その先の極小を探すため（暦の範囲を出るなら伝播しない）。
const OVERSHOOT_S: f64 = 10.0 * SECONDS_PER_DAY;

/// 軌跡の図に出す点の数の目安。
const CHART_POINTS: usize = 120;

/// 軌跡の 1 点（黄道面。日心。J2000 平均黄道の x・y [m]）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CruiseSample {
    /// 出発の日からの経過 [s]
    pub t_s: f64,
    pub spacecraft_xy_m: [f64; 2],
    pub from_xy_m: [f64; 2],
    pub to_xy_m: [f64; 2],
    /// 探査機と到着天体の距離（3 次元）[m]
    pub range_to_arrival_m: f64,
}

/// 巡航の確認の結果。
#[derive(Debug, Clone, PartialEq)]
pub struct CruiseCheck {
    /// 天体の位置の出所（遷移の出所。出発・到着の両方が DE440 のときだけ `De440`）
    pub ephemeris: EphemerisSource,
    /// 到着の日の、到着天体の中心から探査機へのベクトル（ECI の向き）[m]
    pub miss_vector_m: Vec3,
    /// 到着の天体の影響圏の半径 [m]
    pub arrival_soi_m: Option<f64>,
    /// 最接近（時刻は出発の日からの経過 [s]。到着の日の後 [`OVERSHOOT_S`] まで探す）
    pub closest: ClosestApproach,
    /// 伝播した到着の超過速度ベクトル（探査機の速度 − 到着天体の速度）[m/s]
    pub v_inf_arrival_m_s: Vec3,
    /// 力モデルに第三体として入れた天体
    pub perturbers: Vec<CentralBody>,
    /// 軌跡（出発の日から到着の日まで）
    pub samples: Vec<CruiseSample>,
}

impl CruiseCheck {
    /// 到着の日のずれの大きさ [m]。
    pub fn miss_distance_m(&self) -> f64 {
        self.miss_vector_m.norm()
    }
}

/// 日心の位置（ECI の向き）[m]。
fn heliocentric_position(body: CentralBody, parent: CentralBody, epoch: Epoch) -> Vec3 {
    body.position_eci_m(epoch) - parent.position_eci_m(epoch)
}

/// 第三体に入れた天体の一覧（惑星と、端でなければ地球）。
fn perturbers(forces: &ForceConfig) -> Vec<CentralBody> {
    let earth = (!forces.omit_earth).then_some(CentralBody::Earth);
    let all = [
        CentralBody::Mercury,
        CentralBody::Venus,
        CentralBody::Mars,
        CentralBody::Jupiter,
        CentralBody::Saturn,
        CentralBody::Uranus,
        CentralBody::Neptune,
    ];
    earth
        .into_iter()
        .chain(all.into_iter().filter(|c| {
            c.as_body()
                .is_some_and(|b| forces.third_bodies.contains(&b))
        }))
        .collect()
}

/// 粗い格子（出発 → 到着）と、その先の [`OVERSHOOT_S`] までの時刻（出発の日からの経過 [s]）。
/// 戻りの `usize` は到着の日の時刻の添字。
fn time_grid(arrive: Epoch, transfer_s: f64) -> (Vec<f64>, usize) {
    let mut times = step_offsets(transfer_s, COARSE_STEP_S);
    let arrival_index = times.len() - 1;
    if covers(arrive.plus_seconds(OVERSHOOT_S)) {
        times.extend(
            step_offsets(OVERSHOOT_S, COARSE_STEP_S)
                .into_iter()
                .skip(1)
                .map(|t| transfer_s + t),
        );
    }
    (times, arrival_index)
}

/// 軌跡の図の点（粗い格子を間引く。両端を含む）。
fn chart(
    parent: CentralBody,
    (from, to): (CentralBody, CentralBody),
    depart: Epoch,
    times: &[f64],
    states: &[OrbitState],
) -> Vec<CruiseSample> {
    let stride = times.len().div_ceil(CHART_POINTS).max(1);
    let xy = |v: Vec3| {
        let e = eci_to_ecliptic_j2000(v);
        [e.x, e.y]
    };
    let pick = |i: usize| {
        let epoch = depart.plus_seconds(times[i]);
        let to_position = heliocentric_position(to, parent, epoch);
        CruiseSample {
            t_s: times[i],
            spacecraft_xy_m: xy(states[i].r_eci),
            from_xy_m: xy(heliocentric_position(from, parent, epoch)),
            to_xy_m: xy(to_position),
            range_to_arrival_m: (states[i].r_eci - to_position).norm(),
        }
    };
    let mut out: Vec<CruiseSample> = (0..times.len()).step_by(stride).map(pick).collect();
    if let Some(last) = times.len().checked_sub(1)
        && out.last().is_some_and(|s| s.t_s < times[last])
    {
        out.push(pick(last));
    }
    out
}

/// 確認する巡航の弧の始まり（出発の日の状態・親・飛行時間・暦の出所）。日付つきの遷移の全体（[`check_cruise`]）か、
/// 通過のある遷移の 1 本の脚（[`check_cruise_leg`]）から作る。
struct CruiseStart {
    parent: CentralBody,
    state: OrbitState,
    transfer_s: f64,
    ephemeris: EphemerisSource,
}

/// 日付つきの遷移 `transfer`（`from` を `depart` に出て `to` に着く。途中の通過のないもの）の巡航を、太陽と惑星の点質量で
/// 伝播して確認する。出発の状態は `transfer.cruise_state`（出発天体の中心、Lambert の速度）。
/// 到着の日まで（と、その後の [`OVERSHOOT_S`] まで）を [`CRUISE_INTEGRATOR`] で伝播する。
/// 途中の通過のある遷移は、脚ごとに [`check_cruise_leg`] で確認する（通過の前後をつなげて伝播はしない）。
pub fn check_cruise(
    transfer: &DatedTransfer,
    from: CentralBody,
    to: CentralBody,
    depart: Epoch,
) -> Result<CruiseCheck, AstroError> {
    let start = CruiseStart {
        parent: transfer.parent,
        state: transfer.cruise_state,
        transfer_s: transfer.transfer_time_s,
        ephemeris: transfer.ephemeris,
    };
    check_from(&start, from, to, depart)
}

/// 通過のある遷移の 1 本の脚 `arc`（`from` を `depart` に出て `to` に着く Lambert の弧）の巡航を、[`check_cruise`] と同じ
/// 伝播で確認する。脚の始まりは `from` の中心から弧の出発の速度。両端（通過する天体を含む）の重力は入れない
/// （通過の天体の近くの双曲線の弧・影響圏の中は伝播しない）。`ephemeris` は暦の出所（判定に使う）。
pub fn check_cruise_leg(
    arc: &LambertArc,
    from: CentralBody,
    to: CentralBody,
    depart: Epoch,
    ephemeris: EphemerisSource,
) -> Result<CruiseCheck, AstroError> {
    let start = CruiseStart {
        parent: arc.parent,
        state: OrbitState {
            r_eci: arc.from_state.r_eci,
            v_eci: arc.v_departure_m_s,
        },
        transfer_s: arc.transfer_time_s,
        ephemeris,
    };
    check_from(&start, from, to, depart)
}

fn check_from(
    start: &CruiseStart,
    from: CentralBody,
    to: CentralBody,
    depart: Epoch,
) -> Result<CruiseCheck, AstroError> {
    let parent = start.parent;
    let forces = ForceConfig::heliocentric_point_masses(&[from, to]);
    let arrive = depart.plus_seconds(start.transfer_s);
    let (times, arrival_index) = time_grid(arrive, start.transfer_s);
    let propagate =
        |ts: &[f64]| propagate_states(parent, &forces, depart, start.state, ts, &CRUISE_INTEGRATOR);
    let distance = |ts: &[f64], states: &[OrbitState]| -> Vec<f64> {
        ts.iter()
            .zip(states)
            .map(|(t, s)| {
                (s.r_eci - heliocentric_position(to, parent, depart.plus_seconds(*t))).norm()
            })
            .collect()
    };
    let states = propagate(&times)?;
    let closest = closest_approach(&times, &distance(&times, &states), |ts| {
        Ok(distance(ts, &propagate(ts)?))
    })?;
    let at_arrival = states[arrival_index];
    let target = to
        .state_about_parent(arrive)
        .ok_or(AstroError::NotSiblingBodies { from, to })?;
    let upto = &times[..=arrival_index];
    Ok(CruiseCheck {
        ephemeris: start.ephemeris,
        miss_vector_m: at_arrival.r_eci - target.r_eci,
        arrival_soi_m: to.soi_radius_m(),
        closest,
        v_inf_arrival_m_s: at_arrival.v_eci - target.v_eci,
        perturbers: perturbers(&forces),
        samples: chart(parent, (from, to), depart, upto, &states[..=arrival_index]),
    })
}

#[cfg(test)]
mod tests;
