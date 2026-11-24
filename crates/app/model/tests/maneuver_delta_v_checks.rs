//! 直接書いた ΔV（マヌーバの `delta_v`）の検査: 速度の次元・負でない・軌道を変える書き方と併用しない・計算の段階。
use fune_model::design::{
    Maneuver, ManeuverCategory, ManeuverConstraints, ManeuverMethod, ManeuverTarget,
};
use fune_model::value::Quantity;
use fune_model::{Design, check};

fn design(dv: Option<Quantity>, method: ManeuverMethod, target: ManeuverTarget) -> Design {
    let mut d = Design::default();
    d.maneuvers.insert(
        "TCM".into(),
        Maneuver {
            id: "TCM".into(),
            label: None,
            category: ManeuverCategory::StationKeeping,
            method,
            thruster: None,
            after: None,
            via_altitude: None,
            delta_v: dv,
            target,
            constraints: ManeuverConstraints::default(),
            transfer: None,
        },
    );
    d
}

fn codes(d: &Design) -> Vec<String> {
    check(d)
        .into_iter()
        .filter(|x| x.code.starts_with("maneuver.delta_v") || x.code.starts_with("unit."))
        .map(|x| x.code)
        .collect()
}

fn mps(v: f64) -> Option<Quantity> {
    Some(Quantity::new(v, "m/s"))
}

#[test]
fn a_written_delta_v_is_accepted() {
    let d = design(
        mps(15.0),
        ManeuverMethod::Impulsive,
        ManeuverTarget::default(),
    );
    assert!(codes(&d).is_empty());
    let d = design(
        mps(0.0),
        ManeuverMethod::LowThrust,
        ManeuverTarget::default(),
    );
    assert!(codes(&d).is_empty());
}

#[test]
fn delta_v_must_be_a_velocity() {
    let d = design(
        Some(Quantity::new(15.0, "km")),
        ManeuverMethod::Impulsive,
        ManeuverTarget::default(),
    );
    assert_eq!(codes(&d), ["unit.dimension"]);
}

#[test]
fn delta_v_must_not_be_negative() {
    let d = design(
        mps(-1.0),
        ManeuverMethod::Impulsive,
        ManeuverTarget::default(),
    );
    assert_eq!(codes(&d), ["maneuver.delta_v_negative"]);
}

#[test]
fn delta_v_is_not_combined_with_an_orbit_change() {
    let target = ManeuverTarget {
        altitude: Some(Quantity::new(600.0, "km")),
        ..ManeuverTarget::default()
    };
    let d = design(mps(15.0), ManeuverMethod::Impulsive, target);
    assert_eq!(codes(&d), ["maneuver.delta_v_conflict"]);
    // 目標も書かない通常のマヌーバは何も言わない
    let d = design(None, ManeuverMethod::Impulsive, ManeuverTarget::default());
    assert!(codes(&d).is_empty());
}

#[test]
fn delta_v_needs_impulsive_or_low_thrust() {
    for m in [ManeuverMethod::FiniteBurn, ManeuverMethod::Propagated] {
        let d = design(mps(15.0), m, ManeuverTarget::default());
        assert_eq!(codes(&d), ["maneuver.delta_v_method"], "{m}");
    }
}
