//! 遷移の式の手計算・文献値との比較。

use super::*;
use crate::constants::{MU_EARTH, R_EARTH};

fn close(a: f64, b: f64, tol: f64) {
    assert!(
        (a - b).abs() <= tol,
        "difference between {a} and {b} exceeds {tol}"
    );
}

/// LEO 400 → 550 km のホーマン遷移。手計算:
/// `Δv₁ = √(μ/r₁)(√(2r₂/(r₁+r₂)) − 1)`、`Δv₂ = √(μ/r₂)(1 − √(2r₁/(r₁+r₂)))`。合計 ≈ 83.5 m/s。
#[test]
fn hohmann_leo_400_to_550_km() {
    let (r1, r2) = (R_EARTH + 400e3, R_EARTH + 550e3);
    let t = hohmann(MU_EARTH, r1, r2).unwrap();
    let dv1 = (MU_EARTH / r1).sqrt() * ((2.0 * r2 / (r1 + r2)).sqrt() - 1.0);
    let dv2 = (MU_EARTH / r2).sqrt() * (1.0 - (2.0 * r1 / (r1 + r2)).sqrt());
    assert_eq!(t.burns.len(), 2);
    close(t.burns[0].delta_v_m_s, dv1, 1e-9);
    close(t.burns[1].delta_v_m_s, dv2, 1e-9);
    close(t.total_m_s(), 83.5, 0.1);
    // 遷移楕円の半周 π√(a³/μ)、a = (r₁ + r₂)/2 ≈ 47 分
    let a = 0.5 * (r1 + r2);
    close(t.coast_s, PI * (a * a * a / MU_EARTH).sqrt(), 1e-6);
    close(t.coast_s / 60.0, 47.1, 0.1);
    // 下げる向きも同じ ΔV（噴射の順が逆）
    let down = hohmann(MU_EARTH, r2, r1).unwrap();
    close(down.total_m_s(), t.total_m_s(), 1e-9);
}

/// Curtis, *Orbital Mechanics for Engineering Students* Example 6.3 と同じ条件（μ = 398 600 km³/s²、7 000 km → 105 000 km の
/// 円軌道、中間の遠点 210 000 km）の手計算: ホーマン 2.787 + 1.260 = 4.047 km/s、バイエリプティック
/// 2.952 + 0.775 + 0.301 = 4.029 km/s（比が 11.94 を超えるのでバイエリプティックが安い）。
#[test]
fn bi_elliptic_beats_hohmann_for_a_large_ratio() {
    let mu = 398_600e9;
    let (ra, rc, rb) = (7_000e3, 105_000e3, 210_000e3);
    let h = hohmann(mu, ra, rc).unwrap();
    let b = bi_elliptic(mu, Apsides::circular(ra), Apsides::circular(rc), rb, 0.0).unwrap();
    close(h.total_m_s() / 1e3, 4.0467, 5e-4);
    close(b.total_m_s() / 1e3, 4.0287, 5e-4);
    assert_eq!(b.burns.len(), 3);
    // 中間の遠点が目標と同じならホーマンと同じ
    let same = bi_elliptic(mu, Apsides::circular(ra), Apsides::circular(rc), rc, 0.0).unwrap();
    close(same.total_m_s(), h.total_m_s(), 1e-6);
    assert!(bi_elliptic(mu, Apsides::circular(ra), Apsides::circular(rc), ra, 0.0).is_err());
}

/// 平面変更 `2 v sin(θ/2)`。LEO（約 7.6 km/s）で 1° ≈ 133 m/s。
#[test]
fn plane_change_by_hand() {
    let r = R_EARTH + 550e3;
    let v = (MU_EARTH / r).sqrt();
    let dv = plane_change_m_s(v, 1f64.to_radians());
    close(dv, 2.0 * v * 0.5f64.to_radians().sin(), 1e-9);
    close(dv, 132.4, 0.2);
    // 同じ半径の円軌道どうしの遷移は平面変更の 1 回だけ
    let t = apse_transfer(
        MU_EARTH,
        Apsides::circular(r),
        Apsides::circular(r),
        1f64.to_radians(),
    )
    .unwrap();
    assert_eq!(t.burns.len(), 1);
    close(t.total_m_s(), dv, 1e-9);
    assert_eq!(t.coast_s, 0.0);
}

/// 高度を上げながらの平面変更は遠い方の噴射に合わせる（合わせた方が分けるより安い）。
#[test]
fn plane_change_is_combined_with_the_outer_burn() {
    let (r1, r2) = (R_EARTH + 400e3, R_EARTH + 550e3);
    let angle = 0.5f64.to_radians();
    let t = apse_transfer(
        MU_EARTH,
        Apsides::circular(r1),
        Apsides::circular(r2),
        angle,
    )
    .unwrap();
    let h = hohmann(MU_EARTH, r1, r2).unwrap();
    let separate = h.total_m_s() + plane_change_m_s((MU_EARTH / r2).sqrt(), angle);
    assert!(t.total_m_s() < separate);
    close(t.burns[0].delta_v_m_s, h.burns[0].delta_v_m_s, 1e-9);
}

/// 円軌道 600 km から近地点 250 km への離脱は 1 回噴射（遠地点で減速）。手計算 `v_c − v_apo`。
#[test]
fn deorbit_lowers_perigee_with_one_burn() {
    let (r, rp) = (R_EARTH + 600e3, R_EARTH + 250e3);
    let t = apse_transfer(MU_EARTH, Apsides::circular(r), Apsides::new(rp, r), 0.0).unwrap();
    assert_eq!(t.burns.len(), 1);
    let vc = (MU_EARTH / r).sqrt();
    let va = (MU_EARTH * (2.0 / r - 2.0 / (r + rp))).sqrt();
    close(t.total_m_s(), vc - va, 1e-9);
    close(t.total_m_s(), 97.84, 0.01);
    assert_eq!(t.coast_s, 0.0);
}

/// Edelbaum: 傾斜角を変えなければ円軌道の速さの差。LEO 300 km（28.5°）→ GEO（0°）は約 5.9 km/s
/// （手計算 √(v₀² + v₁² − 2v₀v₁cos(π/2 · 28.5°))、v₀ = 7.726 km/s、v₁ = 3.075 km/s）。
#[test]
fn edelbaum_by_hand() {
    let (a0, a1) = (R_EARTH + 400e3, R_EARTH + 550e3);
    let coplanar = edelbaum(MU_EARTH, a0, a1, 0.0).unwrap();
    close(
        coplanar,
        (MU_EARTH / a0).sqrt() - (MU_EARTH / a1).sqrt(),
        1e-9,
    );
    // 低推力の螺旋はホーマンより少しだけ高い（同じ 2 円の間では |v₀ − v₁| ≥ ホーマン）
    assert!(coplanar >= hohmann(MU_EARTH, a0, a1).unwrap().total_m_s());
    let geo = 42_164e3;
    let leo = R_EARTH + 300e3;
    let dv = edelbaum(MU_EARTH, leo, geo, 28.5f64.to_radians()).unwrap();
    let (v0, v1) = ((MU_EARTH / leo).sqrt(), (MU_EARTH / geo).sqrt());
    let hand = (v0 * v0 + v1 * v1 - 2.0 * v0 * v1 * (0.5 * PI * 28.5f64.to_radians()).cos()).sqrt();
    close(dv, hand, 1e-9);
    close(dv / 1e3, 5.95, 0.05);
}

/// Robbins の倍率: 小さい弧で 1 + Φ²/24、Φ = 90° で (π/4)/sin(π/4) = 1.1107。
#[test]
fn finite_burn_factor_by_hand() {
    close(finite_burn_factor(0.0), 1.0, 1e-15);
    close(finite_burn_factor(0.1), 1.0 + 0.01 / 24.0, 1e-6);
    close(finite_burn_factor(0.5 * PI), 1.110_720_7, 1e-6);
}

/// 有限噴射: 推力が大きければインパルスと同じ。噴射時間は使った推薬の m_p v_e / F。
#[test]
fn finite_burn_converges_and_matches_the_rocket_equation() {
    let r = R_EARTH + 550e3;
    let n = (MU_EARTH / (r * r * r)).sqrt();
    let ve = crate::rocket::exhaust_velocity(220.0);
    let quick = finite_burn(50.0, n, 1.0e4, ve, 10.0).unwrap();
    close(quick.delta_v_m_s, 50.0, 1e-6);
    // 0.5 N・10 kg・50 m/s → 噴射時間 ≈ 1000 s、弧 ≈ 63°。損失は約 5 %
    let slow = finite_burn(50.0, n, 0.5, ve, 10.0).unwrap();
    let mp = crate::rocket::propellant_for(ve, 10.0, slow.delta_v_m_s).unwrap();
    close(slow.burn_time_s, mp * ve / 0.5, 1e-6);
    close(
        slow.delta_v_m_s,
        50.0 * finite_burn_factor(n * slow.burn_time_s),
        1e-6,
    );
    assert!(slow.delta_v_m_s > 50.0 && slow.delta_v_m_s < 55.0);
    // 半周を超える噴射は使えない
    assert!(finite_burn(50.0, n, 0.01, ve, 10.0).is_err());
}

#[test]
fn invalid_orbits_are_rejected() {
    assert!(hohmann(MU_EARTH, -1.0, 7e6).is_err());
    assert!(hohmann(0.0, 7e6, 7e6).is_err());
    assert!(edelbaum(MU_EARTH, f64::NAN, 7e6, 0.0).is_err());
}
