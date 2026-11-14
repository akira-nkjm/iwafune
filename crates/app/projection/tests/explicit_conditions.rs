//! 解析条件が曖昧なときに先頭の候補へ暗黙にフォールバックしないこと。

use fune_model::{Design, Target};
use fune_projection::{Context, MissingInputs, aocs, thermal};
use sat_analysis::thermal::ThermalExposure;
use std::path::Path;

fn demo() -> Design {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    fune_store::load(&root).expect("デモを読めること").design
}

#[test]
fn multiple_root_boxes_are_ambiguous() {
    let design = demo();
    let mut resolved = fune_model::resolve(&design);
    let mut other = resolved.parts["BUS_FRAME"].clone();
    other.id = "OTHER_ROOT".to_owned();
    resolved.parts.insert(other.id.clone().into(), other);
    let ctx = Context::new(&design, &resolved, Some("hot_imaging")).unwrap();
    let missing = aocs::rw_unloading(&ctx).unwrap_err().into_vec();
    assert!(missing.iter().any(|m| {
        m.target == Some(Target::any_part("size"))
            && m.reason.contains("BUS_FRAME")
            && m.reason.contains("OTHER_ROOT")
            && m.reason.contains("曖昧")
    }));
}

#[test]
fn attitude_law_requires_the_requested_mode_or_a_unique_law() {
    let mut design = demo();
    let resolved = fune_model::resolve(&design);
    let ctx = Context::new(&design, &resolved, None).unwrap();
    for (mode, path) in [
        (Some("UNKNOWN"), "analysis.aocs.law.UNKNOWN"),
        (None, "analysis.aocs.law"),
    ] {
        let mut missing = MissingInputs::new();
        assert!(ctx.attitude_law(mode, &mut missing).is_none());
        assert!(missing.into_vec().iter().any(|m| m.path == path));
    }
    design
        .analysis
        .aocs
        .as_mut()
        .unwrap()
        .law
        .retain(|id, _| id == "SAFE");
    let ctx = Context::new(&design, &resolved, None).unwrap();
    let mut missing = MissingInputs::new();
    assert!(ctx.attitude_law(None, &mut missing).is_some());
    assert!(missing.is_empty());
}

#[test]
fn thermal_condition_must_be_selected_when_ambiguous() {
    let mut design = demo();
    // デモの eclipse_worst は熱の条件を選んでいるので、外して曖昧にする。
    if let Some(case) = design.cases.get_mut("eclipse_worst") {
        case.thermal_case = None;
    }
    let resolved = fune_model::resolve(&design);
    let ctx = Context::new(&design, &resolved, Some("eclipse_worst")).unwrap();
    let missing = thermal::transient(&ctx).unwrap_err().into_vec();
    assert!(
        missing
            .iter()
            .any(|m| { m.target == Some(Target::case("eclipse_worst", Some("thermal_case"))) })
    );
    let ctx = Context::new(&design, &resolved, None).unwrap();
    assert!(thermal::transient(&ctx).is_err());
    design
        .analysis
        .thermal
        .as_mut()
        .unwrap()
        .cases
        .retain(|id, _| id == "hot");
    let ctx = Context::new(&design, &resolved, None).unwrap();
    let input = thermal::transient(&ctx).unwrap();
    assert_eq!(input.environment.solar_flux.unwrap().value(), 1414.0);
}

#[test]
fn unknown_selected_thermal_condition_does_not_fall_back() {
    let mut design = demo();
    design.cases.get_mut("hot_imaging").unwrap().thermal_case = Some("UNKNOWN".to_owned());
    let resolved = fune_model::resolve(&design);
    let ctx = Context::new(&design, &resolved, Some("hot_imaging")).unwrap();
    let missing = thermal::transient(&ctx).unwrap_err().into_vec();
    assert!(
        missing
            .iter()
            .any(|m| { m.target == Some(Target::case("hot_imaging", Some("thermal_case"))) })
    );
}

#[test]
fn thermal_beta_only_overrides_case_beta_when_present() {
    let mut design = demo();
    let resolved = fune_model::resolve(&design);
    for condition_beta in [None, Some(15.0_f64)] {
        for condition in design.analysis.thermal.as_mut().unwrap().cases.values_mut() {
            condition.beta = condition_beta.map(|value| {
                serde_json::from_value(serde_json::json!({"value": value, "unit": "deg"})).unwrap()
            });
        }
        let ctx = Context::new(&design, &resolved, Some("hot_imaging")).unwrap();
        let expected = condition_beta.unwrap_or(60.0).to_radians();
        let transient = thermal::transient(&ctx).unwrap();
        assert!((transient.timeline.sampling.beta.unwrap().value() - expected).abs() < 1e-12);
        for case in thermal::steady(&ctx).unwrap().cases {
            let ThermalExposure::OrbitAverage { timeline } = case.exposure else {
                panic!("軌道平均の入熱であること");
            };
            assert!((timeline.sampling.beta.unwrap().value() - expected).abs() < 1e-12);
        }
    }
}
