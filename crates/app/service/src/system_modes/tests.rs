//! 系ごとのモードの見方の値と判定。
use super::*;
use fune_model::design::{Allocation, AttitudeLawKind, Part};
use fune_model::value::{Authored, Quantity, Scalar, Value};
use sat_analysis::spec::{System, Verdict};

fn snapshot() -> Snapshot {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    let mut design = Workspace::open(root).unwrap().snapshot().unwrap().design;
    let mut part = Part::new("TEST");
    part.count = 2;
    for (key, value, unit) in [
        ("power", 10.0, "W"),
        ("heat_fraction", 0.5, ""),
        ("hk_rate", 100.0, "bit/s"),
        ("mission_rate", 400.0, "bit/s"),
    ] {
        let scalar = Scalar::new(value, unit);
        let value = if key == "heat_fraction" {
            Value::Scalar(scalar)
        } else {
            Value::ByState(
                [
                    ("on".into(), Authored::bare(scalar)),
                    ("off".into(), Authored::bare(Scalar::new(0.0, unit))),
                ]
                .into(),
            )
        };
        part.attr.insert(key.into(), Authored::bare(value));
    }
    design.parts = [(part.id.clone(), part)].into();
    let mode = serde_json::from_value::<fune_model::design::Mode>(serde_json::json!({
        "id": "TEST", "states": {"TEST": {"state":"on"}},
        "eclipse": {"TEST": {"state":"off"}}
    }))
    .unwrap();
    design.modes = [(mode.id.clone(), mode)].into();
    design.budget.policy = Default::default();
    design.budget.allocation.power = Some(Allocation {
        limit: Some(Quantity::new(100.0, "W")),
        system_margin: 0.2,
        alloc: Default::default(),
    });
    design.analysis.thermal =
        Some(serde_json::from_value(serde_json::json!({"nodes":{"node":["TEST"]}})).unwrap());
    design.analysis.aocs.as_mut().unwrap().law = [(
        "TEST".into(),
        serde_json::from_value(serde_json::json!({"law":"sun_pointing"})).unwrap(),
    )]
    .into();
    Snapshot::of(design, Default::default())
}

#[test]
fn dto_values_and_budget_are_hand_checkable() {
    let s = snapshot();
    let power = system_modes_of(&s, System::Power).unwrap();
    let sun = &power.modes[0].sunlit;
    let eclipse = &power.modes[0].eclipse;
    assert_eq!(sun.parts["TEST"].as_ref().unwrap().si, 20.0);
    assert_eq!(sun.total.as_ref().unwrap().si, 20.0);
    assert_eq!(sun.available.as_ref().unwrap().si, 80.0);
    assert_eq!(sun.margin.as_ref().unwrap().si, 60.0);
    assert_eq!(eclipse.total.as_ref().unwrap().si, 0.0);
    assert_eq!(eclipse.margin.as_ref().unwrap().si, 80.0);
    assert_eq!(sun.verdict, Some(Verdict::Ok));
    assert_eq!(
        sun.total.as_ref().unwrap().display,
        crate::units::quantity_auto(20.0, "Power").display
    );
    let data = system_modes_of(&s, System::Cdh).unwrap();
    assert_eq!(data.modes[0].sunlit.total.as_ref().unwrap().si, 1000.0); // 2 × (100 + 400)
    let heat = system_modes_of(&s, System::Thermal).unwrap();
    assert_eq!(
        heat.modes[0].sunlit.parts["TEST"].as_ref().unwrap().si,
        10.0
    ); // 2 × 10 × 0.5
    assert_eq!(heat.modes[0].eclipse.total.as_ref().unwrap().si, 0.0);
    let aocs = system_modes_of(&s, System::Aocs).unwrap();
    assert_eq!(
        aocs.modes[0].aocs.as_ref().unwrap().attitude.law,
        Some(AttitudeLawKind::SunPointing)
    );
    assert!(aocs.modes[0].sunlit.total.is_none());
    assert!(system_modes_of(&s, System::Structure).is_err());
}

#[test]
fn mode_verdicts_and_missing_states_come_from_rust() {
    let mut s = snapshot();
    s.design.budget.allocation.power.as_mut().unwrap().limit = Some(Quantity::new(10.0, "W"));
    let power = system_modes_of(&s, System::Power).unwrap();
    assert_eq!(power.modes[0].sunlit.margin.as_ref().unwrap().si, -12.0);
    assert_eq!(power.modes[0].sunlit.verdict, Some(Verdict::Violation));
    s.design
        .modes
        .values_mut()
        .next()
        .unwrap()
        .states
        .values_mut()
        .next()
        .unwrap()
        .state = "missing".into();
    for system in [System::Power, System::Cdh, System::Thermal] {
        let view = system_modes_of(&s, system).unwrap();
        assert!(view.modes[0].sunlit.parts["TEST"].is_none());
        assert!(view.modes[0].sunlit.total.is_none());
        assert_eq!(view.modes[0].sunlit.verdict, Some(Verdict::Warning));
    }
}

#[test]
fn missing_inputs_do_not_become_zero_or_ok() {
    let mut s = snapshot();
    s.design.analysis.aocs.as_mut().unwrap().law.clear();
    let aocs = system_modes_of(&s, System::Aocs).unwrap();
    assert_eq!(
        aocs.modes[0].aocs.as_ref().unwrap().rw.status,
        crate::dto::AnalysisRunStatus::MissingInputs
    );
    let values = mode_values([("part".into(), None)], "Power", &[]);
    assert!(values.parts["part"].is_none());
    assert!(values.total.is_none());
    assert_eq!(values.verdict, Some(Verdict::Warning));
    let empty = mode_values([], "Power", &[]);
    assert!(empty.total.is_none());
    assert_eq!(empty.verdict, Some(Verdict::Warning));
}

/// 電力のモード表示は、表を作る入力の不足（運用モードが無い）を捨てずに返す。
#[test]
fn power_modes_return_missing_inputs() {
    let mut s = snapshot();
    assert!(
        system_modes_of(&s, System::Power)
            .unwrap()
            .missing
            .is_empty()
    );
    s.design.modes.clear();
    let view = system_modes_of(&s, System::Power).unwrap();
    assert!(view.missing.iter().any(|m| m.path.contains("modes")));
}
