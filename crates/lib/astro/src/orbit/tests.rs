use super::*;
use crate::constants::{MU_EARTH, R_EARTH};
use crate::force::{ForceConfig, GravityModel};

fn deg(v: f64) -> Qty<Angle> {
    Qty::from_raw(v.to_radians())
}

fn km(v: f64) -> Qty<Length> {
    Qty::from_raw(v * 1e3)
}

fn epoch() -> Epoch {
    Epoch::parse("2027-04-01T00:00:00Z").expect("元期")
}

#[test]
fn iss_like_period_is_about_93_minutes() {
    let spec = OrbitSpec::circular(epoch(), km(420.0), deg(51.6), deg(0.0));
    let p = spec.period_s() / 60.0;
    assert!((p - 92.97).abs() < 0.02, "{p} min");
}

#[test]
fn sso_inclination_at_550_km() {
    let i = sso_inclination_rad(R_EARTH + 550e3, 0.0).to_degrees();
    assert!((i - 97.59).abs() < 0.03, "{i}°");
    // 800 km では ≈ 98.6°（Wertz の表）
    let i800 = sso_inclination_rad(R_EARTH + 800e3, 0.0).to_degrees();
    assert!((i800 - 98.60).abs() < 0.05, "{i800}°");
    assert!(sso_inclination_rad(R_EARTH * 20.0, 0.0).is_nan());
}

#[test]
fn sso_raan_rate_matches_the_sun() {
    let a = R_EARTH + 550e3;
    let i = sso_inclination_rad(a, 0.0);
    let (raan_dot, _, _) = j2_secular_rates(a, 0.0, i);
    let deg_per_day = raan_dot.to_degrees() * 86_400.0;
    assert!((deg_per_day - 0.985_6).abs() < 1e-3, "{deg_per_day}");
}

#[test]
fn circular_orbit_state_has_constant_radius_and_speed() {
    let spec = OrbitSpec::circular(epoch(), km(550.0), deg(97.6), deg(30.0));
    let a = R_EARTH + 550e3;
    let v_circ = (MU_EARTH / a).sqrt();
    for t in [0.0, 1000.0, 5000.0, 86_400.0] {
        let s = spec.state_at(t).expect("状態");
        assert!((s.r_eci.norm() - a).abs() < 1e-6);
        assert!((s.v_eci.norm() - v_circ).abs() < 1e-9);
        assert!(s.r_eci.dot(s.v_eci).abs() < 1e-3);
    }
    // 元期は昇交点（z = 0、z 方向の速度が正）
    let s0 = spec.state_at(0.0).expect("状態");
    assert!(s0.r_eci.z.abs() < 1e-6 && s0.v_eci.z > 0.0);
}

#[test]
fn kepler_returns_after_one_period() {
    let mut spec = OrbitSpec::circular(epoch(), km(700.0), deg(45.0), deg(10.0));
    spec.elements.eccentricity = 0.05;
    spec.elements.arg_perigee = deg(40.0);
    spec.elements.mean_anomaly = deg(123.0);
    spec.propagator = Propagator::Kepler;
    let s0 = spec.state_at(0.0).expect("状態");
    let s1 = spec.state_at(spec.period_s()).expect("状態");
    assert!((s0.r_eci - s1.r_eci).norm() < 1e-4);
}

#[test]
fn elements_state_round_trip() {
    let el = KeplerElements {
        a_m: 7.2e6,
        e: 0.03,
        i_rad: 0.9,
        raan_rad: 2.0,
        argp_rad: 0.7,
        mean_anomaly_rad: 4.0,
    };
    let s = elements_to_state(&el, MU_EARTH);
    let back = osculating_elements(&s, MU_EARTH).expect("楕円");
    assert!((back.a_m - el.a_m).abs() < 1e-4);
    assert!((back.e - el.e).abs() < 1e-12);
    assert!((back.i_rad - el.i_rad).abs() < 1e-12);
    assert!((back.raan_rad - el.raan_rad).abs() < 1e-12);
    assert!((back.argp_rad - el.argp_rad).abs() < 1e-9);
    assert!((back.mean_anomaly_rad - el.mean_anomaly_rad).abs() < 1e-9);
}

#[test]
fn j2_drifts_raan_westward_for_prograde() {
    let spec = OrbitSpec::circular(epoch(), km(400.0), deg(51.6), deg(0.0));
    let el = spec.mean_elements_at(86_400.0).expect("要素");
    // ISS: ≈ −5.0°/日
    let d = (el.raan_rad - TAU).to_degrees();
    assert!((d + 5.0).abs() < 0.2, "{d}");
}

#[test]
fn invalid_elements_are_rejected() {
    let mut spec = OrbitSpec::circular(epoch(), km(550.0), deg(97.6), deg(0.0));
    spec.elements.eccentricity = 1.2;
    assert!(spec.state_at(0.0).is_err());
    spec.elements.eccentricity = 0.0;
    spec.elements.semi_major_axis = Qty::from_raw(-1.0);
    assert!(spec.states_at(&[0.0]).is_err());
}

#[test]
fn beta_angle_of_dawn_dusk_sso_is_large() {
    // 春分の頃、太陽はほぼ +x。RAAN = 90°（LTAN 18:00）の SSO は軌道面法線が太陽を向き β ≈ 60〜90°。
    let e = Epoch::parse("2027-03-21T00:00:00Z").expect("元期");
    let i = sso_inclination_rad(R_EARTH + 550e3, 0.0);
    let spec = OrbitSpec::circular(e, km(550.0), Qty::from_raw(i), deg(90.0));
    let beta = beta_angle_rad(&spec, e).expect("β").to_degrees().abs();
    assert!(beta > 80.0, "{beta}");
    // RAAN = 0（正午・真夜中）なら β ≈ 0
    let noon = OrbitSpec::circular(e, km(550.0), Qty::from_raw(i), deg(0.0));
    let beta0 = beta_angle_rad(&noon, e).expect("β").to_degrees();
    assert!(beta0.abs() < 2.0, "{beta0}");
}

#[test]
fn numerical_point_mass_matches_kepler() {
    let mut spec = OrbitSpec::circular(epoch(), km(550.0), deg(97.6), deg(20.0));
    spec.elements.eccentricity = 0.01;
    spec.propagator = Propagator::Kepler;
    let times: Vec<f64> = (0..=48).map(|i| f64::from(i) * 1800.0).collect();
    let kepler = spec.states_at(&times).expect("Kepler");
    spec.propagator = Propagator::Numerical {
        forces: ForceConfig::gravity_only(GravityModel::PointMass),
    };
    let numerical = spec.states_at(&times).expect("数値伝搬");
    for (k, n) in kepler.iter().zip(numerical.iter()) {
        let d = (k.r_eci - n.r_eci).norm();
        assert!(d < 0.05, "位置の差 {d} m");
    }
    let one = spec.state_at(times[10]).expect("1 点");
    assert!((one.r_eci - kepler[10].r_eci).norm() < 0.05);
}

#[test]
fn numerical_j2_raan_drift_matches_secular_rate() {
    // 1 日の数値積分（J2 の力）で軌道面法線の回転が永年項の RAAN 変化率と合う（短周期を除いて ~1%）。
    let spec_j2 = OrbitSpec::circular(epoch(), km(550.0), deg(60.0), deg(0.0));
    let mut spec_num = spec_j2.clone();
    spec_num.propagator = Propagator::Numerical {
        forces: ForceConfig::gravity_only(GravityModel::J2),
    };
    let t = 86_400.0;
    let s = spec_num.state_at(t).expect("数値伝搬");
    let n = s.orbit_normal();
    let raan_num = n.x.atan2(-n.y);
    let raan_sec = spec_j2.mean_elements_at(t).expect("要素").raan_rad;
    let diff =
        (raan_num - raan_sec + core::f64::consts::PI).rem_euclid(TAU) - core::f64::consts::PI;
    assert!(
        diff.to_degrees().abs() < 0.03,
        "RAAN の差 {}°",
        diff.to_degrees()
    );
}

#[test]
fn raan_for_beta_round_trips_with_beta_angle() {
    let e = epoch();
    let at = e.plus_seconds(40.0 * 86_400.0);
    let i = sso_inclination_rad(R_EARTH + 550e3, 0.0);
    for propagator in [Propagator::J2, Propagator::Kepler] {
        let mut spec = OrbitSpec::circular(e, km(550.0), Qty::from_raw(i), deg(0.0));
        spec.propagator = propagator.clone();
        for beta_deg in [-60.0, -20.0, 0.0, 15.0, 45.0, 62.0] {
            let raan = raan_for_beta(&spec, at, f64::to_radians(beta_deg)).expect("届く β");
            spec.elements.raan = Qty::from_raw(raan);
            let beta = beta_angle_rad(&spec, at).expect("β").to_degrees();
            assert!(
                (beta - beta_deg).abs() < 1e-9,
                "{propagator:?}: {beta} vs {beta_deg}"
            );
        }
    }
    // 5 月（δ ≈ 17.8°）の SSO では β の上限は δ + i − 90° ≈ 64.6° なので 75° には届かない
    let sso = OrbitSpec::circular(e, km(550.0), Qty::from_raw(i), deg(0.0));
    assert!(raan_for_beta(&sso, at, f64::to_radians(75.0)).is_err());
    // 低傾斜では大きな β に届かない
    let low = OrbitSpec::circular(e, km(550.0), deg(10.0), deg(0.0));
    assert!(raan_for_beta(&low, at, f64::to_radians(80.0)).is_err());
}

#[test]
fn raan_for_beta_zero_is_noon_side() {
    // β = 0 の解は昇交点が太陽の赤経の近く（正午側）
    let e = epoch();
    let i = sso_inclination_rad(R_EARTH + 550e3, 0.0);
    let spec = OrbitSpec::circular(e, km(550.0), Qty::from_raw(i), deg(0.0));
    let raan = raan_for_beta(&spec, e, 0.0).expect("β = 0");
    let s = crate::ephem::sun_direction_eci(e);
    let d = (raan - s.y.atan2(s.x) + core::f64::consts::PI).rem_euclid(TAU) - core::f64::consts::PI;
    assert!(d.abs().to_degrees() < 10.0, "{}", d.to_degrees());
}
