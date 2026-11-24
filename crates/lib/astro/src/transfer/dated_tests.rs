//! 日付つきの遷移（`dated_transfer`）と Lambert の文献値・手計算との比較。暦は解析式（DE440 は登録しない）。

use super::*;
use crate::constants::{GM_MARS, GM_SUN, MU_EARTH, R_EARTH, R_MARS};
use crate::ephem::{CentralBody, EphemerisSource, ecliptic_normal_eci};
use crate::math::Vec3;
use crate::orbit::{elements_to_state, mean_motion_rad_s, osculating_elements};
use crate::state::OrbitState;
use crate::time::Epoch;

fn close(a: f64, b: f64, tol: f64) {
    assert!(
        (a - b).abs() <= tol,
        "difference between {a} and {b} exceeds {tol}"
    );
}

fn epoch(s: &str) -> Epoch {
    Epoch::parse(s).expect("epoch")
}

fn leo() -> Apsides {
    Apsides::circular(R_EARTH + 300e3)
}

fn lmo() -> Apsides {
    Apsides::circular(R_MARS + 300e3)
}

/// Curtis Example 8.8 の遷移（1996-11-07 → 1997-09-12）。
fn curtis() -> DatedTransfer {
    dated_transfer(
        CentralBody::Earth,
        leo(),
        CentralBody::Mars,
        Some(lmo()),
        epoch("1996-11-07T00:00:00Z"),
        epoch("1997-09-12T00:00:00Z"),
    )
    .expect("transfer")
}

/// Vallado Example 7-5: r₀ = (15 945.34, 0, 0) km、r = (12 214.838 99, 10 249.467 31, 0) km、76 min、短い経路、
/// 地球の μ → v₀ = (2.058 913, 2.915 965, 0) km/s、v = (−3.451 565, 0.910 315, 0) km/s。
#[test]
fn lambert_matches_vallado_example_7_5() {
    let r0 = Vec3::new(15_945.34e3, 0.0, 0.0);
    let r = Vec3::new(12_214.838_99e3, 10_249.467_31e3, 0.0);
    let s = lambert(MU_EARTH, r0, r, 76.0 * 60.0, true).expect("solution");
    for (got, want) in [
        (s.v1_m_s, [2.058_913, 2.915_965, 0.0]),
        (s.v2_m_s, [-3.451_565, 0.910_315, 0.0]),
    ] {
        close(got.x / 1e3, want[0], 1e-5);
        close(got.y / 1e3, want[1], 1e-5);
        close(got.z / 1e3, want[2], 1e-9);
    }
}

/// 順行の基準を黄道の北にしても、z 軸のときと同じ面内の問題は同じ解になる（`lambert` は z を基準にしたまま）。
#[test]
fn lambert_about_normal_with_z_equals_lambert() {
    let r1 = Vec3::new(7_000e3, 0.0, 0.0);
    let r2 = Vec3::new(0.0, 9_000e3, 1_000e3);
    let a = lambert(MU_EARTH, r1, r2, 3_000.0, true).expect("a");
    let b = lambert_about_normal(MU_EARTH, r1, r2, 3_000.0, true, Vec3::Z).expect("b");
    assert_eq!(a, b);
    let flipped = lambert_about_normal(MU_EARTH, r1, r2, 3_000.0, true, -Vec3::Z).expect("c");
    let retro = lambert(MU_EARTH, r1, r2, 3_000.0, false).expect("d");
    assert_eq!(flipped, retro);
}

/// Curtis Example 8.8（出発 1996-11-07 0h UT、到着 1997-09-12 0h UT、309 日）: v∞ 出発 3.1651 km/s、到着 2.8852 km/s。
/// 本の位置は JPL の暦、ここは Standish の解析式（火星で 10⁵ km 級の差）なので 0.015 km/s（0.5 %）まで許す
/// （実測の差は出発 3.1621（−3.0 m/s）、到着 2.8756（−9.6 m/s）km/s）。
#[test]
fn dated_transfer_matches_curtis_example_8_8() {
    let t = curtis();
    close(t.transfer_time_s / 86_400.0, 309.0, 1e-9);
    close(t.v_inf_departure_m_s.norm() / 1e3, 3.1651, 0.015);
    close(t.v_inf_arrival_m_s.norm() / 1e3, 2.8852, 0.015);
    assert_eq!(t.ephemeris, EphemerisSource::Analytic);
    assert_eq!(t.parent, CentralBody::Sun);
}

/// 手計算（本に依らない）。高度 300 km の円軌道（r = 6 678.137 km）から超過速度 v∞ で出る噴射は
/// `√(v∞² + 2μ/r) − √(μ/r)`。v∞ = 3 km/s なら `√(9e6 + 1.19380e8) − 7 725.76 = 11 330.26 − 7 725.76 = 3 604.50 m/s`。
/// 火星の高度 300 km の円軌道（r = 3 696.19 km、μ = 4.282837e13）への捕獲は v∞ = 2.5 km/s で
/// `√(6.25e6 + 2.317e7) − √(μ/r) = 5 424.4 − 3 404.0 ≈ 2 020 m/s`。楕円の捕獲軌道（遠点 20 000 km）は
/// 近点の速さが `√(μ(2/r_p − 1/a))` になる分だけ小さい。
#[test]
fn hyperbolic_periapsis_dv_matches_hand_calculation() {
    let r = R_EARTH + 300e3;
    let dv = hyperbolic_periapsis_dv_m_s(MU_EARTH, r, 3_000.0, leo());
    let by_hand = (9.0e6 + 2.0 * MU_EARTH / r).sqrt() - (MU_EARTH / r).sqrt();
    close(dv, by_hand, 1e-6);
    close(dv, 3_604.50, 0.01);
    let rm = R_MARS + 300e3;
    let dv_m = hyperbolic_periapsis_dv_m_s(GM_MARS, rm, 2_500.0, lmo());
    close(
        dv_m,
        (6.25e6 + 2.0 * GM_MARS / rm).sqrt() - (GM_MARS / rm).sqrt(),
        1e-6,
    );
    close(dv_m, 2_020.4, 0.5);
    let ellipse = Apsides::new(rm, 20_000e3);
    let a = ellipse.semi_major_axis_m();
    let v_p = (GM_MARS * (2.0 / rm - 1.0 / a)).sqrt();
    let dv_e = hyperbolic_periapsis_dv_m_s(GM_MARS, rm, 2_500.0, ellipse);
    close(dv_e, (6.25e6 + 2.0 * GM_MARS / rm).sqrt() - v_p, 1e-6);
    assert!(dv_e < dv_m);
}

/// 日付つきの遷移の噴射は、出発・到着の v∞ から手計算の式で出る。C3 = |v∞|²。
#[test]
fn burns_follow_the_asymptote_speeds() {
    let t = curtis();
    close(
        t.c3_m2_s2,
        t.v_inf_departure_m_s.dot(t.v_inf_departure_m_s),
        1e-3,
    );
    let (v_dep, arrival) = (t.v_inf_departure_m_s.norm(), t.arrival.expect("arrival"));
    assert_eq!(t.departure.central, CentralBody::Earth);
    close(t.departure.v_inf_m_s.expect("v_inf"), v_dep, 1e-9);
    let r = R_EARTH + 300e3;
    close(
        t.departure.delta_v_m_s,
        (v_dep * v_dep + 2.0 * MU_EARTH / r).sqrt() - (MU_EARTH / r).sqrt(),
        1e-6,
    );
    let (v_arr, rm) = (t.v_inf_arrival_m_s.norm(), R_MARS + 300e3);
    assert_eq!(arrival.central, CentralBody::Mars);
    close(arrival.v_inf_m_s.expect("v_inf"), v_arr, 1e-9);
    close(
        arrival.delta_v_m_s,
        (v_arr * v_arr + 2.0 * GM_MARS / rm).sqrt() - (GM_MARS / rm).sqrt(),
        1e-6,
    );
    // 出発の赤緯は地球の赤道（z 軸）に対する v∞ の仰角
    close(
        t.departure_declination_rad,
        (t.v_inf_departure_m_s.z / v_dep).asin(),
        1e-12,
    );
    assert!(t.departure_declination_rad.abs() < 0.7);
    // 捕獲しなければ到着の噴射は無い
    let flyby = dated_transfer(
        CentralBody::Earth,
        leo(),
        CentralBody::Mars,
        None,
        epoch("1996-11-07T00:00:00Z"),
        epoch("1997-09-12T00:00:00Z"),
    )
    .expect("flyby");
    assert!(flyby.arrival.is_none());
    assert_eq!(flyby.departure, t.departure);
}

/// 遷移の軌道（出発の日の状態）を太陽のまわりで二体問題として飛行時間だけ進めると、到着の日の火星の位置に着く。
/// 向きは黄道の北に対して順行。
#[test]
fn cruise_state_lands_on_the_arrival_body() {
    let t = curtis();
    let depart = epoch("1996-11-07T00:00:00Z");
    let arrive = epoch("1997-09-12T00:00:00Z");
    let mut el = osculating_elements(&t.cruise_state, GM_SUN).expect("elements");
    el.mean_anomaly_rad += mean_motion_rad_s(GM_SUN, el.a_m) * t.transfer_time_s;
    let end: OrbitState = elements_to_state(&el, GM_SUN);
    let target = CentralBody::Mars.state_about_parent(arrive).expect("mars");
    assert!(
        (end.r_eci - target.r_eci).norm() < 1e3,
        "miss {} m",
        (end.r_eci - target.r_eci).norm()
    );
    assert!((end.v_eci - target.v_eci - t.v_inf_arrival_m_s).norm() < 1e-3);
    let start = CentralBody::Earth
        .state_about_parent(depart)
        .expect("earth");
    assert_eq!(t.cruise_state.r_eci, start.r_eci);
    assert!((t.cruise_state.v_eci - start.v_eci - t.v_inf_departure_m_s).norm() < 1e-9);
    assert!(t.cruise_state.orbit_normal().dot(ecliptic_normal_eci()) > 0.9);
}

#[test]
fn rejects_bodies_that_are_not_siblings() {
    let (d, a) = (epoch("1996-11-07T00:00:00Z"), epoch("1997-09-12T00:00:00Z"));
    for (from, to) in [
        (CentralBody::Earth, CentralBody::Earth),
        (CentralBody::Moon, CentralBody::Mars),
        (CentralBody::Earth, CentralBody::Moon),
        (CentralBody::Sun, CentralBody::Earth),
        (CentralBody::Mars, CentralBody::Sun),
    ] {
        let e = dated_transfer(from, leo(), to, None, d, a).expect_err("not siblings");
        assert!(matches!(e, AstroError::NotSiblingBodies { .. }), "{e}");
        assert!(e.to_string().contains("same parent"));
    }
}

#[test]
fn rejects_arrival_not_after_departure() {
    let d = epoch("1996-11-07T00:00:00Z");
    for arrive in [d, epoch("1996-11-06T00:00:00Z")] {
        let e = dated_transfer(
            CentralBody::Earth,
            leo(),
            CentralBody::Mars,
            None,
            d,
            arrive,
        )
        .expect_err("order");
        assert!(
            matches!(e, AstroError::ArrivalNotAfterDeparture { .. }),
            "{e}"
        );
    }
}

#[test]
fn rejects_dates_outside_the_ephemeris() {
    for (d, a) in [
        ("1700-01-01T00:00:00Z", "1700-09-01T00:00:00Z"),
        ("2049-01-01T00:00:00Z", "2051-01-01T00:00:00Z"),
        ("2100-01-01T00:00:00Z", "2100-09-01T00:00:00Z"),
    ] {
        let e = dated_transfer(
            CentralBody::Earth,
            leo(),
            CentralBody::Mars,
            None,
            epoch(d),
            epoch(a),
        )
        .expect_err("outside");
        assert!(matches!(e, AstroError::OutsideEphemeris { .. }), "{e}");
    }
}

/// 解けない問題（飛行時間が 1 秒で収束しない）は Lambert のエラーがそのまま返る。
#[test]
fn passes_through_lambert_rejection() {
    let d = epoch("1996-11-07T00:00:00Z");
    let e = dated_transfer(
        CentralBody::Earth,
        leo(),
        CentralBody::Mars,
        None,
        d,
        d.plus_seconds(1.0),
    )
    .expect_err("one second to Mars");
    assert!(matches!(e, AstroError::InvalidInput(m) if m.contains("Lambert")));
    let bad = Apsides::circular(-1.0);
    assert!(
        dated_transfer(
            CentralBody::Earth,
            bad,
            CentralBody::Mars,
            None,
            d,
            d.plus_seconds(3e7)
        )
        .is_err()
    );
}

/// 巡航の軌道（`cruise_orbit`）は Lambert の弧そのもの: 元期（出発の日）に出発天体の位置にあり、飛行時間の後に到着天体の位置にあり、
/// 中心は共通の親（太陽）、伝搬は二体。位置は暦の天体の位置（`state_about_parent`）と比べる。
///
/// 許容は 1 m。Lambert は二分法を `1e-13` の幅まで詰める（飛行時間の誤差は 1e-13 × 2.9×10⁷ s ほど。速さ 30 km/s で 0.1 m 未満）。
/// 接触要素への往復と Kepler 方程式の反復の丸めはそれより小さい。
#[test]
fn cruise_orbit_passes_through_both_bodies_at_the_dates() {
    let depart = epoch("2028-12-10T00:00:00Z");
    let arrive = epoch("2029-09-25T00:00:00Z");
    let spec = cruise_orbit(CentralBody::Earth, CentralBody::Mars, depart, arrive).expect("orbit");
    let tof = arrive.seconds_since(depart);
    assert_eq!(spec.central, CentralBody::Sun);
    assert_eq!(spec.epoch, depart);
    let earth = CentralBody::Earth
        .state_about_parent(depart)
        .expect("earth");
    let mars = CentralBody::Mars.state_about_parent(arrive).expect("mars");
    let start = spec.state_at(0.0).expect("state");
    let end = spec.state_at(tof).expect("state");
    close((start.r_eci - earth.r_eci).norm(), 0.0, 1.0);
    close((end.r_eci - mars.r_eci).norm(), 0.0, 1.0);
    // 二体の軌道: 出発の状態は dated_transfer の巡航の状態と一致する（速度の差は丸めだけ）
    let dated = dated_transfer(
        CentralBody::Earth,
        leo(),
        CentralBody::Mars,
        Some(lmo()),
        depart,
        arrive,
    )
    .expect("transfer");
    close((start.v_eci - dated.cruise_state.v_eci).norm(), 0.0, 1e-6);
    assert_eq!(dated.transfer_time_s, tof);
}

/// 日心の距離の幾何の境界: 二体の楕円なので、弧の上の距離は近日点 `a(1-e)` と遠日点 `a(1+e)` のあいだにあり、両端の天体の距離も
/// 同じ範囲にある。弧が近日点・遠日点を通れば距離は両端の天体の距離の外へ出る（出発・到着の距離のあいだに収まるとは限らない）。
#[test]
fn cruise_distance_is_bounded_by_the_apsides() {
    for (a, b) in [
        ("1996-11-07T00:00:00Z", "1997-09-12T00:00:00Z"),
        ("2028-12-10T00:00:00Z", "2029-09-25T00:00:00Z"),
    ] {
        let (depart, arrive) = (epoch(a), epoch(b));
        let spec =
            cruise_orbit(CentralBody::Earth, CentralBody::Mars, depart, arrive).expect("orbit");
        let (a, e) = (
            spec.elements.semi_major_axis.value(),
            spec.elements.eccentricity,
        );
        let (q, big_q) = (a * (1.0 - e), a * (1.0 + e));
        let tof = arrive.seconds_since(depart);
        let r_at = |t: f64| spec.state_at(t).expect("state").r_eci.norm();
        for k in 0..=200 {
            let r = r_at(tof * f64::from(k) / 200.0);
            assert!(
                r >= q * (1.0 - 1e-9) && r <= big_q * (1.0 + 1e-9),
                "r = {r}"
            );
        }
    }
}

/// 双曲線の弧（短い飛行時間）は理由つきのエラーにする。
#[test]
fn a_hyperbolic_cruise_arc_is_an_error() {
    let depart = epoch("2028-12-10T00:00:00Z");
    let arrive = epoch("2029-01-20T00:00:00Z");
    let e = cruise_orbit(CentralBody::Earth, CentralBody::Mars, depart, arrive)
        .expect_err("hyperbolic");
    assert!(e.to_string().contains("not an ellipse"), "{e}");
}

fn distance_to(body: CentralBody, orbit: &crate::orbit::OrbitSpec, t: f64) -> f64 {
    let at = orbit.epoch.plus_seconds(t);
    let planet = body.state_about_parent(at).expect("state");
    (orbit.state_at(t).expect("state").r_eci - planet.r_eci).norm()
}

/// 巡航の区間は、出発の天体の影響圏を出てから到着の天体の影響圏に入るまで: 区間の始め（t = 0）で出発の天体との距離が
/// 影響圏の半径に等しく、終わり（t = 期間）で到着の天体との距離が影響圏の半径に等しい。許容は 100 m
/// （境の時刻は二分法で 1 ms まで詰め、探査機と天体の相対速度は最大でも 5×10⁴ m/s 未満なので距離の誤差は 50 m 未満）。
#[test]
fn the_cruise_leg_runs_from_one_sphere_of_influence_to_the_other() {
    let (depart, arrive) = (epoch("2028-12-10T00:00:00Z"), epoch("2029-09-25T00:00:00Z"));
    let leg = cruise_leg(CentralBody::Earth, CentralBody::Mars, depart, arrive).expect("leg");
    let soi_earth = CentralBody::Earth.soi_radius_m().expect("soi");
    let soi_mars = CentralBody::Mars.soi_radius_m().expect("soi");
    close(
        distance_to(CentralBody::Earth, &leg.orbit, 0.0),
        soi_earth,
        100.0,
    );
    close(
        distance_to(CentralBody::Mars, &leg.orbit, leg.span_s),
        soi_mars,
        100.0,
    );
    assert_eq!(leg.orbit.epoch, depart.plus_seconds(leg.leave_s));
    assert_eq!(leg.span_s, leg.enter_s - leg.leave_s);
    let tof = arrive.seconds_since(depart);
    assert!(0.0 < leg.leave_s && leg.leave_s < leg.enter_s && leg.enter_s < tof);
    // 影響圏の中にいる時間は日の単位（地球の影響圏は半径 9.2×10⁸ m 級、相対速度 3 km/s 級）
    assert!(leg.leave_s > 86_400.0 && leg.leave_s < 10.0 * 86_400.0);
}

/// 切り出した弧は同じ楕円（長半径・離心率・面）で、共通の時刻の位置は元の弧と同じ。許容は位置 1 mm
/// （同じ状態から要素へ往復するだけで、丸めは 1.5×10¹¹ m の 10⁻¹⁵ 倍級）。
#[test]
fn the_clipped_arc_is_the_same_ellipse_and_the_same_positions() {
    let (depart, arrive) = (epoch("2028-12-10T00:00:00Z"), epoch("2029-09-25T00:00:00Z"));
    let full = cruise_orbit(CentralBody::Earth, CentralBody::Mars, depart, arrive).expect("full");
    let leg = cruise_leg(CentralBody::Earth, CentralBody::Mars, depart, arrive).expect("leg");
    let (f, l) = (&full.elements, &leg.orbit.elements);
    close(
        l.semi_major_axis.value() / f.semi_major_axis.value(),
        1.0,
        1e-12,
    );
    close(l.eccentricity, f.eccentricity, 1e-12);
    close(l.inclination.value(), f.inclination.value(), 1e-12);
    close(l.raan.value(), f.raan.value(), 1e-12);
    close(l.arg_perigee.value(), f.arg_perigee.value(), 1e-9);
    for k in 0..=20 {
        let t = leg.span_s * f64::from(k) / 20.0;
        let a = leg.orbit.state_at(t).expect("clipped").r_eci;
        let b = full.state_at(leg.leave_s + t).expect("full").r_eci;
        assert!((a - b).norm() < 1e-3, "t = {t}: {}", (a - b).norm());
    }
}
