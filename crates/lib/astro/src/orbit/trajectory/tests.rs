use super::*;
use crate::force::{ForceConfig, GravityModel};
use crate::orbit::Propagator;
use sat_units::typed::Qty;

fn epoch() -> Epoch {
    Epoch::parse("2027-04-01T00:00:00Z").expect("epoch")
}

fn spec(central: CentralBody, altitude_m: f64, propagator: Propagator) -> OrbitSpec {
    let mut o = OrbitSpec::circular_about(
        central,
        epoch(),
        Qty::from_raw(altitude_m),
        Qty::from_raw(1.1),
        Qty::from_raw(0.4),
    );
    o.elements.eccentricity = 0.01;
    o.elements.mean_anomaly = Qty::from_raw(0.7);
    o.propagator = propagator;
    o
}

fn cases() -> Vec<OrbitSpec> {
    let numerical = || Propagator::Numerical {
        forces: ForceConfig::gravity_only(GravityModel::J2),
    };
    vec![
        spec(CentralBody::Earth, 550e3, Propagator::Kepler),
        spec(CentralBody::Earth, 550e3, Propagator::J2),
        spec(CentralBody::Earth, 550e3, numerical()),
        spec(CentralBody::Moon, 100e3, Propagator::J2),
        spec(CentralBody::Moon, 100e3, numerical()),
        spec(CentralBody::Mars, 300e3, Propagator::Kepler),
        spec(CentralBody::Sun, 1.0e11, Propagator::Kepler),
    ]
}

/// `Elements` は元の [`OrbitSpec`] と同じ状態を、ビットまで同じに返す（解析的・数値の伝搬、複数の中心天体）。
#[test]
fn elements_states_are_bit_equal_to_the_spec() {
    let times = [0.0, 12.5, 600.0, 3600.25, 5400.0, 86_400.0];
    for o in cases() {
        let t = Trajectory::Elements(o.clone());
        assert_eq!(t.epoch(), o.epoch);
        assert_eq!(t.central(), o.central);
        assert_eq!(t.states_at(&times).unwrap(), o.states_at(&times).unwrap());
        for &s in &times {
            assert_eq!(t.state_at(s).unwrap(), o.state_at(s).unwrap(), "{o:?} {s}");
        }
        let (a, b) = (
            t.track(86_400.0, 30.0).unwrap(),
            o.track(86_400.0, 30.0).unwrap(),
        );
        for &s in &[0.0, 17.3, 1234.5, 40_000.0, 86_400.0] {
            assert_eq!(a.state_at(s), b.state_at(s), "{o:?} {s}");
        }
    }
}

/// 要素で書かれた軌道の繰り返しは周期的で、長さは Kepler 周期。
#[test]
fn elements_cycle_is_periodic_with_the_kepler_period() {
    for o in cases() {
        let t = Trajectory::Elements(o.clone());
        assert_eq!(
            t.cycle(),
            Cycle::Periodic {
                period_s: o.period_s()
            }
        );
        assert_eq!(t.cycle().length_s(), o.period_s());
        assert_eq!(t.cycle().period_s(), Some(o.period_s()));
        assert_eq!(t.elements().expect("elements").elements.eccentricity, 0.01);
    }
}

#[test]
fn one_off_cycle_has_a_span_and_no_period() {
    let c = Cycle::OneOff { span_s: 123.0 };
    assert_eq!(c.length_s(), 123.0);
    assert_eq!(c.period_s(), None);
}

/// 書き出しは [`OrbitSpec`] と同じ（タグが付かない）で、書き出したものを読み戻すと同じになる。
#[test]
fn serializes_exactly_as_the_orbit_spec_and_round_trips() {
    for o in cases() {
        let direct = serde_json::to_string(&o).unwrap();
        let t = Trajectory::Elements(o);
        let text = serde_json::to_string(&t).unwrap();
        assert_eq!(text, direct);
        let back: Trajectory = serde_json::from_str(&text).unwrap();
        assert_eq!(serde_json::to_string(&back).unwrap(), direct);
    }
}

/// この変更の前に書かれた解析入力（デモの `power.energy_balance` の `input.timeline.orbit`）が、そのまま読めて同じ形で書き出せる。
#[test]
fn input_written_before_the_change_reads_and_writes_back_identically() {
    let old = r#"{"central":"earth","elements":{"arg_perigee":0.0,"eccentricity":0.0,"inclination":1.7033183882367373,"mean_anomaly":0.0,"raan":2.9199616612848334,"semi_major_axis":6928137.0},"epoch":"2027-04-01T00:00:00Z","propagator":{"method":"j2"}}"#;
    let t: Trajectory = serde_json::from_str(old).unwrap();
    let spec: OrbitSpec = serde_json::from_str(old).unwrap();
    assert_eq!(
        t.states_at(&[0.0, 60.0]).unwrap(),
        spec.states_at(&[0.0, 60.0]).unwrap()
    );
    let reparsed: serde_json::Value = serde_json::to_value(&t).unwrap();
    let original: serde_json::Value = serde_json::from_str(old).unwrap();
    assert_eq!(reparsed, original);
    // 中心天体を書かない入力（既定は地球）も読める。
    let mut no_central = original;
    no_central.as_object_mut().unwrap().remove("central");
    let t: Trajectory = serde_json::from_value(no_central).unwrap();
    assert_eq!(t.central(), CentralBody::Earth);
}

/// `source: "elements"` を明示しても読め、知らない source は理由つきのエラーにする。
#[test]
fn explicit_source_tag_is_accepted_and_unknown_source_is_rejected() {
    let o = spec(CentralBody::Earth, 550e3, Propagator::J2);
    let mut v = serde_json::to_value(&o).unwrap();
    v["source"] = "elements".into();
    let t: Trajectory = serde_json::from_value(v.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(&t).unwrap(),
        serde_json::to_value(&o).unwrap()
    );
    v["source"] = "teleport".into();
    let e = serde_json::from_value::<Trajectory>(v)
        .unwrap_err()
        .to_string();
    assert!(e.contains("Unknown orbit source"), "{e}");
}

/// 要素の欠けは [`OrbitSpec`] のエラーのまま伝わる。
#[test]
fn malformed_elements_report_the_underlying_error() {
    let e = serde_json::from_str::<Trajectory>(r#"{"epoch":"2027-04-01T00:00:00Z"}"#)
        .unwrap_err()
        .to_string();
    assert!(e.contains("missing field"), "{e}");
}
