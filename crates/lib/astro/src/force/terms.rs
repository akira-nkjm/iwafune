//! 力の項ごとの加速度（ECI）。[`super::acceleration_breakdown`] がこれらを並べ、
//! [`super::acceleration_eci`] がその和をとる。

use super::gravity;
use super::{DragParams, GravityModel, SrpParams};
use crate::atmosphere::density_kg_m3;
use crate::constants::{C_LIGHT, J2_EARTH, MU_EARTH, OMEGA_EARTH, R_EARTH};
use crate::earth::{ecef_to_eci, eci_to_ecef};
use crate::illumination::solar_flux_w_m2;
use crate::math::Vec3;
use crate::shadow::{ShadowModel, earth_shadow_factor};
use crate::state::OrbitState;
use crate::time::Epoch;

/// 重力場を「中心項」と「非球形の項」に分けて返す（ECI）。和が重力場全体の加速度。
///
/// 係数表が使えない（埋め込みデータの破損）ときは点質量＋J2 で代える。
pub(super) fn gravity_split(model: GravityModel, epoch: Epoch, r: Vec3) -> (Vec3, Vec3) {
    match model {
        GravityModel::PointMass => (point_mass(MU_EARTH, r), Vec3::ZERO),
        GravityModel::J2 => (point_mass(MU_EARTH, r), j2_perturbation(r)),
        GravityModel::Harmonics { degree, order } => match gravity::egm96() {
            Some(table) => {
                let r_ecef = eci_to_ecef(r, epoch);
                let (d, o) = (usize::from(degree), usize::from(order));
                let rest = table.acceleration_ecef(r_ecef, d, o, 1);
                (point_mass(table.mu, r), ecef_to_eci(rest, epoch))
            }
            None => (point_mass(MU_EARTH, r), j2_perturbation(r)),
        },
    }
}

/// 点質量の重力 −μ r/|r|³。
pub fn point_mass(mu: f64, r: Vec3) -> Vec3 {
    let rn = r.norm();
    if rn > 0.0 {
        r * (-mu / (rn * rn * rn))
    } else {
        Vec3::ZERO
    }
}

/// 地球の J2 による摂動加速度（ECI。地球の自転軸は ECI の z 軸とみなす）。[`zonal_j2`] の地球・`pole = z`。
pub fn j2_perturbation(r: Vec3) -> Vec3 {
    zonal_j2(MU_EARTH, R_EARTH, J2_EARTH, Vec3::Z, r)
}

/// 自転軸 `pole`（単位ベクトル）まわりの J2 の摂動（μ [m³/s²]・基準半径 `re` [m]・J2）。**J2 の式はここだけ。**
///
/// `a = −(3/2) J2 μ R²/r⁵ [r (1 − 5 (r·p)²/r²) + 2 (r·p) p]`。`pole = z` なら成分は
/// `k x (1 − 5 z²/r²)`・`k y (1 − 5 z²/r²)`・`k z (3 − 5 z²/r²)`（`k = −(3/2) J2 μ R²/r⁵`）。
pub fn zonal_j2(mu: f64, re: f64, j2: f64, pole: Vec3, r: Vec3) -> Vec3 {
    let rn = r.norm();
    if rn.is_nan() || rn <= 0.0 {
        return Vec3::ZERO;
    }
    let k = -1.5 * j2 * mu * re * re / rn.powi(5);
    let z = r.dot(pole);
    (r * (1.0 - 5.0 * z * z / (rn * rn)) + pole * (2.0 * z)) * k
}

/// 太陽輻射圧（キャノンボール）。`nu` は日照率（影の判定は呼び出し側が [`crate::shadow`] で行う）。
pub(super) fn srp_with_shadow(p: &SrpParams, r_sat: Vec3, r_sun: Vec3, nu: f64) -> Vec3 {
    let d = r_sat - r_sun;
    let dn = d.norm();
    if dn.is_nan() || dn <= 0.0 {
        return Vec3::ZERO;
    }
    let pressure = srp_pressure(dn);
    d * (nu * pressure * p.cr * p.area_to_mass / dn)
}

/// 第三体の摂動 μ_b [(r_b − r)/|r_b − r|³ − r_b/|r_b|³]（地心系なので間接項を引く）。
pub fn third_body_eci(gm: f64, r_body: Vec3, r_sat: Vec3) -> Vec3 {
    let d = r_body - r_sat;
    let (dn, bn) = (d.norm(), r_body.norm());
    if !(dn > 0.0 && bn > 0.0) {
        return Vec3::ZERO;
    }
    (d / dn.powi(3) - r_body / bn.powi(3)) * gm
}

/// 太陽輻射圧（キャノンボール、円錐影）。太陽から遠ざかる向き。
pub fn srp_eci(p: &SrpParams, r_sat: Vec3, r_sun: Vec3) -> Vec3 {
    let nu = earth_shadow_factor(r_sat, r_sun, ShadowModel::Conical);
    srp_with_shadow(p, r_sat, r_sun, nu)
}

/// 大気抵抗（キャノンボール、大気は地球と共回転）。密度は [`crate::atmosphere::density_kg_m3`]。
/// 加速度は [`drag_force`] に `Cd·A/m` を渡したもの。
pub fn drag_eci(p: &DragParams, epoch: Epoch, state: &OrbitState) -> Vec3 {
    let v_rel = relative_wind_eci(state.r_eci, state.v_eci);
    let rho = density_kg_m3(&p.atmosphere, state.r_eci, epoch);
    drag_force(rho, p.cd * p.area_to_mass, v_rel)
}

/// 抗力 `F = −½ ρ (Cd·A) |v| v`。`rho` は大気密度 [kg/m³]、`cd_area` は抗力係数 × 面積 [m²]、
/// `v_rel` は大気に対する速度 [m/s]（[`relative_wind_eci`] か、それを機体座標系に直したもの）。
/// `cd_area` に `Cd·A/m` [m²/kg] を渡すと加速度 [m/s²]。**抗力の式はここだけ**（軌道の抗力と空力トルクで共有する）。
pub fn drag_force(rho: f64, cd_area: f64, v_rel: Vec3) -> Vec3 {
    v_rel * (-0.5 * rho * cd_area * v_rel.norm())
}

/// 一般相対論の補正（Schwarzschild 項、IERS 2010 eq. 10.12 の主項。中心天体の μ [m³/s²]）。
///
/// `a = μ/(c² r³) [(4μ/r − v²) r + 4 (r·v) v]`。LEO で ~1e-8 m/s²（主重力の 1e-9 程度）。
pub fn relativity_eci(mu: f64, state: &OrbitState) -> Vec3 {
    let (r, v) = (state.r_eci, state.v_eci);
    let rn = r.norm();
    if rn.is_nan() || rn <= 0.0 {
        return Vec3::ZERO;
    }
    let pre = mu / (C_LIGHT * C_LIGHT * rn * rn * rn);
    (r * (4.0 * mu / rn - v.norm_squared()) + v * (4.0 * r.dot(v))) * pre
}

/// 太陽からの距離 `sun_distance_m` [m] での太陽輻射圧 [N/m²]（完全吸収面）= 太陽光の強さ / c。
/// 強さは [`crate::illumination::solar_flux_w_m2`]。
pub fn srp_pressure(sun_distance_m: f64) -> f64 {
    solar_flux_w_m2(sun_distance_m) / C_LIGHT
}

/// 地球と共回転する大気に対する衛星の速度（ECI）[m/s] = v − ω_E × r。抗力と空力トルクで共有する。
pub fn relative_wind_eci(r_eci_m: Vec3, v_eci_m_s: Vec3) -> Vec3 {
    v_eci_m_s - Vec3::new(0.0, 0.0, OMEGA_EARTH).cross(r_eci_m)
}
