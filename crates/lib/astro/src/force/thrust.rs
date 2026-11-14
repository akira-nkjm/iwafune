//! 推力の項（ECI）。推力の向きの決まり（[`ThrustSteering`]）と、推力 / 質量の加速度。
//!
//! - 接線方向（[`ThrustSteering::Tangential`]）: 速度の向きを、軌道面の外へ偏揺れ角 `β` だけ傾けた向き。
//!   高度と傾斜角を同時に変えるときの `β` は Edelbaum の決まり（Edelbaum, ARS Journal 31(8), 1961）:
//!   `tan β₀ = sin(π Δi / 2) / (v₀ / v₁ − cos(π Δi / 2))`、噴射で積んだ ΔV を `Δv` として
//!   `v sin β = v₀ sin β₀`・`v cos β = v₀ cos β₀ − Δv` から `β = atan2(v₀ sin β₀, v₀ cos β₀ − Δv)`。
//!   面外の成分の符号は昇交点からの引数 `u` の `cos u` に合わせる（傾斜角の変化率 `di/dt ∝ cos u`）。
//!   `β > 90°` は速度と逆向き（高度を下げる）。ΔV の総量の式は `crate::transfer::edelbaum` で、ここは向きだけを持つ。
//! - 慣性固定（[`ThrustSteering::Inertial`]）: 噴射の始めに決めた ECI の向きのまま。

use crate::math::Vec3;
use crate::state::OrbitState;
use core::f64::consts::FRAC_PI_2;
use serde::{Deserialize, Serialize};

/// 推力の向きの決まり。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ThrustSteering {
    /// 速度の向き（傾斜角を変えるときは Edelbaum の偏揺れ角で面外へ傾ける）
    #[default]
    Tangential,
    /// 噴射の始めの向きのまま慣性空間に固定する
    Inertial,
}

/// Edelbaum の始めの偏揺れ角 `β₀` [rad]（0〜π）。`v0`・`v1` は始めと目標の円軌道の速さ [m/s]、
/// `delta_i_rad` は傾斜角の変化の大きさ。速さが正でなければ 0（接線方向）。
pub fn edelbaum_initial_yaw_rad(v0_m_s: f64, v1_m_s: f64, delta_i_rad: f64) -> f64 {
    if !(v0_m_s > 0.0 && v1_m_s > 0.0) {
        return 0.0;
    }
    let half = FRAC_PI_2 * delta_i_rad.abs();
    half.sin().atan2(v0_m_s / v1_m_s - half.cos())
}

/// ΔV `delta_v_m_s` を積んだときの Edelbaum の偏揺れ角 `β` [rad]（0〜π）。
pub fn edelbaum_yaw_rad(v0_m_s: f64, initial_yaw_rad: f64, delta_v_m_s: f64) -> f64 {
    (v0_m_s * initial_yaw_rad.sin()).atan2(v0_m_s * initial_yaw_rad.cos() - delta_v_m_s)
}

/// 昇交点からの引数 `u` の余弦（軌道面の法線 `h` と ECI の z 軸から）。赤道軌道では x 軸を交点の向きとみなす。
pub fn cos_argument_of_latitude(state: &OrbitState) -> f64 {
    let h = state.orbit_normal();
    let node = Vec3::Z.cross(h).normalized().unwrap_or(Vec3::X);
    state
        .r_eci
        .normalized()
        .map_or(1.0, |r| r.dot(node).clamp(-1.0, 1.0))
}

/// 接線方向の推力の単位ベクトル: 速度の向きを偏揺れ角 `yaw_rad` だけ、軌道面の法線の向き `normal_sign`（±1）へ傾ける。
pub fn tangential_direction(state: &OrbitState, yaw_rad: f64, normal_sign: f64) -> Vec3 {
    let along = state.v_eci.normalized().unwrap_or(Vec3::X);
    let normal = state.orbit_normal();
    along * yaw_rad.cos() + normal * (normal_sign.signum() * yaw_rad.sin())
}

/// 推力の加速度 [m/s²] = 向き（単位ベクトル）× 推力 [N] / 質量 [kg]。質量が正でなければ 0。
pub fn thrust_acceleration_eci(direction: Vec3, thrust_n: f64, mass_kg: f64) -> Vec3 {
    if mass_kg > 0.0 && mass_kg.is_finite() {
        direction * (thrust_n / mass_kg)
    } else {
        Vec3::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::PI;

    #[test]
    fn coplanar_yaw_is_along_or_against_velocity() {
        // 傾斜角を変えなければ、上げるときは β = 0、下げるときは β = π
        assert!(edelbaum_initial_yaw_rad(7600.0, 7500.0, 0.0).abs() < 1e-12);
        assert!((edelbaum_initial_yaw_rad(7500.0, 7600.0, 0.0) - PI).abs() < 1e-12);
        // 速さを変えない面の変更は β = 90°（Edelbaum の式で Δv = v π Δi / 2）
        let b = edelbaum_initial_yaw_rad(7500.0, 7500.0, 0.1);
        assert!(
            (b - (FRAC_PI_2 - FRAC_PI_2 * 0.1 / 2.0)).abs() < 1e-12,
            "{b}"
        );
    }

    #[test]
    fn yaw_reaches_target_speed_at_edelbaum_delta_v() {
        // v cos β = v₀ cos β₀ − Δv・v sin β = v₀ sin β₀ を Δv_Edelbaum まで回すと v = v₁
        let (v0, v1, di) = (7600.0_f64, 7500.0_f64, 2.0_f64.to_radians());
        let dv = crate::transfer::combined_burn_m_s(v0, v1, FRAC_PI_2 * di);
        let b0 = edelbaum_initial_yaw_rad(v0, v1, di);
        let b = edelbaum_yaw_rad(v0, b0, dv);
        let (vs, vc) = (v0 * b0.sin(), v0 * b0.cos() - dv);
        assert!(((vs * vs + vc * vc).sqrt() - v1).abs() < 1e-9);
        assert!((b.sin() * v1 - vs).abs() < 1e-9);
    }

    #[test]
    fn thrust_acceleration_by_hand() {
        let a = thrust_acceleration_eci(Vec3::X, 1e-3, 10.0);
        assert!((a.x - 1e-4).abs() < 1e-18);
        assert_eq!(thrust_acceleration_eci(Vec3::X, 1.0, 0.0), Vec3::ZERO);
    }
}
