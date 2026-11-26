//! スイングバイ（`flyby_geometry`・`dated_transfer_via`）の手計算との比較と、保存則・つなぎの確認。暦は解析式。

use super::*;
use crate::constants::{R_EARTH, R_MARS};
use crate::ephem::CentralBody;
use crate::math::Vec3;
use crate::time::Epoch;
use core::f64::consts::PI;

fn epoch(s: &str) -> Epoch {
    Epoch::parse(s).expect("epoch")
}

fn venus_flyby(v_in: Vec3, v_out: Vec3) -> FlybyPass {
    flyby_geometry(
        CentralBody::Venus,
        epoch("2030-01-01T00:00:00Z"),
        v_in,
        v_out,
        300e3,
    )
    .expect("flyby")
}

/// 動力なし: v∞ = 5 km/s の両側で 60° 曲がる。`sin(δ/2) = 1/(1 + r_p v∞²/μ)` で `sin 30° = 0.5` なので `r_p v∞²/μ = 1`、
/// つまり `r_p = μ / v∞²`（金星の μ = 3.2486e14 m³/s² なら 3.2486e14 / 25e6 = 1.2994e7 m。高度は 1.2994e7 − 6.0518e6 ≈ 6.94e6 m）。
#[test]
fn ballistic_turn_of_sixty_degrees_matches_hand_calculation() {
    let mu = CentralBody::Venus.gm();
    let v = 5e3;
    let v_in = Vec3::new(v, 0.0, 0.0);
    let v_out = Vec3::new(v * (PI / 3.0).cos(), v * (PI / 3.0).sin(), 0.0);
    let f = venus_flyby(v_in, v_out);
    assert!((f.turn_angle_rad - PI / 3.0).abs() < 1e-12);
    let r_p = f.periapsis_radius_m.expect("periapsis");
    let want = mu / (v * v);
    assert!((r_p - want).abs() / want < 1e-10, "{r_p} vs {want}");
    assert!(f.feasible && f.powered_dv_m_s.abs() < 1e-6 && f.shortfall_dv_m_s == 0.0);
    assert!(f.is_ballistic());
}

/// 動力あり: 入り 5 km/s・出 7 km/s で近点が 1.0e7 m の双曲線の組。偏向角は両側の `asin(1/(1 + r_p v∞²/μ))` の和で、
/// これをそのまま与えると近点の半径 1.0e7 m が戻り、噴射は近点の速さの差 `√(v₂² + 2μ/r_p) − √(v₁² + 2μ/r_p)`。
#[test]
fn powered_flyby_solves_the_periapsis_and_burns_the_speed_difference() {
    let mu = CentralBody::Venus.gm();
    let r_p = 1.0e7;
    let (a, b) = (5e3, 7e3);
    let half = |v: f64| (1.0 / (1.0 + r_p * v * v / mu)).asin();
    let delta = half(a) + half(b);
    let v_in = Vec3::new(a, 0.0, 0.0);
    let v_out = Vec3::new(b * delta.cos(), b * delta.sin(), 0.0);
    let f = venus_flyby(v_in, v_out);
    let got = f.periapsis_radius_m.expect("periapsis");
    assert!((got - r_p).abs() / r_p < 1e-10, "{got}");
    let want = (b * b + 2.0 * mu / r_p).sqrt() - (a * a + 2.0 * mu / r_p).sqrt();
    assert!(
        (f.powered_dv_m_s - want).abs() < 1e-6,
        "{} vs {want}",
        f.powered_dv_m_s
    );
    assert!(f.feasible && !f.is_ballistic());
}

/// 近点の高度の下限（300 km）の半径でも曲がりきれない偏向: 下限の半径での最大の偏向角は
/// `2 asin(1/(1 + r_min v∞²/μ))`。足りない偏向 `δ − δ_max` の分は弦 `2 v∞ sin(Δ/2)`。必要な近点の半径は下限より小さい。
#[test]
fn a_turn_beyond_the_maximum_is_infeasible_and_reports_the_shortfall() {
    let mu = CentralBody::Venus.gm();
    let v = 5e3;
    let delta = 170.0_f64.to_radians();
    let v_in = Vec3::new(v, 0.0, 0.0);
    let v_out = Vec3::new(v * delta.cos(), v * delta.sin(), 0.0);
    let f = venus_flyby(v_in, v_out);
    let r_min = CentralBody::Venus.radius_m() + 300e3;
    let max_turn = 2.0 * (1.0 / (1.0 + r_min * v * v / mu)).asin();
    assert!((f.max_turn_rad - max_turn).abs() < 1e-12);
    assert!(!f.feasible && delta > max_turn);
    assert!(f.periapsis_radius_m.expect("required radius") < r_min);
    let want = 2.0 * v * (0.5 * (delta - max_turn)).sin();
    assert!((f.shortfall_dv_m_s - want).abs() < 1e-6);
    assert!(
        f.powered_dv_m_s.abs() < 1e-6,
        "equal speeds burn nothing at periapsis"
    );
    assert!((f.total_dv_m_s() - want).abs() < 1e-6);
}

#[test]
fn invalid_inputs_are_refused() {
    let t = epoch("2030-01-01T00:00:00Z");
    let v = Vec3::new(5e3, 0.0, 0.0);
    assert!(flyby_geometry(CentralBody::Venus, t, v, v, 0.0).is_err());
    assert!(flyby_geometry(CentralBody::Venus, t, v, v, -1.0).is_err());
    assert!(flyby_geometry(CentralBody::Venus, t, v, Vec3::ZERO, 300e3).is_err());
}

#[test]
fn default_altitudes_exist_for_the_rocky_planets_only() {
    for b in [
        CentralBody::Mercury,
        CentralBody::Venus,
        CentralBody::Earth,
        CentralBody::Mars,
    ] {
        assert!(default_min_altitude_m(b).is_some_and(|h| h > 0.0), "{b:?}");
    }
    for b in [
        CentralBody::Jupiter,
        CentralBody::Saturn,
        CentralBody::Uranus,
        CentralBody::Neptune,
        CentralBody::Sun,
        CentralBody::Moon,
    ] {
        assert_eq!(default_min_altitude_m(b), None, "{b:?}");
    }
}

/// 同じ円錐曲線の上の 3 点（出発・通過・到着）を 2 本の Lambert でつなぐと、通過で速度は連続（偏向角 ≈ 0、噴射 ≈ 0）。
/// 許容は Lambert の解の許容（`z` の二分法 1e-13 の相対）から、速度で 1e-5 m/s（実測はこれより十分小さい）。
#[test]
fn legs_on_one_conic_pass_the_body_without_turning() {
    let (t0, t1, t2) = (
        epoch("2029-01-01T00:00:00Z"),
        epoch("2029-06-01T00:00:00Z"),
        epoch("2029-11-01T00:00:00Z"),
    );
    let orbit = cruise_orbit(
        CentralBody::Earth,
        CentralBody::Mars,
        epoch("2028-12-01T00:00:00Z"),
        epoch("2029-09-01T00:00:00Z"),
    )
    .expect("orbit");
    let at = |t: Epoch| orbit.state_at(t.seconds_since(orbit.epoch)).expect("state");
    let (s0, s1, s2) = (at(t0), at(t1), at(t2));
    let mu = crate::constants::GM_SUN;
    let n = crate::ephem::ecliptic_normal_eci();
    let leg =
        |a: Vec3, b: Vec3, tof: f64| lambert_about_normal(mu, a, b, tof, true, n).expect("lambert");
    let l1 = leg(s0.r_eci, s1.r_eci, t1.seconds_since(t0));
    let l2 = leg(s1.r_eci, s2.r_eci, t2.seconds_since(t1));
    let body_v = Vec3::new(0.0, 35e3, 0.0);
    let f = flyby_geometry(
        CentralBody::Venus,
        t1,
        l1.v2_m_s - body_v,
        l2.v1_m_s - body_v,
        300e3,
    )
    .expect("flyby");
    assert!(f.turn_angle_rad < 1e-9, "turn {}", f.turn_angle_rad);
    assert!(f.powered_dv_m_s < 1e-5, "dv {}", f.powered_dv_m_s);
    // 連続な速度は軌道の速度そのもの
    assert!((l1.v2_m_s - s1.v_eci).norm() < 1e-5);
}

fn earth_venus_mars(flybys: &[FlybyStop]) -> DatedTransfer {
    dated_transfer_via(
        CentralBody::Earth,
        Apsides::circular(R_EARTH + 300e3),
        CentralBody::Mars,
        Some(Apsides::circular(R_MARS + 300e3)),
        epoch("2028-10-01T00:00:00Z"),
        epoch("2029-12-01T00:00:00Z"),
        flybys,
    )
    .expect("transfer")
}

fn venus_stop() -> FlybyStop {
    FlybyStop {
        body: CentralBody::Venus,
        date: epoch("2029-02-01T00:00:00Z"),
        min_altitude_m: None,
    }
}

/// 通過がなければ `dated_transfer` と同じ（同じ式の経路）で、脚は 1 本。
#[test]
fn zero_flybys_equal_the_direct_transfer() {
    let args = (
        CentralBody::Earth,
        Apsides::circular(R_EARTH + 300e3),
        CentralBody::Mars,
        Some(Apsides::circular(R_MARS + 300e3)),
        epoch("2028-10-01T00:00:00Z"),
        epoch("2029-12-01T00:00:00Z"),
    );
    let direct = dated_transfer(args.0, args.1, args.2, args.3, args.4, args.5).expect("direct");
    assert_eq!(direct, earth_venus_mars(&[]));
    assert_eq!(direct.legs.len(), 1);
    assert!(direct.flybys.is_empty() && direct.flyby_dv_m_s() == 0.0);
}

/// 保存則: 通過の前後で日心の速度の変化は超過速度ベクトルの差に等しい（天体の運動量。天体の速度は引き算で消える）。
/// 動力なしの通過なら `|v∞|` は両側で等しく、動力あり・なしにかかわらず噴射は近点の速さの差。
#[test]
fn heliocentric_velocity_change_is_the_difference_of_the_excess_velocities() {
    let t = earth_venus_mars(&[venus_stop()]);
    assert_eq!(t.legs.len(), 2);
    let f = t.flybys[0];
    let helio = t.legs[1].v_departure_m_s - t.legs[0].v_arrival_m_s;
    let excess = f.v_inf_out_m_s - f.v_inf_in_m_s;
    assert!((helio - excess).norm() < 1e-6, "{helio:?} vs {excess:?}");
    // 脚の接続: 前の脚の到着の位置 = 次の脚の出発の位置（通過の天体の位置）
    assert!((t.legs[0].to_state.r_eci - t.legs[1].from_state.r_eci).norm() < 1e-6);
    // 噴射 = 近点の速さの差
    if let Some(r_p) = f.periapsis_radius_m.filter(|_| f.feasible) {
        let mu = CentralBody::Venus.gm();
        let want = ((f.v_inf_out_m_s.norm_squared() + 2.0 * mu / r_p).sqrt()
            - (f.v_inf_in_m_s.norm_squared() + 2.0 * mu / r_p).sqrt())
        .abs();
        assert!((f.powered_dv_m_s - want).abs() < 1e-6);
    }
    // 出発・到着の超過速度は最初・最後の脚から
    assert!(
        (t.v_inf_departure_m_s - (t.legs[0].v_departure_m_s - t.legs[0].from_state.v_eci)).norm()
            < 1e-9
    );
    assert!(
        (t.v_inf_arrival_m_s - (t.legs[1].v_arrival_m_s - t.legs[1].to_state.v_eci)).norm() < 1e-9
    );
    assert!(
        (t.transfer_time_s
            - epoch("2029-12-01T00:00:00Z").seconds_since(epoch("2028-10-01T00:00:00Z")))
        .abs()
            < 1e-6
    );
}

#[test]
fn flyby_dates_must_be_ordered_and_bodies_must_differ() {
    let early = FlybyStop {
        date: epoch("2028-09-01T00:00:00Z"),
        ..venus_stop()
    };
    let late = FlybyStop {
        date: epoch("2030-01-01T00:00:00Z"),
        ..venus_stop()
    };
    let same = FlybyStop {
        body: CentralBody::Earth,
        ..venus_stop()
    };
    let moon = FlybyStop {
        body: CentralBody::Moon,
        ..venus_stop()
    };
    let giant = FlybyStop {
        body: CentralBody::Jupiter,
        ..venus_stop()
    };
    let run = |s: &[FlybyStop]| {
        dated_transfer_via(
            CentralBody::Earth,
            Apsides::circular(R_EARTH + 300e3),
            CentralBody::Mars,
            None,
            epoch("2028-10-01T00:00:00Z"),
            epoch("2029-12-01T00:00:00Z"),
            s,
        )
    };
    assert!(run(&[early]).is_err() && run(&[late]).is_err());
    assert!(run(&[same]).is_err() && run(&[moon]).is_err());
    // 木星は高度の下限の慣例が無いので、書かなければエラー
    assert!(run(&[giant]).is_err());
    let near = FlybyStop {
        date: epoch("2029-02-01T00:00:00Z"),
        min_altitude_m: Some(1.0e6),
        ..giant
    };
    assert!(run(&[near]).is_ok());
    // 2 回の通過の日が逆順
    let a = venus_stop();
    let b = FlybyStop {
        body: CentralBody::Earth,
        date: epoch("2029-01-01T00:00:00Z"),
        ..a
    };
    assert!(run(&[a, b]).is_err());
}
