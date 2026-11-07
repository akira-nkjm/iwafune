use super::*;
use crate::constants::R_EARTH;
use crate::ephem::sun_direction_eci;
use crate::orbit::{Propagator, beta_angle_rad};

fn polar_orbit_with_beta_zero() -> OrbitSpec {
    let epoch = Epoch::parse("2027-06-01T00:00:00Z").expect("元期");
    let s = sun_direction_eci(epoch);
    let raan = s.y.atan2(s.x);
    let mut orbit = OrbitSpec::circular(
        epoch,
        Qty::from_raw(550e3),
        Qty::from_raw(90f64.to_radians()),
        Qty::from_raw(raan),
    );
    orbit.propagator = Propagator::Kepler;
    orbit
}

fn spec(shadow: ShadowModel) -> TimelineSpec {
    let orbit = polar_orbit_with_beta_zero();
    let span = orbit.period_s();
    TimelineSpec {
        orbit,
        attitude: AttitudeLaw::SunPointing {
            axis: Vec3::Z,
            secondary: Vec3::Y,
        },
        span: Qty::from_raw(span),
        step: Qty::from_raw(5.0),
        shadow,
        beta: None,
    }
}

/// 550 km 円軌道・β = 0 の日陰率: 円柱影で (1/π)·acos(√(h²+2Rh)/r) ≈ 0.372。
#[test]
fn eclipse_fraction_at_550_km_beta_zero() {
    let sp = spec(ShadowModel::Cylindrical);
    let beta = beta_angle_rad(&sp.orbit, sp.orbit.epoch).expect("β");
    assert!(beta.abs() < 1e-6, "β = {beta}");
    let h = 550e3;
    let r = R_EARTH + h;
    let expected = ((h * h + 2.0 * R_EARTH * h).sqrt() / r).acos() / core::f64::consts::PI;
    let samples = build(&sp).expect("時系列");
    let eclipse = 1.0 - mean_sunlit(&samples);
    assert!(
        (eclipse - expected).abs() < 0.002,
        "{eclipse} vs {expected}"
    );
    assert!((0.37..=0.38).contains(&eclipse), "{eclipse}");
    // 円錐影では半影の分だけわずかに変わるが 0.37〜0.38 の範囲
    let conical = 1.0 - mean_sunlit(&build(&spec(ShadowModel::Conical)).expect("時系列"));
    assert!((0.37..=0.38).contains(&conical), "{conical}");
    // 日陰は 1 区間、長さは約 35.8 分
    let w = shadow_windows(&samples, 0.5);
    assert_eq!(w.len(), 1, "{w:?}");
    let minutes = w[0].duration.value() / 60.0;
    let expected_min = expected * sp.orbit.period_s() / 60.0;
    // 端は標本の時刻なので刻み（5 s）の分だけ短くなりうる
    assert!((minutes - expected_min).abs() < 0.2, "{minutes} 分");
}

#[test]
fn samples_are_consistent() {
    let samples = build(&spec(ShadowModel::Conical)).expect("時系列");
    let last = samples.last().expect("標本");
    assert!((last.t_s - polar_orbit_with_beta_zero().period_s()).abs() < 1e-9);
    for s in samples.iter().step_by(50) {
        assert!((s.sun_dir_eci.norm() - 1.0).abs() < 1e-12);
        // 太陽指向: 機体 +Z が太陽を向く
        let z = s.sun_dir_body();
        assert!((z - Vec3::Z).norm() < 1e-9, "{z:?}");
        let nadir = s.nadir_body();
        assert!((s.to_eci(nadir) + s.r_eci.normalized().expect("非ゼロ")).norm() < 1e-9);
        assert!(s.sun_distance_m > 1.4e11 && s.sun_distance_m < 1.6e11);
    }
}

#[test]
fn sample_times_cover_span() {
    assert_eq!(
        sample_times(10.0, 2.5).expect("時刻"),
        vec![0.0, 2.5, 5.0, 7.5, 10.0]
    );
    assert_eq!(
        sample_times(10.0, 4.0).expect("時刻"),
        vec![0.0, 4.0, 8.0, 10.0]
    );
    assert_eq!(sample_times(0.0, 4.0).expect("時刻"), vec![0.0]);
    assert!(sample_times(10.0, 0.0).is_err());
    assert!(sample_times(-1.0, 1.0).is_err());
    assert!(sample_times(1e9, 1e-3).is_err());
}

#[test]
fn contiguous_windows() {
    let t = [0.0, 10.0, 20.0, 30.0, 40.0, 50.0];
    let f = [false, true, true, false, true, true];
    let w = windows(&t, &f);
    assert_eq!(w.len(), 2);
    assert_eq!(w[0].start.value(), 10.0);
    assert_eq!(w[0].end.value(), 20.0);
    assert_eq!(w[1].duration.value(), 10.0);
}

#[test]
fn empty_and_all_true() {
    assert!(windows(&[], &[]).is_empty());
    let w = windows(&[0.0, 5.0], &[true, true]);
    assert_eq!(w.len(), 1);
    assert_eq!(w[0].duration.value(), 5.0);
}

#[test]
fn beta_override_sets_raan_at_start() {
    let mut sp = spec(ShadowModel::Conical);
    sp.span = Qty::from_raw(600.0);
    for beta_deg in [-30.0_f64, 0.0, 25.0, 60.0] {
        sp.beta = Some(Qty::from_raw(beta_deg.to_radians()));
        let orbit = sp.resolved_orbit().expect("β に届く");
        let beta = beta_angle_rad(&orbit, orbit.epoch).expect("β").to_degrees();
        assert!((beta - beta_deg).abs() < 1e-9, "{beta} vs {beta_deg}");
        // build も同じ軌道を使う
        let first = build(&sp).expect("時系列")[0];
        let expected = orbit.state_at(0.0).expect("状態").r_eci;
        assert!((first.r_eci - expected).norm() < 1e-6);
    }
    sp.beta = None;
    assert_eq!(
        sp.resolved_orbit().expect("そのまま").elements.raan,
        sp.orbit.elements.raan
    );
}

#[test]
fn time_mean_and_cumulative_trapezoid() {
    let t = [0.0, 1.0, 3.0];
    let v = [0.0, 2.0, 2.0];
    // ∫ = 1 + 4 = 5、期間 3
    assert_eq!(cumulative_trapezoid(&t, &v), vec![0.0, 1.0, 5.0]);
    assert!((time_mean(&t, &v) - 5.0 / 3.0).abs() < 1e-15);
    assert!(time_mean(&[], &[]).is_nan());
    assert_eq!(time_mean(&[2.0], &[7.0]), 7.0);
    assert_eq!(time_mean(&[1.0, 1.0], &[4.0, 9.0]), 4.0);
    assert_eq!(cumulative_trapezoid(&t, &v[..2]).len(), 2);
}
