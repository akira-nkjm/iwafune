//! 回転系 ⇄ 慣性系の変換のテスト。

use super::*;
use crate::constants::{AU, GM_SUN, MU_EARTH};
use crate::math::Vec3;
use crate::state::OrbitState;

fn sun_earth() -> Cr3bp {
    Cr3bp::from_bodies(CentralBody::Sun, Secondary::Body(CentralBody::Earth)).expect("mu")
}

/// 円軌道の副天体（主天体中心、角速度 n、軌道面は xy 平面を少し傾ける）。位相 `phase` [rad]。
fn circular_secondary(cr: &Cr3bp, a: f64, gm_total: f64, phase: f64) -> (OrbitState, f64) {
    let n = (gm_total / a.powi(3)).sqrt();
    let _ = cr;
    let (s, c) = phase.sin_cos();
    (
        OrbitState {
            r_eci: Vec3::new(c, s, 0.0) * a,
            v_eci: Vec3::new(-s, c, 0.0) * (a * n),
        },
        n,
    )
}

#[test]
fn round_trip_for_an_eccentric_secondary() {
    let cr = sun_earth();
    // 離心・傾斜した副天体
    let sec = OrbitState {
        r_eci: Vec3::new(1.2e11, -4.0e10, 2.0e10),
        v_eci: Vec3::new(8.0e3, 2.6e4, -1.0e3),
    };
    let x = [0.7, -0.3, 0.2, 0.05, -0.4, 0.1];
    let s = cr.rotating_to_inertial(&sec, &x).expect("inertial");
    let back = cr.inertial_to_rotating(&sec, &s).expect("rotating");
    for i in 0..6 {
        assert!(
            (back[i] - x[i]).abs() < 1e-12,
            "{i}: {} vs {}",
            back[i],
            x[i]
        );
    }
}

#[test]
fn distances_are_preserved() {
    let cr = sun_earth();
    let sec = OrbitState {
        r_eci: Vec3::new(1.2e11, -4.0e10, 2.0e10),
        v_eci: Vec3::new(8.0e3, 2.6e4, -1.0e3),
    };
    let d = sec.r_eci.norm();
    let x = [0.7, -0.3, 0.2, 0.05, -0.4, 0.1];
    let s = cr.rotating_to_inertial(&sec, &x).expect("inertial");
    let mu = cr.mu();
    // 重心からの距離 = d·|X|、主天体・副天体からの距離 = d·r1、d·r2
    let bary = sec.r_eci * mu;
    assert!(
        ((s.r_eci - bary).norm() - d * (x[0] * x[0] + x[1] * x[1] + x[2] * x[2]).sqrt()).abs()
            < 1e-3
    );
    let r1 = ((x[0] + mu).powi(2) + x[1] * x[1] + x[2] * x[2]).sqrt();
    let r2 = ((x[0] - 1.0 + mu).powi(2) + x[1] * x[1] + x[2] * x[2]).sqrt();
    assert!((s.r_eci.norm() - d * r1).abs() < 1e-3);
    assert!(((s.r_eci - sec.r_eci).norm() - d * r2).abs() < 1e-3);
}

/// 副天体が円軌道のとき、L4 に静止した点は慣性系で重心まわりの円軌道（副天体と同じ周期 2π/n）。
/// 位置は a·|X| の円、速度は n·a·|X| で位置に垂直。2 つの位相で確かめる。主天体・副天体からの距離は a。
#[test]
fn point_at_rest_at_l4_is_an_inertial_circular_orbit_with_the_secondary_period() {
    let cr = sun_earth();
    let a = AU;
    let gm = GM_SUN + MU_EARTH;
    let l4 = cr.libration_point(LibrationPoint::L4).expect("l4");
    let x = [l4[0], l4[1], 0.0, 0.0, 0.0, 0.0];
    let rho_norm = a * (l4[0] * l4[0] + l4[1] * l4[1]).sqrt();
    let mut angles = Vec::new();
    for phase in [0.3, 1.9] {
        let (sec, n) = circular_secondary(&cr, a, gm, phase);
        let s = cr.rotating_to_inertial(&sec, &x).expect("inertial");
        let rho = s.r_eci - sec.r_eci * cr.mu();
        let vel = s.v_eci - sec.v_eci * cr.mu();
        assert!((rho.norm() - rho_norm).abs() < 1e-3);
        assert!((vel.norm() - n * rho_norm).abs() < 1e-9 * n * rho_norm);
        assert!(rho.dot(vel).abs() < 1e-12 * rho.norm() * vel.norm());
        // 主天体・副天体からの距離は a（正三角形）
        assert!((s.r_eci.norm() - a).abs() < 1e-3);
        assert!(((s.r_eci - sec.r_eci).norm() - a).abs() < 1e-3);
        angles.push((rho.y.atan2(rho.x) - phase).rem_euclid(std::f64::consts::TAU));
    }
    // 副天体と同じ角速度で回る: 副天体との位相差が一定
    assert!((angles[0] - angles[1]).abs() < 1e-12);
}

/// μ → 0 の主天体まわりの円軌道（回転系で速度 r(r^(−3/2) − 1)）は、慣性系で速度 √((1−μ)/r)·a·n の円軌道。
/// 速度の変換（ω × ρ の項）の検証。許容は μ（1e-9）程度の相対 1e-8。
#[test]
fn velocity_conversion_gives_the_two_body_speed_for_tiny_mu() {
    let mu = 1e-9;
    let cr = Cr3bp::new(mu).expect("mu");
    let (a, gm) = (1.0e9, 1.0e10);
    let (sec, n) = circular_secondary(&cr, a, gm, 0.7);
    let r = 0.1_f64;
    let x = [r - mu, 0.0, 0.0, 0.0, r * (r.powf(-1.5) - 1.0), 0.0];
    let s = cr.rotating_to_inertial(&sec, &x).expect("inertial");
    let expected = ((1.0 - mu) / r).sqrt() * a * n;
    assert!((s.v_eci.norm() - expected).abs() < 1e-8 * expected);
    assert!((s.r_eci.norm() - r * a).abs() < 1e-8 * r * a);
    assert!(s.r_eci.dot(s.v_eci).abs() < 1e-8 * r * a * expected);
}

#[test]
fn degenerate_secondary_state_is_rejected() {
    let cr = sun_earth();
    let radial = OrbitState {
        r_eci: Vec3::new(1.0e11, 0.0, 0.0),
        v_eci: Vec3::new(1.0e3, 0.0, 0.0),
    };
    assert!(cr.rotating_to_inertial(&radial, &[0.0; 6]).is_none());
}
