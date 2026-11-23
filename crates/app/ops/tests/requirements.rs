//! 要求の Op（fixtures/demo-6u に対して計画だけを立てる）。

use fune_model::design::{Constraint, ConstraintKind, Quantification, Requirement, Verification};
use fune_model::value::Quantity;
use fune_ops::{Op, OpError, plan, to_cli};
use fune_store::{Snapshot, load};
use std::path::PathBuf;

fn snapshot() -> Snapshot {
    load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")).unwrap()
}

fn requirement(id: &str) -> Requirement {
    Requirement {
        id: id.into(),
        label: None,
        statement: "The mass shall not exceed the limit.".into(),
        statement_en: None,
        quantity: "Mass".into(),
        source: Some("sys_judgement".into()),
        input: None,
        constraint: Constraint {
            kind: ConstraintKind::AtMost,
            value: Some(Quantity::new(12.0, "kg")),
            tolerance: None,
            min: None,
            max: None,
        },
        quantify: Quantification::default(),
        verify: Verification::default(),
    }
}

fn written<'a>(p: &'a fune_ops::Plan, file: &str) -> &'a str {
    p.changes
        .iter()
        .find(|c| c.path == file)
        .and_then(|c| c.after.as_deref())
        .unwrap_or_default()
}

#[test]
fn requirements_are_created_once_in_canonical_form() {
    let snap = snapshot();
    let op = Op::CreateRequirement {
        requirement: requirement("T_REQ"),
    };
    let p = plan(&snap, std::slice::from_ref(&op)).unwrap();
    let text = written(&p, "requirements/T_REQ.toml");
    assert!(text.contains("[constraint]"), "{text}");
    assert!(
        text.contains("value = { value = 12.0, unit = \"kg\" }"),
        "{text}"
    );
    assert!(
        !text.contains("quantify") && !text.contains("verify"),
        "{text}"
    );
    assert!(!text.contains("id ="), "{text}");
    assert_eq!(
        to_cli(&op),
        [
            r#"fune requirement add T_REQ --statement "The mass shall not exceed the limit." --quantity Mass --source sys_judgement --kind at_most --value "12 kg""#
        ]
    );
}

#[test]
fn a_requirement_that_fails_the_check_is_rejected() {
    let snap = snapshot();
    // 質量の要求に温度の単位
    let mut r = requirement("T_BAD");
    r.constraint.value = Some(Quantity::new(12.0, "degC"));
    let op = Op::CreateRequirement { requirement: r };
    assert!(matches!(
        plan(&snap, std::slice::from_ref(&op)),
        Err(OpError::Rejected { .. })
    ));
    // 無いケース・無い出典・無い解析
    let mut r = requirement("T_BAD");
    r.quantify.cases = vec!["no_such_case".into()];
    r.source = Some("no_such_source".into());
    r.verify.analysis = Some("power.no_such".into());
    let op = Op::SetRequirement { requirement: r };
    assert!(matches!(
        plan(&snap, std::slice::from_ref(&op)),
        Err(OpError::Rejected { .. })
    ));
}

#[test]
fn ids_follow_the_id_rules_and_removal_needs_an_existing_requirement() {
    let snap = snapshot();
    let bad = Op::CreateRequirement {
        requirement: requirement("bad id"),
    };
    assert!(plan(&snap, std::slice::from_ref(&bad)).is_err());
    let missing = Op::RemoveRequirement { id: "NOPE".into() };
    assert!(matches!(
        plan(&snap, std::slice::from_ref(&missing)),
        Err(OpError::NotFound { .. })
    ));
}

/// 基準が要求へ移った設定（`budget.mass.limit`・`analysis.power.dod_max` など）は、書き込みを拒む（編集は要求で行う）。
/// 消す（`unset`）のは通る。
#[test]
fn moved_criterion_settings_cannot_be_set_but_can_be_unset() {
    use fune_model::value::SettingValue;
    let snap = snapshot();
    for path in [
        "budget.mass.limit",
        "analysis.power.dod_max",
        "analysis.comm.min_margin",
        "analysis.comm.link.s_down.pfd_limit",
        "conditions.orbit.duration",
    ] {
        let set = Op::SetSetting {
            path: path.into(),
            value: Some(SettingValue::Number(1.0)),
        };
        let err = plan(&snap, &[set]).unwrap_err().to_string();
        assert!(err.contains("fune requirement set"), "{path}: {err}");
    }
    let unset = Op::SetSetting {
        path: "analysis.power.dod_max".into(),
        value: None,
    };
    assert!(plan(&snap, &[unset]).is_ok());
}
