//! 日付を決めた惑星間の遷移（出発の日・到着の日の天体の位置から Lambert の問題を解く）。
//!
//! 共通の親（太陽）を回る 2 つの天体（惑星 ↔ 惑星）の間を、周回数 0 の 1 本の円錐曲線でつなぐ（パッチドコニック）。
//! 出発の日の出発天体の位置 `r₁`・到着の日の到着天体の位置 `r₂`（どちらも親の中心から。[`CentralBody::state_about_parent`]）と
//! 飛行時間から親のまわりの遷移の軌道を求め（[`lambert_about_normal`]。順行は黄道の北に対して決める）、
//! 両端の天体に対する超過速度 `v∞ = v_遷移 − v_天体` から C3・出発の漸近線の赤緯・近点の噴射を出す
//! （Curtis, *Orbital Mechanics for Engineering Students*, 3rd ed., §8.6〜8.10。Vallado §12.4）。
//! 影響圏は点とみなす。軌道面の違いは天体の実際の位置・速度に含まれる。

use super::flyby::{FlybyPass, FlybyStop, default_min_altitude_m, flyby_geometry};
use super::lambert::lambert_about_normal;
use super::patched::{PatchedBurn, capture_burn};
use super::{Apsides, check_mu};
use crate::AstroError;
use crate::ephem::{
    CentralBody, EphemerisSource, covers, ecliptic_normal_eci, orientation::pole_eci, source_at,
};
use crate::math::Vec3;
use crate::orbit::{OrbitElements, OrbitSpec, Propagator, osculating_elements};
use crate::state::OrbitState;
use crate::time::Epoch;
use sat_units::typed::Qty;

/// 日付を決めた遷移。ベクトルは ECI（J2000 平均赤道）の向き。途中に天体を通過（スイングバイ。[`super::flyby`]）するなら、
/// 前後の天体の組ごとの Lambert の弧（脚）をつなぐ（通過のないときは弧 1 本）。
#[derive(Debug, Clone, PartialEq)]
pub struct DatedTransfer {
    /// 出発の天体のまわりの駐車軌道の近点での脱出の噴射
    pub departure: PatchedBurn,
    /// 到着の捕獲の噴射（捕獲の軌道を書かなければ `None`。フライバイ・大気突入）
    pub arrival: Option<PatchedBurn>,
    /// 出発の特性エネルギー C3 = |v∞|² [m²/s²]
    pub c3_m2_s2: f64,
    /// 出発の双曲線超過速度ベクトル（最初の脚の出発の速度 − 出発天体の速度）[m/s]
    pub v_inf_departure_m_s: Vec3,
    /// 到着の双曲線超過速度ベクトル（最後の脚の到着の速度 − 到着天体の速度）[m/s]
    pub v_inf_arrival_m_s: Vec3,
    /// 途中のスイングバイ（日付の順。無ければ空）
    pub flybys: Vec<FlybyPass>,
    /// 脚ごとの Lambert の弧（出発 → 最初の通過 → … → 到着。通過がなければ 1 本）
    pub legs: Vec<LambertArc>,
    /// 出発の漸近線の赤緯 [rad]（出発天体の赤道に対して。出発の駐車軌道の傾斜角はこれ以上にならないと直接は出られない）
    pub departure_declination_rad: f64,
    /// 飛行時間（出発 → 到着の全体）[s]
    pub transfer_time_s: f64,
    /// 遷移の軌道の中心（共通の親）
    pub parent: CentralBody,
    /// 出発の日の遷移の軌道（最初の脚）の状態（親の中心から。ECI の向き）
    pub cruise_state: OrbitState,
    /// 天体の位置の出所（出発・到着の両方が DE440 のときだけ `De440`）
    pub ephemeris: EphemerisSource,
}

impl DatedTransfer {
    /// スイングバイの ΔV の合計 [m/s]（近点の噴射 + 足りない偏向の分。通過がなければ 0）。
    pub fn flyby_dv_m_s(&self) -> f64 {
        self.flybys.iter().map(FlybyPass::total_dv_m_s).sum()
    }
}

fn iso(epoch: Epoch) -> String {
    epoch.to_iso()
}

/// 2 天体が親を共有する別々の天体で、日付が暦の中にあり順序が正しいことを確かめて、親を返す。
fn validate(
    from: CentralBody,
    to: CentralBody,
    depart: Epoch,
    arrive: Epoch,
) -> Result<CentralBody, AstroError> {
    let parent = match (from.parent(), to.parent()) {
        (Some(a), Some(b)) if a == b && from != to => a,
        _ => return Err(AstroError::NotSiblingBodies { from, to }),
    };
    for epoch in [depart, arrive] {
        if !covers(epoch) {
            return Err(AstroError::OutsideEphemeris { epoch: iso(epoch) });
        }
    }
    if arrive.seconds_since(depart) <= 0.0 {
        return Err(AstroError::ArrivalNotAfterDeparture {
            depart: iso(depart),
            arrive: iso(arrive),
        });
    }
    Ok(parent)
}

/// 日付を決めた遷移の Lambert の弧（両端の天体の状態と、親のまわりの遷移の軌道の両端の速度）。
/// **日付つきの遷移の Lambert を解くのはここだけ**（[`dated_transfer`] の噴射と、巡航の軌道 [`cruise_orbit`] が共有する）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LambertArc {
    /// 遷移の軌道の中心（共通の親）
    pub parent: CentralBody,
    /// 飛行時間 [s]
    pub transfer_time_s: f64,
    /// 出発の日の出発天体の状態（親の中心から。ECI の向き）
    pub from_state: OrbitState,
    /// 到着の日の到着天体の状態（同）
    pub to_state: OrbitState,
    /// 遷移の軌道の出発の速度 [m/s]
    pub v_departure_m_s: Vec3,
    /// 遷移の軌道の到着の速度 [m/s]
    pub v_arrival_m_s: Vec3,
}

/// 天体 `from` を `depart` に出て天体 `to` に `arrive` に着く Lambert の弧を解く（順行は黄道の北に対して決める）。
/// 条件（共通の親の別々の天体・暦の範囲・日付の順序）に合わない、または Lambert が解けないときはエラー。
pub fn lambert_arc(
    from: CentralBody,
    to: CentralBody,
    depart: Epoch,
    arrive: Epoch,
) -> Result<LambertArc, AstroError> {
    let parent = validate(from, to, depart, arrive)?;
    let mu = parent.gm();
    check_mu(mu)?;
    let transfer_time_s = arrive.seconds_since(depart);
    let (Some(start), Some(end)) = (
        from.state_about_parent(depart),
        to.state_about_parent(arrive),
    ) else {
        return Err(AstroError::NotSiblingBodies { from, to });
    };
    let sol = lambert_about_normal(
        mu,
        start.r_eci,
        end.r_eci,
        transfer_time_s,
        true,
        ecliptic_normal_eci(),
    )?;
    Ok(LambertArc {
        parent,
        transfer_time_s,
        from_state: start,
        to_state: end,
        v_departure_m_s: sol.v1_m_s,
        v_arrival_m_s: sol.v2_m_s,
    })
}

/// 日付つきの遷移の巡航の軌道: 出発の日の遷移の状態（[`LambertArc`]）を親のまわりの接触要素にした二体（Kepler）の軌道。
/// 元期は出発の日、中心は共通の親、伝播は二体なので Lambert の弧そのもの（両端の天体の位置を通る）。
/// 遷移の軌道が楕円でない（双曲線・放物線）ときはエラー（長半径が定まらない。状態ベクトルの source が受け持つ）。
pub fn cruise_orbit(
    from: CentralBody,
    to: CentralBody,
    depart: Epoch,
    arrive: Epoch,
) -> Result<OrbitSpec, AstroError> {
    arc_orbit(&lambert_arc(from, to, depart, arrive)?, from, to, depart)
}

/// Lambert の弧 `arc`（`from` を `depart` に出て `to` に着く）を、出発の日を元期とする親のまわりの二体の軌道にする
/// （[`cruise_orbit`] の本体。通過のある遷移の脚ごとの軌道 [`super::leg::cruise_legs`] も使う）。楕円でなければエラー。
pub(super) fn arc_orbit(
    arc: &LambertArc,
    from: CentralBody,
    to: CentralBody,
    depart: Epoch,
) -> Result<OrbitSpec, AstroError> {
    let state = OrbitState {
        r_eci: arc.from_state.r_eci,
        v_eci: arc.v_departure_m_s,
    };
    let mu = arc.parent.gm();
    let energy = 0.5 * state.v_eci.norm_squared() - mu / state.r_eci.norm();
    if energy.is_nan() || energy >= 0.0 {
        return Err(AstroError::InvalidInput(format!(
            "The transfer arc from {from:?} to {to:?} is not an ellipse about {parent:?} (specific energy {energy:.3e} J/kg)",
            parent = arc.parent
        )));
    }
    let mut spec = spec_from_state(&state, mu, depart)?;
    spec.central = arc.parent;
    Ok(spec)
}

/// 親のまわりの状態 `state`（元期 `epoch`）を接触要素にした二体（Kepler）の軌道（中心天体は呼ぶ側が置く）。
/// **状態 → 巡航の軌道の変換はここだけ**（[`cruise_orbit`] と、巡航の区間の切り出し [`super::leg::cruise_leg`] が使う）。
pub(super) fn spec_from_state(
    state: &OrbitState,
    mu: f64,
    epoch: Epoch,
) -> Result<OrbitSpec, AstroError> {
    let el = osculating_elements(state, mu)?;
    let elements = OrbitElements {
        semi_major_axis: Qty::from_raw(el.a_m),
        eccentricity: el.e,
        inclination: Qty::from_raw(el.i_rad),
        raan: Qty::from_raw(el.raan_rad),
        arg_perigee: Qty::from_raw(el.argp_rad),
        mean_anomaly: Qty::from_raw(el.mean_anomaly_rad),
    };
    Ok(OrbitSpec::new(epoch, elements, Propagator::Kepler))
}

/// 天体 `from` の軌道 `parking` から、日付 `depart` に出発して `arrive` に天体 `to` に着く遷移
/// （`capture` はその軌道への捕獲。`None` なら捕獲しない）。出発・到着の噴射は軌道の近点で行う。
/// 途中の通過がない遷移（[`dated_transfer_via`] に空の列を渡したもの）。
/// `from` と `to` が同じ親を回る別々の天体でない、到着が出発より後でない、日付が暦の外にある、
/// Lambert の問題が解けない（2 点が正反対に近く面が決まらない）ときはエラー。
pub fn dated_transfer(
    from: CentralBody,
    parking: Apsides,
    to: CentralBody,
    capture: Option<Apsides>,
    depart: Epoch,
    arrive: Epoch,
) -> Result<DatedTransfer, AstroError> {
    dated_transfer_via(from, parking, to, capture, depart, arrive, &[])
}

/// 日付つきの脚の並び（出発 → `flybys` の通過 → 到着）の Lambert の弧。通過の日は出発と到着の間で、
/// 順に厳密に増える必要がある。隣り合う天体は別々でなければならない（同じ天体の脚は周回数 0 の弧にならない）。
/// **脚の弧を並べるのはここだけ**（[`dated_transfer_via`] が使う）。
pub fn lambert_chain(
    from: CentralBody,
    to: CentralBody,
    depart: Epoch,
    arrive: Epoch,
    flybys: &[FlybyStop],
) -> Result<Vec<LambertArc>, AstroError> {
    let with_flybys = !flybys.is_empty();
    let mut nodes = vec![(from, depart)];
    nodes.extend(flybys.iter().map(|f| (f.body, f.date)));
    nodes.push((to, arrive));
    nodes
        .windows(2)
        .map(|pair| {
            let ((a, ta), (b, tb)) = (pair[0], pair[1]);
            // 通過のない遷移の同じ天体は `lambert_arc` が従来どおり `NotSiblingBodies` にする
            if a == b && with_flybys {
                return Err(AstroError::InvalidInput(format!(
                    "Consecutive bodies of a transfer must differ ({a:?} is followed by {b:?}); a flyby of the departure or arrival body needs another body in between"
                )));
            }
            lambert_arc(a, b, ta, tb)
        })
        .collect()
}

/// [`dated_transfer`] に途中のスイングバイ（`flybys`。日付の順）を足したもの。脚ごとに日心の Lambert を解き
/// （[`lambert_chain`]）、出発の噴射は最初の脚、捕獲の噴射は最後の脚の超過速度から、各通過は前後の脚の超過速度から
/// [`flyby_geometry`] で求める（近点の高度の下限は書かれていなければ [`default_min_altitude_m`]。慣例の無い天体は書かせる）。
/// 通過がなければ [`dated_transfer`] と同じ数値になる。
pub fn dated_transfer_via(
    from: CentralBody,
    parking: Apsides,
    to: CentralBody,
    capture: Option<Apsides>,
    depart: Epoch,
    arrive: Epoch,
    flybys: &[FlybyStop],
) -> Result<DatedTransfer, AstroError> {
    parking.check()?;
    if let Some(c) = capture {
        c.check()?;
    }
    let legs = lambert_chain(from, to, depart, arrive, flybys)?;
    let (Some(first), Some(last)) = (legs.first().copied(), legs.last().copied()) else {
        return Err(AstroError::InvalidInput(
            "A transfer needs at least one leg".to_owned(),
        ));
    };
    let v_inf_dep = first.v_departure_m_s - first.from_state.v_eci;
    let v_inf_arr = last.v_arrival_m_s - last.to_state.v_eci;
    let v_dep = v_inf_dep.norm();
    let declination = v_inf_dep.normalized().map_or(0.0, |u| {
        u.dot(pole_eci(from, depart)).clamp(-1.0, 1.0).asin()
    });
    let passes = flyby_passes(&legs, flybys)?;
    let all_de440 = std::iter::once(depart)
        .chain(flybys.iter().map(|f| f.date))
        .chain(std::iter::once(arrive))
        .all(|t| source_at(t) == EphemerisSource::De440);
    Ok(DatedTransfer {
        departure: capture_burn(from, v_dep, parking),
        arrival: capture.map(|c| capture_burn(to, v_inf_arr.norm(), c)),
        c3_m2_s2: v_dep * v_dep,
        v_inf_departure_m_s: v_inf_dep,
        v_inf_arrival_m_s: v_inf_arr,
        flybys: passes,
        legs,
        departure_declination_rad: declination,
        transfer_time_s: arrive.seconds_since(depart),
        parent: first.parent,
        cruise_state: OrbitState {
            r_eci: first.from_state.r_eci,
            v_eci: first.v_departure_m_s,
        },
        ephemeris: if all_de440 {
            EphemerisSource::De440
        } else {
            EphemerisSource::Analytic
        },
    })
}

/// 隣り合う脚の組（`legs[i]` の到着・`legs[i+1]` の出発が `flybys[i]` の通過）から、通過ごとの幾何を求める。
fn flyby_passes(legs: &[LambertArc], flybys: &[FlybyStop]) -> Result<Vec<FlybyPass>, AstroError> {
    legs.windows(2)
        .zip(flybys)
        .map(|(pair, stop)| {
            let min_altitude = stop
                .min_altitude_m
                .or_else(|| default_min_altitude_m(stop.body))
                .ok_or_else(|| {
                    AstroError::InvalidInput(format!(
                        "No default minimum flyby altitude exists for {:?}; write one",
                        stop.body
                    ))
                })?;
            let body_v = pair[0].to_state.v_eci;
            flyby_geometry(
                stop.body,
                stop.date,
                pair[0].v_arrival_m_s - body_v,
                pair[1].v_departure_m_s - body_v,
                min_altitude,
            )
        })
        .collect()
}
