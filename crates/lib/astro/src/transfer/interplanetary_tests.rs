//! パッチドコニックと Lambert の問題の、文献値・手計算との比較。

use super::*;
use crate::constants::{A_MOON, MU_EARTH, R_EARTH, R_MARS, R_MOON};
use crate::ephem::CentralBody;
use crate::math::Vec3;
use crate::orbit::{elements_to_state, mean_motion_rad_s, osculating_elements};
use crate::state::OrbitState;

fn close(a: f64, b: f64, tol: f64) {
    assert!(
        (a - b).abs() <= tol,
        "difference between {a} and {b} exceeds {tol}"
    );
}

/// 地球 → 火星のホーマン遷移（Curtis, *Orbital Mechanics for Engineering Students*, 3rd ed., Example 8.3・8.4）:
/// 高度 300 km の円軌道から出発する ΔV 3.590 km/s、出発の v∞ 2.943 km/s（C3 ≈ 8.66 km²/s²）、
/// 到着の v∞ 2.648 km/s、飛行時間 259 日。Curtis は軌道の半径 149.6・227.9 × 10⁶ km、ここは Standish の J2000 の長半径なので
/// 1 % 程度の差を許す。火星の高度 300 km の円軌道への捕獲は手計算で
/// `√(2.648² + 2·42 828/3 696) − √(42 828/3 696) ≈ 5.494 − 3.404 = 2.09 km/s`。
#[test]
fn earth_to_mars_hohmann_matches_curtis() {
    let leo = Apsides::circular(R_EARTH + 300e3);
    let lmo = Apsides::circular(R_MARS + 300e3);
    let t = patched_conic(CentralBody::Earth, leo, CentralBody::Mars, Some(lmo)).unwrap();
    close(t.departure.delta_v_m_s / 1e3, 3.590, 0.02);
    close(t.departure.v_inf_m_s.unwrap() / 1e3, 2.943, 0.02);
    close(t.c3_m2_s2 / 1e6, 8.66, 0.1);
    let arrival = t.arrival.unwrap();
    assert_eq!(arrival.central, CentralBody::Mars);
    close(arrival.v_inf_m_s.unwrap() / 1e3, 2.648, 0.02);
    close(arrival.delta_v_m_s / 1e3, 2.09, 0.02);
    close(t.transfer_time_s / 86_400.0, 259.0, 1.5);
    // 捕獲しなければ出発だけ
    let flyby = patched_conic(CentralBody::Earth, leo, CentralBody::Mars, None).unwrap();
    assert!(flyby.arrival.is_none());
    close(flyby.total_m_s(), t.departure.delta_v_m_s, 1e-9);
}

/// 地球 → 月（平均距離 384 400 km の円軌道の月へのホーマン遷移）。手計算:
/// 高度 185 km（Apollo の待機軌道）から `a = (6 563 + 384 400)/2 km`、`v_p = √(μ(2/r − 1/a)) ≈ 10.928 km/s`、
/// `v_c ≈ 7.793 km/s` で TLI ≈ 3.13 km/s（Apollo の TLI は約 3.1〜3.2 km/s。NASA, *Apollo by the Numbers*, SP-2000-4029）。
/// 月での v∞ = 1.018 − 0.188 ≈ 0.830 km/s、高度 100 km の円軌道への LOI は
/// `√(0.830² + 2·4 902.8/1 837.4) − √(4 902.8/1 837.4) ≈ 2.455 − 1.634 ≈ 0.82 km/s`。飛行時間は約 5 日。
#[test]
fn earth_to_moon_tli_and_loi() {
    let leo = Apsides::circular(R_EARTH + 185e3);
    let llo = Apsides::circular(R_MOON + 100e3);
    let t = patched_conic(CentralBody::Earth, leo, CentralBody::Moon, Some(llo)).unwrap();
    close(t.departure.delta_v_m_s / 1e3, 3.135, 0.01);
    assert!(t.departure.v_inf_m_s.is_none() && t.c3_m2_s2 < 0.0);
    let a = 0.5 * (R_EARTH + 185e3 + A_MOON);
    close(t.c3_m2_s2, -MU_EARTH / a, 1e-3);
    let loi = t.arrival.unwrap();
    close(loi.v_inf_m_s.unwrap() / 1e3, 0.830, 0.005);
    close(loi.delta_v_m_s / 1e3, 0.821, 0.01);
    close(t.transfer_time_s / 86_400.0, 4.98, 0.05);
    // 帰り（月 → 地球の高度 185 km の近地点）は脱出が同じ v∞、地球側は捕獲の軌道を楕円のままにすれば 0
    let back = patched_conic(
        CentralBody::Moon,
        llo,
        CentralBody::Earth,
        Some(Apsides::new(R_EARTH + 185e3, A_MOON)),
    )
    .unwrap();
    close(back.departure.delta_v_m_s, loi.delta_v_m_s, 1e-6);
    close(back.arrival.unwrap().delta_v_m_s, 0.0, 1e-6);
}

/// 親子でも兄弟でもない天体（月 → 火星）は途中で分ける。
#[test]
fn unrelated_bodies_are_rejected() {
    let llo = Apsides::circular(R_MOON + 100e3);
    assert!(patched_conic(CentralBody::Moon, llo, CentralBody::Mars, None).is_err());
}

/// Curtis Example 5.2: μ = 398 600 km³/s²、r₁ = (5 000, 10 000, 2 100) km、r₂ = (−14 600, 2 500, 7 000) km、
/// Δt = 1 h（順行）→ v₁ = (−5.9925, 1.9254, 3.2456) km/s、v₂ = (−3.3125, −4.1966, −0.38529) km/s。
#[test]
fn lambert_matches_curtis_example_5_2() {
    let mu = 398_600e9;
    let r1 = Vec3::new(5_000e3, 10_000e3, 2_100e3);
    let r2 = Vec3::new(-14_600e3, 2_500e3, 7_000e3);
    let s = lambert(mu, r1, r2, 3_600.0, true).unwrap();
    for (got, want) in [
        (s.v1_m_s, [-5.9925, 1.9254, 3.2456]),
        (s.v2_m_s, [-3.3125, -4.1966, -0.38529]),
    ] {
        close(got.x / 1e3, want[0], 1e-3);
        close(got.y / 1e3, want[1], 1e-3);
        close(got.z / 1e3, want[2], 1e-3);
    }
    // r₁・v₁ を Kepler で 1 h 進めると r₂・v₂ に戻る
    let mut el = osculating_elements(
        &OrbitState {
            r_eci: r1,
            v_eci: s.v1_m_s,
        },
        mu,
    )
    .unwrap();
    el.mean_anomaly_rad += mean_motion_rad_s(mu, el.a_m) * 3_600.0;
    let end = elements_to_state(&el, mu);
    assert!((end.r_eci - r2).norm() < 1.0, "{:?}", end.r_eci);
    assert!((end.v_eci - s.v2_m_s).norm() < 1e-3);
}

/// 半周ちょうどのホーマンの端どうしは面が決まらない。順行と逆行は違う解になる。
#[test]
fn lambert_rejects_degenerate_geometry_and_honours_direction() {
    let r1 = Vec3::new(7_000e3, 0.0, 0.0);
    assert!(lambert(MU_EARTH, r1, -r1 * 2.0, 5_000.0, true).is_err());
    assert!(lambert(MU_EARTH, r1, r1 * 2.0, 5_000.0, true).is_err());
    assert!(lambert(MU_EARTH, r1, Vec3::new(0.0, 9_000e3, 0.0), 0.0, true).is_err());
    let r2 = Vec3::new(0.0, 9_000e3, 0.0);
    let pro = lambert(MU_EARTH, r1, r2, 3_000.0, true).unwrap();
    let retro = lambert(MU_EARTH, r1, r2, 3_000.0, false).unwrap();
    assert!(r1.cross(pro.v1_m_s).z > 0.0 && r1.cross(retro.v1_m_s).z < 0.0);
}
