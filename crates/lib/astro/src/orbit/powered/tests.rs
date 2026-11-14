//! 推力を入れた伝搬を、推力の小さい極限の解析式（Edelbaum・ロケット方程式）と比べる。

use super::*;
use crate::constants::{MU_EARTH, R_EARTH};
use crate::orbit::{KeplerElements, elements_to_state};
use crate::rocket::{exhaust_velocity, propellant_for};
use crate::transfer::edelbaum;

const A0: f64 = R_EARTH + 550e3;
const A1: f64 = R_EARTH + 600e3;

fn start(i_deg: f64) -> OrbitState {
    elements_to_state(
        &KeplerElements {
            a_m: A0,
            e: 0.0,
            i_rad: i_deg.to_radians(),
            raan_rad: 0.0,
            argp_rad: 0.0,
            mean_anomaly_rad: 0.0,
        },
        MU_EARTH,
    )
}

/// 推力 10 mN・質量 10 kg（1e-3 m/s²。局所の重力の 1e-4 倍）・比推力 1200 s。
fn arc(sunlit: bool) -> ThrustArc {
    ThrustArc {
        thrust_n: 0.01,
        exhaust_velocity_m_s: exhaust_velocity(1200.0),
        steering: ThrustSteering::Tangential,
        min_sunlit_factor: sunlit.then_some(0.5),
        duty: 1.0,
        control_step_s: 60.0,
        max_duration_s: 10.0 * 86_400.0,
    }
}

fn fly(
    gravity: GravityModel,
    x0: OrbitState,
    arc: &ThrustArc,
    target: PoweredTarget,
) -> PoweredArc {
    let forces = ForceConfig::gravity_only(gravity);
    propagate_powered(
        CentralBody::Earth,
        &forces,
        Epoch { j2000_s: 0.0 },
        x0,
        10.0,
        arc,
        &target,
    )
    .unwrap()
}

/// 共面の上昇: ΔV は Edelbaum（= |v₀ − v₁|）と 0.5 % 以内、推薬はその ΔV のロケット方程式と 1e-6 以内。
#[test]
fn coplanar_spiral_matches_edelbaum_in_the_small_thrust_limit() {
    let target = PoweredTarget {
        semi_major_axis_m: A1,
        inclination_rad: 0.0,
    };
    let out = fly(GravityModel::PointMass, start(0.0), &arc(false), target);
    let analytic = edelbaum(MU_EARTH, A0, A1, 0.0).unwrap();
    assert!(out.reached);
    let rel = (out.delta_v_m_s - analytic).abs() / analytic;
    assert!(rel < 5e-3, "{} vs {analytic} ({rel})", out.delta_v_m_s);
    let mp = propellant_for(exhaust_velocity(1200.0), 10.0, out.delta_v_m_s).unwrap();
    assert!(
        (out.propellant_kg - mp).abs() < 1e-6 * mp,
        "{} vs {mp}",
        out.propellant_kg
    );
    // 噴き続けるので噴射時間 = 時間、質量の減りは流量 × 時間
    assert!((out.firing_time_s - out.duration_s).abs() < 1e-6);
    assert!((out.propellant_kg - 0.01 / exhaust_velocity(1200.0) * out.duration_s).abs() < 1e-9);
    let end = out.samples.last().unwrap();
    assert!((end.mean_semi_major_axis_m - A1).abs() < 1.0);
    assert!(end.eccentricity < 1e-3, "{}", end.eccentricity);
}

/// 高度と傾斜角（1°）を同時に変える: Edelbaum の偏揺れ角で、ΔV は Edelbaum と 2 % 以内、傾斜角は 0.05° 以内。
#[test]
fn combined_raise_and_plane_change_matches_edelbaum() {
    let di = 1.0_f64.to_radians();
    let i0 = 30.0_f64.to_radians();
    let target = PoweredTarget {
        semi_major_axis_m: A1,
        inclination_rad: i0 + di,
    };
    let out = fly(GravityModel::PointMass, start(30.0), &arc(false), target);
    let analytic = edelbaum(MU_EARTH, A0, A1, di).unwrap();
    let rel = (out.delta_v_m_s - analytic).abs() / analytic;
    assert!(rel < 2e-2, "{} vs {analytic} ({rel})", out.delta_v_m_s);
    let end = out.samples.last().unwrap();
    assert!(
        (end.inclination_rad - (i0 + di)).abs() < 0.05_f64.to_radians(),
        "{}",
        end.inclination_rad.to_degrees()
    );
}

/// 日照中だけ噴くと、時間は長く（噴いた時間の割合が日照の割合 ~0.6）、ΔV はほぼ同じ（2 % 以内）。
#[test]
fn sunlit_only_takes_longer_for_the_same_delta_v() {
    let target = PoweredTarget {
        semi_major_axis_m: A1,
        inclination_rad: 0.0,
    };
    let all = fly(GravityModel::PointMass, start(0.0), &arc(false), target);
    let lit = fly(GravityModel::PointMass, start(0.0), &arc(true), target);
    assert!(lit.reached);
    assert!(
        lit.duration_s > 1.4 * all.duration_s,
        "{} vs {}",
        lit.duration_s,
        all.duration_s
    );
    let fraction = lit.firing_time_s / lit.duration_s;
    assert!((0.55..0.75).contains(&fraction), "{fraction}");
    assert!((lit.delta_v_m_s - all.delta_v_m_s).abs() < 0.02 * all.delta_v_m_s);
}

/// J2 があっても、平均の長半径で判定するので目標に着く（接触の長半径の振れ ±9 km に引きずられない）。
#[test]
fn j2_raise_stops_on_the_mean_semi_major_axis() {
    let target = PoweredTarget {
        semi_major_axis_m: A1,
        inclination_rad: 97.6_f64.to_radians(),
    };
    let x0 = start(97.6);
    let out = fly(GravityModel::J2, x0, &arc(false), target);
    // 元期の接触の長半径 A0（昇交点）の平均は ~9 km 低いので、平均どうしで比べる
    let a0 = mean_semi_major_axis_m(CentralBody::Earth, &x0, true).unwrap();
    assert!((A0 - a0 - 9.4e3).abs() < 0.5e3, "{}", A0 - a0);
    let analytic = edelbaum(MU_EARTH, a0, A1, 0.0).unwrap();
    assert!(
        (out.delta_v_m_s - analytic).abs() < 1e-2 * analytic,
        "{} vs {analytic}",
        out.delta_v_m_s
    );
}

/// デューティ 0.5 なら噴く時間は半分、時間は ~2 倍。上限の時間までに届かなければ reached = false。
#[test]
fn duty_halves_the_firing_and_the_cap_stops_early() {
    let target = PoweredTarget {
        semi_major_axis_m: A1,
        inclination_rad: 0.0,
    };
    let half = ThrustArc {
        duty: 0.5,
        ..arc(false)
    };
    let out = fly(GravityModel::PointMass, start(0.0), &half, target);
    assert!((out.firing_time_s / out.duration_s - 0.5).abs() < 0.01);
    let short = ThrustArc {
        max_duration_s: 3600.0,
        ..arc(false)
    };
    let out = fly(GravityModel::PointMass, start(0.0), &short, target);
    assert!(!out.reached);
    assert!((out.duration_s - 3600.0).abs() < 1e-9);
}

/// 月を回る軌道（高度 100 → 150 km、点質量）でも、ΔV は月の μ の Edelbaum と 0.5 % 以内。
#[test]
fn lunar_spiral_matches_edelbaum_about_the_moon() {
    let moon = CentralBody::Moon;
    let (a0, a1) = (moon.radius_m() + 100e3, moon.radius_m() + 150e3);
    let el = KeplerElements {
        a_m: a0,
        e: 0.0,
        i_rad: 0.3,
        raan_rad: 0.0,
        argp_rad: 0.0,
        mean_anomaly_rad: 0.0,
    };
    let x0 = elements_to_state(&el, moon.gm());
    let forces = ForceConfig::gravity_only(GravityModel::PointMass);
    let target = PoweredTarget {
        semi_major_axis_m: a1,
        inclination_rad: 0.3,
    };
    let out = propagate_powered(
        moon,
        &forces,
        Epoch { j2000_s: 0.0 },
        x0,
        10.0,
        &arc(false),
        &target,
    )
    .unwrap();
    let analytic = edelbaum(moon.gm(), a0, a1, 0.0).unwrap();
    assert!(out.reached);
    assert!(
        (out.delta_v_m_s - analytic).abs() < 5e-3 * analytic,
        "{} vs {analytic}",
        out.delta_v_m_s
    );
}
