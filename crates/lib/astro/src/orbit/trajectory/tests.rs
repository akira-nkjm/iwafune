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

/// 期間を持たせると繰り返しは 1 回限りになるが、位置・速度・中心天体・元期は変わらない。要素の入口（`elements`）は閉じる。
#[test]
fn a_span_makes_the_cycle_one_off_without_changing_the_motion() {
    for o in cases() {
        let plain = Trajectory::Elements(o.clone());
        let spanned = plain.clone().with_span(86_400.0);
        assert_eq!(spanned.cycle(), Cycle::OneOff { span_s: 86_400.0 });
        assert_eq!(spanned.span_s(), Some(86_400.0));
        assert_eq!(plain.span_s(), None);
        assert!(spanned.elements().is_none());
        assert_eq!(spanned.epoch(), o.epoch);
        assert_eq!(spanned.central(), o.central);
        let times = [0.0, 100.0, 5_000.0];
        assert_eq!(
            spanned.states_at(&times).unwrap(),
            plain.states_at(&times).unwrap()
        );
        // 期間は置き換わる（入れ子にならない）
        assert_eq!(spanned.with_span(10.0).span_s(), Some(10.0));
    }
}

/// 期間を持つ軌道は `span_s` を足した形で書け、読み戻すと同じ。期間の無い軌道の書き出しには `span_s` が出ない。
#[test]
fn a_span_round_trips_and_a_plain_orbit_never_writes_it() {
    let o = spec(CentralBody::Sun, 1.0e10, Propagator::Kepler);
    let plain = serde_json::to_value(Trajectory::Elements(o.clone())).unwrap();
    assert!(plain.get("span_s").is_none());
    let spanned = Trajectory::Elements(o).with_span(2.5e7);
    let v = serde_json::to_value(&spanned).unwrap();
    assert_eq!(v["span_s"], 2.5e7);
    let back: Trajectory = serde_json::from_value(v.clone()).unwrap();
    assert_eq!(back.cycle(), Cycle::OneOff { span_s: 2.5e7 });
    assert_eq!(serde_json::to_value(&back).unwrap(), v);
    let bad = serde_json::from_value::<Trajectory>(serde_json::json!({
        "span_s": "long", "epoch": "2027-04-01T00:00:00Z"
    }))
    .unwrap_err()
    .to_string();
    assert!(bad.contains("span_s"), "{bad}");
}

fn state_orbit(span_s: f64) -> Trajectory {
    let r = 1.5e11;
    let mu = CentralBody::Sun.gm();
    Trajectory::State(StateSpec::with_defaults(
        epoch(),
        CentralBody::Sun,
        OrbitState {
            r_eci: crate::math::Vec3::new(r, 0.0, 0.0),
            v_eci: crate::math::Vec3::new(0.0, (mu / r).sqrt(), 0.0),
        },
        span_s,
        None,
    ))
}

/// 状態ベクトルの軌道の入口: 元期・中心天体・期間は書いたとおり、繰り返しは 1 回限り（期間）、要素も高度も持たない。
/// `with_span` は期間の欄を置き換える（`Spanned` で包まない）。状態は `StateSpec::states_at` と同じ。
#[test]
fn a_state_orbit_is_one_off_with_its_written_span_and_has_no_elements() {
    let t = state_orbit(3.0e7);
    assert_eq!(t.epoch(), epoch());
    assert_eq!(t.central(), CentralBody::Sun);
    assert_eq!(t.cycle(), Cycle::OneOff { span_s: 3.0e7 });
    assert_eq!(t.span_s(), Some(3.0e7));
    assert!(t.elements().is_none() && t.altitude_m().is_none());
    let longer = t.clone().with_span(5.0e7);
    assert!(matches!(longer, Trajectory::State(_)));
    assert_eq!(longer.cycle(), Cycle::OneOff { span_s: 5.0e7 });
    let Trajectory::State(spec) = &t else {
        unreachable!()
    };
    let times = [0.0, 1.0e6, 2.5e7];
    assert_eq!(
        t.states_at(&times).unwrap(),
        spec.states_at(&times).unwrap()
    );
    assert_eq!(
        t.state_at(1.0e6).unwrap(),
        t.states_at(&[1.0e6]).unwrap()[0]
    );
    assert_eq!(t.table_step_s(), spec.table_step_s());
}

/// 状態ベクトルの軌道は `source = "state"` を足した形で書け、読み戻すと同じ。壊れた値は読むときに理由を返す。
#[test]
fn a_state_orbit_round_trips_with_the_source_tag() {
    let t = state_orbit(3.0e7);
    let v = serde_json::to_value(&t).unwrap();
    assert_eq!(v["source"], "state");
    assert_eq!(v["span_s"], 3.0e7);
    let back: Trajectory = serde_json::from_value(v.clone()).unwrap();
    assert_eq!(serde_json::to_value(&back).unwrap(), v);
    assert_eq!(back.cycle(), t.cycle());
    let mut bad = v;
    bad.as_object_mut().unwrap().remove("state");
    assert!(serde_json::from_value::<Trajectory>(bad).is_err());
}
