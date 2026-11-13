use super::*;
use sat_astro::constants::{AU, R_EARTH};
use sat_astro::orbit::{Propagator, j2_secular_rates, raan_from_ltdn, sso_inclination_rad};

fn orbit() -> OrbitSpec {
    let epoch = Epoch::parse("2027-04-01T00:00:00Z").unwrap();
    let mut orbit = OrbitSpec::circular(
        epoch,
        Qty::from_raw(550e3),
        Qty::from_raw(sso_inclination_rad(R_EARTH + 550e3, 0.0)),
        Qty::ZERO,
    );
    orbit.elements.raan = Qty::from_raw(raan_from_ltdn(epoch, 10.5).unwrap());
    orbit
}

#[test]
fn cylindrical_shadow_matches_hand_calculation() {
    // 円軌道の円柱影: acos(sqrt(h²+2Rh)/((R+h) cos β))/π × 周期。
    let mut o = orbit();
    o.propagator = Propagator::Kepler;
    let h: f64 = 550e3;
    let a = R_EARTH + h;
    let normal = o.state_at(0.0).unwrap().orbit_normal();
    let in_plane = o.state_at(0.0).unwrap().r_eci / a;
    for degrees in [0.0_f64, 30.0, 60.0] {
        let beta = degrees.to_radians();
        let sun = (in_plane * beta.cos() + normal * beta.sin()) * AU;
        let (umbra, penumbra) = shadow_durations(&o, 0.0, sun, ShadowModel::Cylindrical).unwrap();
        let fraction =
            ((h * h + 2.0 * R_EARTH * h).sqrt() / (a * beta.cos())).acos() / core::f64::consts::PI;
        assert!((umbra - fraction * o.period_s()).abs() <= 2.0 * SHADOW_STEP_S);
        assert_eq!(penumbra, 0.0);
        // 有限の太陽円盤による円錐本影は円柱より短い。550 km の差は30秒以内。
        let (conical, partial) = shadow_durations(&o, 0.0, sun, ShadowModel::Conical).unwrap();
        assert!((conical - fraction * o.period_s()).abs() < 30.0);
        assert!(conical <= umbra + SHADOW_STEP_S && partial > 0.0);
    }
}

#[test]
fn no_eclipse_above_critical_beta() {
    let mut o = orbit();
    o.propagator = Propagator::Kepler;
    // β=90°: 太陽は軌道の法線方向。円錐影の外に全周がある。
    let normal = o.state_at(0.0).unwrap().orbit_normal();
    assert_eq!(
        shadow_durations(&o, 0.0, normal * AU, ShadowModel::Conical).unwrap(),
        (0.0, 0.0)
    );
    // 円柱影の境界 asin(R/a) を 1° 超えたときも円錐影の外。
    let beta = (R_EARTH / (R_EARTH + 550e3)).asin() + 1.0_f64.to_radians();
    let direction = o.state_at(0.0).unwrap().r_eci / (R_EARTH + 550e3);
    assert_eq!(
        shadow_durations(
            &o,
            0.0,
            (direction * beta.cos() + normal * beta.sin()) * AU,
            ShadowModel::Conical
        )
        .unwrap(),
        (0.0, 0.0)
    );
}

#[test]
fn sso_year_and_solar_precession() {
    let o = orbit();
    // 平均太陽の年周運動: 360° / 365.2422 日 ≈ 0.9856°/日。
    let e = o.kepler_elements().unwrap();
    let rate = j2_secular_rates(e.a_m, e.e, e.i_rad).0.to_degrees() * DEFAULT_STEP_S;
    assert!((rate - 0.9856).abs() < 0.001);
    let result = EclipseSeason::run(&EclipseSeasonInput {
        orbit: o.clone(),
        duration: Qty::from_raw(365.25 * DEFAULT_STEP_S),
        step: None,
    })
    .unwrap();
    assert_eq!(result.points.len(), 367); // 始点、365日、端数の終点
    assert_eq!(
        result.series.t.last().copied(),
        Some(365.25 * DEFAULT_STEP_S)
    );
    assert!(result.beta_max.beta.value() - result.beta_min.beta.value() > 0.1);
    for p in &result.points {
        assert!((p.beta.value() - beta_angle_rad(&o, p.epoch).unwrap()).abs() < 1e-12);
        assert!(p.umbra.value() >= 0.0 && p.penumbra.value() >= 0.0);
    }
}

#[test]
fn invalid_inputs_and_non_earth_are_rejected() {
    let mut input = EclipseSeasonInput {
        orbit: orbit(),
        duration: Qty::from_raw(1.0),
        step: None,
    };
    input.orbit.central = CentralBody::Moon;
    assert!(EclipseSeason::run(&input).is_err());
    input.orbit.central = CentralBody::Earth;
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        input.step = None;
        input.duration = Qty::from_raw(bad);
        assert!(EclipseSeason::run(&input).is_err());
        input.duration = Qty::from_raw(1.0);
        input.step = Some(Qty::from_raw(bad));
        assert!(EclipseSeason::run(&input).is_err());
    }
}

#[test]
fn extrema_ties_and_eclipse_free_duration_by_hand() {
    let epoch = orbit().epoch;
    let points: Vec<_> = [(0.0, 0.0, 0.0), (10.0, -1.0, 5.0), (20.0, 1.0, 5.0)]
        .into_iter()
        .map(|(t, beta, umbra)| EclipseSeasonPoint {
            epoch: epoch.plus_seconds(t),
            beta: Qty::from_raw(beta),
            umbra: Qty::from_raw(umbra),
            penumbra: Qty::ZERO,
        })
        .collect();
    let out = summarize(points, vec![0.0, 10.0, 20.0], 10.0).unwrap();
    assert_eq!(out.longest.epoch, epoch.plus_seconds(10.0));
    assert_eq!(out.beta_min.epoch, epoch.plus_seconds(10.0));
    assert_eq!(out.beta_max.epoch, epoch.plus_seconds(20.0));
    assert_eq!(out.max_eclipse_duration.value(), 5.0);
    assert_eq!(out.eclipse_free_samples, 1);
    // フラグ [1,0,0] の台形積算: (1+0)/2 ×10 + (0+0)/2 ×10 = 5秒。
    assert_eq!(out.eclipse_free_duration.value(), 5.0);
}
