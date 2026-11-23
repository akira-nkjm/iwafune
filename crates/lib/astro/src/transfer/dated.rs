//! 日付を決めた惑星間の遷移（出発の日・到着の日の天体の位置から Lambert の問題を解く）。
//!
//! 共通の親（太陽）を回る 2 つの天体（惑星 ↔ 惑星）の間を、周回数 0 の 1 本の円錐曲線でつなぐ（パッチドコニック）。
//! 出発の日の出発天体の位置 `r₁`・到着の日の到着天体の位置 `r₂`（どちらも親の中心から。[`CentralBody::state_about_parent`]）と
//! 飛行時間から親のまわりの遷移の軌道を求め（[`lambert_about_normal`]。順行は黄道の北に対して決める）、
//! 両端の天体に対する超過速度 `v∞ = v_遷移 − v_天体` から C3・出発の漸近線の赤緯・近点の噴射を出す
//! （Curtis, *Orbital Mechanics for Engineering Students*, 3rd ed., §8.6〜8.10。Vallado §12.4）。
//! 影響圏は点とみなす。軌道面の違いは天体の実際の位置・速度に含まれる。

use super::lambert::lambert_about_normal;
use super::patched::{PatchedBurn, capture_burn};
use super::{Apsides, check_mu};
use crate::AstroError;
use crate::ephem::{
    CentralBody, EphemerisSource, covers, ecliptic_normal_eci, orientation::pole_eci, source_at,
};
use crate::math::Vec3;
use crate::state::OrbitState;
use crate::time::Epoch;

/// 日付を決めた遷移。ベクトルは ECI（J2000 平均赤道）の向き。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DatedTransfer {
    /// 出発の天体のまわりの駐車軌道の近点での脱出の噴射
    pub departure: PatchedBurn,
    /// 到着の捕獲の噴射（捕獲の軌道を書かなければ `None`。フライバイ・大気突入）
    pub arrival: Option<PatchedBurn>,
    /// 出発の特性エネルギー C3 = |v∞|² [m²/s²]
    pub c3_m2_s2: f64,
    /// 出発の双曲線超過速度ベクトル（遷移の出発の速度 − 出発天体の速度）[m/s]
    pub v_inf_departure_m_s: Vec3,
    /// 到着の双曲線超過速度ベクトル（遷移の到着の速度 − 到着天体の速度）[m/s]
    pub v_inf_arrival_m_s: Vec3,
    /// 出発の漸近線の赤緯 [rad]（出発天体の赤道に対して。出発の駐車軌道の傾斜角はこれ以上にならないと直接は出られない）
    pub departure_declination_rad: f64,
    /// 飛行時間 [s]
    pub transfer_time_s: f64,
    /// 遷移の軌道の中心（共通の親）
    pub parent: CentralBody,
    /// 出発の日の遷移の軌道の状態（親の中心から。ECI の向き）
    pub cruise_state: OrbitState,
    /// 天体の位置の出所（出発・到着の両方が DE440 のときだけ `De440`）
    pub ephemeris: EphemerisSource,
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

/// 天体 `from` の軌道 `parking` から、日付 `depart` に出発して `arrive` に天体 `to` に着く遷移
/// （`capture` はその軌道への捕獲。`None` なら捕獲しない）。出発・到着の噴射は軌道の近点で行う。
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
    let parent = validate(from, to, depart, arrive)?;
    parking.check()?;
    if let Some(c) = capture {
        c.check()?;
    }
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
    let v_inf_dep = sol.v1_m_s - start.v_eci;
    let v_inf_arr = sol.v2_m_s - end.v_eci;
    let v_dep = v_inf_dep.norm();
    let declination = v_inf_dep.normalized().map_or(0.0, |u| {
        u.dot(pole_eci(from, depart)).clamp(-1.0, 1.0).asin()
    });
    let both_de440 =
        source_at(depart) == EphemerisSource::De440 && source_at(arrive) == EphemerisSource::De440;
    Ok(DatedTransfer {
        departure: capture_burn(from, v_dep, parking),
        arrival: capture.map(|c| capture_burn(to, v_inf_arr.norm(), c)),
        c3_m2_s2: v_dep * v_dep,
        v_inf_departure_m_s: v_inf_dep,
        v_inf_arrival_m_s: v_inf_arr,
        departure_declination_rad: declination,
        transfer_time_s,
        parent,
        cruise_state: OrbitState {
            r_eci: start.r_eci,
            v_eci: sol.v1_m_s,
        },
        ephemeris: if both_de440 {
            EphemerisSource::De440
        } else {
            EphemerisSource::Analytic
        },
    })
}
