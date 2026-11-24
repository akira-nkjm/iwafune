//! `orbit.geometry` のテスト。期待値は導出（手計算・上下限）から決める。

use super::*;
use sat_astro::constants::{AU, C_LIGHT, SECONDS_PER_DAY};
use sat_astro::orbit::{OrbitElements, OrbitSpec, Propagator};

fn epoch() -> Epoch {
    Epoch::parse("2027-04-01T00:00:00Z").expect("epoch")
}

fn leo() -> (OrbitSpec, f64) {
    let o = OrbitSpec::circular(
        epoch(),
        Qty::from_raw(550.0e3),
        Qty::from_raw(0.9),
        Qty::from_raw(0.3),
    );
    let a = o.elements.semi_major_axis.value();
    (o, a)
}

/// 日心 1.5 AU の円軌道（黄道面。1 周は約 1.84 年）。
fn cruise() -> OrbitSpec {
    let eps = sat_astro::ephem::ecliptic_normal_eci().z.acos();
    let elements = OrbitElements {
        semi_major_axis: Qty::from_raw(1.5 * AU),
        eccentricity: 0.0,
        inclination: Qty::from_raw(eps),
        raan: Qty::from_raw(0.0),
        arg_perigee: Qty::from_raw(0.0),
        mean_anomaly: Qty::from_raw(0.0),
    };
    let mut o = OrbitSpec::new(epoch(), elements, Propagator::Kepler);
    o.central = CentralBody::Sun;
    o
}

fn leg_input(phase: Option<&str>, orbit: OrbitSpec, days: Option<f64>) -> GeometryLegInput {
    GeometryLegInput {
        phase: phase.map(str::to_owned),
        orbit: orbit.into(),
        duration: days.map(|d| Qty::from_raw(d * SECONDS_PER_DAY)),
    }
}

fn run(legs: Vec<GeometryLegInput>) -> GeometryOutput {
    Geometry::run(&GeometryInput { legs }).expect("run")
}

/// 地球を回る円軌道（1 周）: 地球距離はどこでも軌道半径（最小・最大とも）、光の伝搬時間 = 半径 / 光速、
/// 太陽距離は地球の日心距離 ± 軌道半径 ⊂ AU(1 ± 0.0168) ± 軌道半径。期間は軌道の 1 周で、書かれた期間が無いことを出力が示す。
#[test]
fn leo_one_cycle() {
    let (o, a) = leo();
    let period = o.period_s();
    let out = run(vec![leg_input(None, o, None)]);
    let l = &out.legs[0];
    assert_eq!(l.span_basis, GeometrySpanBasis::OneCycle);
    assert!((l.span.value() - period).abs() < 1e-9);
    assert_eq!(l.central, CentralBody::Earth);
    for d in [&l.earth_distance.min, &l.earth_distance.max] {
        assert!((d.value.value() / a - 1.0).abs() < 1e-9);
    }
    assert!((l.light_time_max.value.value() * C_LIGHT / a - 1.0).abs() < 1e-9);
    for d in [&l.sun_distance.min, &l.sun_distance.max] {
        let v = d.value.value();
        assert!(
            v >= AU * (1.0 - 0.0168) - a && v <= AU * (1.0 + 0.0168) + a,
            "{v}"
        );
    }
    assert!(l.sun_distance.min.value.value() <= l.sun_distance.max.value.value());
    assert_eq!(l.series.t.len(), l.series.channels[0].values.len());
    assert_eq!(l.series.channels.len(), 4);
}

/// 区間の並びは入力の順、期間はフェーズの期間（書かれていれば）。300 日の LEO の太陽距離は 1 年の中の近日点・遠日点の近くまで動く。
#[test]
fn legs_keep_order_and_use_the_phase_duration() {
    let (o, _) = leo();
    let out = run(vec![
        leg_input(Some("LAUNCH"), o.clone(), Some(1.0)),
        leg_input(Some("CRUISE"), cruise(), Some(300.0)),
        leg_input(None, o, None),
    ]);
    let phases: Vec<_> = out.legs.iter().map(|l| l.phase.as_deref()).collect();
    assert_eq!(phases, [Some("LAUNCH"), Some("CRUISE"), None]);
    let cruise = &out.legs[1];
    assert_eq!(cruise.span_basis, GeometrySpanBasis::PhaseDuration);
    assert!((cruise.span.value() - 300.0 * SECONDS_PER_DAY).abs() < 1e-6);
    assert!(
        (cruise.sample_step.value() * SAMPLES_PER_LEG as f64 - cruise.span.value()).abs() < 1e-6
    );
}

/// 日心 1.5 AU の円軌道: 太陽距離は最小・最大とも 1.5 AU、地球距離は [0.5 − e, 2.5 + e] AU（e = 0.0168。地球の日心距離 AU(1 ± e) から）、
/// 最小の SEP は 0° 以上 180° 以下、最大の光の伝搬時間は地球距離の最大 / 光速（日付も同じ）。
#[test]
fn heliocentric_leg_bounds_and_consistency() {
    let out = run(vec![leg_input(
        Some("CRUISE"),
        cruise(),
        Some(2.0 * 365.25),
    )]);
    let l = &out.legs[0];
    for d in [&l.sun_distance.min, &l.sun_distance.max] {
        assert!((d.value.value() / (1.5 * AU) - 1.0).abs() < 1e-9);
    }
    let (min, max) = (
        l.earth_distance.min.value.value(),
        l.earth_distance.max.value.value(),
    );
    assert!(
        min >= (0.5 - 0.0168) * AU && max <= (2.5 + 0.0168) * AU,
        "{} {}",
        min / AU,
        max / AU
    );
    assert!(max > 2.4 * AU && min < 0.6 * AU);
    assert!((l.light_time_max.value.value() - max / C_LIGHT).abs() < 1e-6 * max / C_LIGHT);
    assert!((0.0..=std::f64::consts::PI).contains(&l.sep_min.value.value()));
    assert_eq!(l.ephemeris, EphemerisSource::Analytic);
}

/// 期間の決め方: 1 回限りの期間が最優先、次にフェーズの期間、なければ 1 周。
#[test]
fn span_rule() {
    let periodic = Cycle::Periodic { period_s: 100.0 };
    let d = Some(Qty::from_raw(40.0));
    assert_eq!(
        leg_span(Cycle::OneOff { span_s: 7.0 }, d),
        (7.0, GeometrySpanBasis::OneOff)
    );
    assert_eq!(
        leg_span(periodic, d),
        (40.0, GeometrySpanBasis::PhaseDuration)
    );
    assert_eq!(
        leg_span(periodic, None),
        (100.0, GeometrySpanBasis::OneCycle)
    );
}

#[test]
fn rejects_no_legs_and_bad_spans() {
    assert!(Geometry::run(&GeometryInput { legs: vec![] }).is_err());
    let (o, _) = leo();
    assert!(
        Geometry::run(&GeometryInput {
            legs: vec![leg_input(None, o, Some(0.0))]
        })
        .is_err()
    );
}
