use super::*;
use crate::constants::R_EARTH;
use crate::ephem::sun_direction_eci;
use crate::orbit::{OrbitSpec, Propagator, beta_angle_rad};

fn polar_orbit_with_beta_zero() -> OrbitSpec {
    let epoch = Epoch::parse("2027-06-01T00:00:00Z").expect("epoch");
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
        sampling: OrbitSampling::new(orbit, Qty::from_raw(span), Qty::from_raw(5.0)),
        attitude: AttitudeLaw::SunPointing {
            axis: Vec3::Z,
            secondary: Vec3::Y,
        },
        shadow,
    }
}

/// 550 km 円軌道・β = 0 の日陰率: 円柱影で (1/π)·acos(√(h²+2Rh)/r) ≈ 0.372。
#[test]
fn eclipse_fraction_at_550_km_beta_zero() {
    let sp = spec(ShadowModel::Cylindrical);
    let orbit = sp.sampling.orbit.elements().expect("elements");
    let beta = beta_angle_rad(orbit, orbit.epoch).expect("β");
    assert!(beta.abs() < 1e-6, "β = {beta}");
    let h = 550e3;
    let r = R_EARTH + h;
    let expected = ((h * h + 2.0 * R_EARTH * h).sqrt() / r).acos() / core::f64::consts::PI;
    let samples = build(&sp).expect("timeline");
    let eclipse = 1.0 - mean_sunlit(&samples);
    assert!(
        (eclipse - expected).abs() < 0.002,
        "{eclipse} vs {expected}"
    );
    assert!((0.37..=0.38).contains(&eclipse), "{eclipse}");
    // 円錐影では半影の分だけわずかに変わるが 0.37〜0.38 の範囲
    let conical = 1.0 - mean_sunlit(&build(&spec(ShadowModel::Conical)).expect("timeline"));
    assert!((0.37..=0.38).contains(&conical), "{conical}");
    // 日陰は 1 区間、長さは約 35.8 分
    let w = shadow_windows(&samples, 0.5);
    assert_eq!(w.len(), 1, "{w:?}");
    let minutes = w[0].duration.value() / 60.0;
    let expected_min = expected * sp.sampling.orbit.cycle().length_s() / 60.0;
    // 端はサンプルの時刻なので刻み（5 s）の分だけ短くなりうる
    assert!((minutes - expected_min).abs() < 0.2, "{minutes} min");
}

#[test]
fn samples_are_consistent() {
    let samples = build(&spec(ShadowModel::Conical)).expect("timeline");
    let last = samples.last().expect("sample");
    assert!((last.t_s - polar_orbit_with_beta_zero().period_s()).abs() < 1e-9);
    for s in samples.iter().step_by(50) {
        assert!((s.sun_dir_eci.norm() - 1.0).abs() < 1e-12);
        // 太陽指向: 機体 +Z が太陽を向く
        let z = s.sun_dir_body();
        assert!((z - Vec3::Z).norm() < 1e-9, "{z:?}");
        let nadir = s.nadir_body();
        assert!((s.to_eci(nadir) + s.r_eci.normalized().expect("nonzero")).norm() < 1e-9);
        assert!(s.sun_distance_m > 1.4e11 && s.sun_distance_m < 1.6e11);
    }
}

#[test]
fn sample_times_cover_span() {
    assert_eq!(
        sample_times(10.0, 2.5).expect("times"),
        vec![0.0, 2.5, 5.0, 7.5, 10.0]
    );
    assert_eq!(
        sample_times(10.0, 4.0).expect("times"),
        vec![0.0, 4.0, 8.0, 10.0]
    );
    assert_eq!(sample_times(0.0, 4.0).expect("times"), vec![0.0]);
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
    sp.sampling.span = Qty::from_raw(600.0);
    for beta_deg in [-30.0_f64, 0.0, 25.0, 60.0] {
        sp.sampling.beta = Some(Qty::from_raw(beta_deg.to_radians()));
        let resolved = sp.resolved_orbit().expect("reaches beta");
        let orbit = resolved.elements().expect("elements");
        let beta = beta_angle_rad(orbit, orbit.epoch).expect("β").to_degrees();
        assert!((beta - beta_deg).abs() < 1e-9, "{beta} vs {beta_deg}");
        // build も同じ軌道を使う
        let first = build(&sp).expect("timeline")[0];
        let expected = orbit.state_at(0.0).expect("state").r_eci;
        assert!((first.r_eci - expected).norm() < 1e-6);
    }
    sp.sampling.beta = None;
    assert_eq!(
        sp.resolved_orbit()
            .expect("as is")
            .elements()
            .expect("elements")
            .elements
            .raan,
        sp.sampling
            .orbit
            .elements()
            .expect("elements")
            .elements
            .raan
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

/// サンプルの取り方は JSON では時系列の同じ階層に並ぶ（`orbit`・`span`・`step`・`beta` を `attitude` と並べて書く）。
#[test]
fn sampling_is_flattened_in_json() {
    let mut sp = spec(ShadowModel::Conical);
    sp.sampling.beta = Some(Qty::from_raw(0.5));
    let json = serde_json::to_value(&sp).expect("JSON");
    for key in ["orbit", "span", "step", "beta", "attitude", "shadow"] {
        assert!(json.get(key).is_some(), "{key}: {json}");
    }
    assert!(json.get("sampling").is_none());
    let back: TimelineSpec = serde_json::from_value(json).expect("round trip");
    assert_eq!(back.sampling.beta, sp.sampling.beta);
    assert_eq!(back.sampling.step, sp.sampling.step);
}

/// サンプルの時刻では、任意の時刻の評価（`TimelineEvaluator::at`）が `build` の点と同じになる。
#[test]
fn evaluator_matches_build_at_sample_times() {
    let sp = spec(ShadowModel::Conical);
    let samples = build(&sp).expect("timeline");
    let ev = TimelineEvaluator::new(&sp).expect("evaluator");
    for s in samples.iter().step_by(37) {
        assert_eq!(ev.at(s.t_s), *s);
    }
}

/// 円柱影の蝕の長さ（日照が 1 → 0 と 0 → 1 に変わる時刻の差）は、半角 asin(R/r) から周期 × asin(R/r)/π。
/// 時刻は日照率の境を二分法で求めるので、サンプルの刻み（60 s）よりずっと小さい誤差で一致する。
#[test]
fn shadow_edges_give_the_analytic_eclipse_duration() {
    let mut sp = spec(ShadowModel::Cylindrical);
    sp.sampling.step = Qty::from_raw(60.0);
    let ev = TimelineEvaluator::new(&sp).expect("evaluator");
    let samples = build(&sp).expect("timeline");
    let mut edges = Vec::new();
    for w in samples.windows(2) {
        let found = ev.shadow_edges(w[0].t_s, w[1].t_s);
        if !found.is_empty() {
            edges.push((w[0].sunlit, found));
        }
    }
    // 円柱影は 1 → 0 の跳びなので、2 つの境が同じ時刻に重なる
    assert_eq!(edges.len(), 2, "{edges:?}");
    let (entry, exit) = (edges[0].1[0], edges[1].1[0]);
    assert!((edges[0].1[0] - edges[0].1[1]).abs() < 1e-3);
    let r = R_EARTH + 550e3;
    let expected =
        sp.sampling.orbit.cycle().length_s() * (R_EARTH / r).asin() / core::f64::consts::PI;
    assert!(
        (exit - entry - expected).abs() < 1.0,
        "{} vs {expected}",
        exit - entry
    );
}

/// 回転座標系の軌跡（ラグランジュ点）の幾何: 太陽–地球 L2 は地球の向こう側にあり、太陽・地球は探査機から同じ側
/// （太陽–探査機–地球の角 SPE ≈ 0°、地球での太陽–地球–探査機の角 SEP ≈ 180°）。L1 は地球と太陽の間で SPE ≈ 180°、SEP ≈ 0°。
#[test]
fn sun_earth_libration_points_have_the_hand_derived_angles() {
    use crate::ephem::CentralBody;
    use crate::orbit::SynodicSpec;
    use crate::orbit::cr3bp::LibrationPoint;
    let epoch = Epoch::parse("2030-01-01T00:00:00Z").expect("epoch");
    let day = 86_400.0;
    for (point, sep_deg, spe_deg) in [
        (LibrationPoint::L2, 180.0, 0.0),
        (LibrationPoint::L1, 0.0, 180.0),
    ] {
        let orbit = Trajectory::Synodic(SynodicSpec::new(
            epoch,
            CentralBody::Sun,
            CentralBody::Earth,
            point,
            None,
        ));
        let times: Vec<f64> = (0..6).map(|i| f64::from(i) * 60.0 * day).collect();
        for g in geometry::geometry_at(&orbit, epoch, &times).expect("geometry") {
            // L1・L2 は地球の近く（γ·d ≈ 0.01 d）なので、太陽の方向の角のずれは 1e-2 rad 以内の幾何誤差でなく定義どおり 0 か 180°
            assert!(
                (g.sep_rad.to_degrees() - sep_deg).abs() < 1e-6,
                "{point:?} sep {}",
                g.sep_rad.to_degrees()
            );
            assert!(
                (g.spe_rad.to_degrees() - spe_deg).abs() < 1e-3,
                "{point:?} spe {}",
                g.spe_rad.to_degrees()
            );
        }
    }
}

/// 日陰を作る天体: 地球–月系の L1（地球と月の間）は、月食のとき地球の本影に入る。中心天体（月）だけを見ると日照 1 のまま
/// （地球は影を作る天体として足されている）。2022-11-08 の皆既月食の最大の時刻（約 10:59 UTC。月の暦の精度で本影の中心から 0.5 本影半径以内）。
#[test]
fn earth_moon_l1_is_in_the_earth_umbra_during_a_total_lunar_eclipse() {
    use crate::ephem::CentralBody;
    use crate::orbit::SynodicSpec;
    use crate::orbit::cr3bp::LibrationPoint;
    let epoch = Epoch::parse("2022-11-08T10:59:00Z").expect("epoch");
    let orbit = Trajectory::Synodic(SynodicSpec::new(
        epoch,
        CentralBody::Earth,
        CentralBody::Moon,
        LibrationPoint::L1,
        None,
    ));
    assert_eq!(orbit.central(), CentralBody::Moon);
    let sp = TimelineSpec::new(
        orbit,
        AttitudeLaw::SunPointing {
            axis: Vec3::Z,
            secondary: Vec3::Y,
        },
        Qty::from_raw(0.0),
        Qty::from_raw(60.0),
    );
    let s = &build(&sp).expect("timeline")[0];
    assert_eq!(s.sunlit, 0.0);
    // 中心天体（月）だけなら、月の影は月の向こう側に伸びるので、手前の L1 は月の影の外で日照
    let r_sun = CentralBody::Moon.relative_position_m(Body::Sun, epoch);
    assert_eq!(
        central_shadow_factor(CentralBody::Moon, s.r_eci, r_sun, sp.shadow),
        1.0
    );
}
