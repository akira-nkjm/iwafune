//! 要求の検査（`requirement.*`・`ref.case`・`ref.source`・`ref.analysis`・`unit.dimension`）。
use fune_model::design::{
    BudgetKind, Constraint, ConstraintKind, Quantification, Requirement, Source, SourceKind,
    Verification,
};
use fune_model::value::Quantity;
use fune_model::{Design, Severity};

fn requirement() -> Requirement {
    Requirement {
        id: "R1".into(),
        label: None,
        statement: "The mass shall not exceed 12 kg.".into(),
        statement_en: None,
        quantity: "Mass".into(),
        source: Some("sys".into()),
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

/// 要求 1 つだけを持つ設計の診断のコード（error と warning）。
fn codes(r: Requirement) -> Vec<String> {
    let mut design = Design::default();
    design
        .sources
        .insert("sys".into(), Source::new("sys", SourceKind::Judgement));
    design.requirements.insert(r.id.clone(), r);
    fune_model::check(&design)
        .into_iter()
        .filter(|d| d.severity != Severity::Info)
        .map(|d| d.code)
        .collect()
}

fn has(codes: &[String], code: &str) -> bool {
    codes.iter().any(|c| c == code)
}

#[test]
fn a_well_formed_requirement_has_no_problems() {
    assert!(codes(requirement()).is_empty());
}

#[test]
fn constraint_values_follow_the_kind_and_the_quantity() {
    let mut r = requirement();
    r.constraint.value = Some(Quantity::new(12.0, "W"));
    assert!(has(&codes(r), "unit.dimension"));

    let mut r = requirement();
    r.constraint.value = None;
    assert!(has(&codes(r), "requirement.constraint_missing"));

    let mut r = requirement();
    r.constraint.max = Some(Quantity::new(1.0, "kg"));
    assert!(has(&codes(r), "requirement.constraint_extra"));

    let mut r = requirement();
    r.constraint.kind = ConstraintKind::Range;
    r.constraint.value = None;
    r.constraint.min = Some(Quantity::new(2.0, "kg"));
    r.constraint.max = Some(Quantity::new(1000.0, "g"));
    assert!(has(&codes(r), "requirement.range"));

    let mut r = requirement();
    r.constraint.kind = ConstraintKind::Equal;
    r.constraint.tolerance = Some(Quantity::new(-1.0, "kg"));
    assert!(has(&codes(r), "requirement.tolerance"));

    let mut r = requirement();
    r.quantity = "Heaviness".into();
    assert!(has(&codes(r), "requirement.quantity"));
}

#[test]
fn references_must_exist() {
    let mut r = requirement();
    r.source = Some("missing".into());
    assert!(has(&codes(r), "ref.source"));

    let mut r = requirement();
    r.source = None;
    assert!(has(&codes(r), "requirement.source_missing"));

    let mut r = requirement();
    r.quantify.cases = vec!["nope".into()];
    assert!(has(&codes(r), "ref.case"));

    let mut r = requirement();
    r.quantify.all_cases = true;
    r.quantify.cases = vec!["nope".into()];
    assert!(has(&codes(r), "requirement.quantify_conflict"));
}

#[test]
fn the_verification_target_is_one_existing_analysis_or_a_matching_budget() {
    let verify = |analysis: Option<&str>, check: Option<&str>, budget| Verification {
        analysis: analysis.map(str::to_owned),
        check: check.map(str::to_owned),
        budget,
    };
    let mut r = requirement();
    r.verify = verify(Some("power.no_such"), Some("dod"), None);
    assert!(has(&codes(r), "ref.analysis"));

    let mut r = requirement();
    r.verify = verify(Some("power.energy_balance"), Some("dod"), None);
    assert!(codes(r).is_empty());

    let mut r = requirement();
    r.verify = verify(None, Some("dod"), None);
    assert!(has(&codes(r), "requirement.verify_incomplete"));

    let mut r = requirement();
    r.verify = verify(Some("power.energy_balance"), None, Some(BudgetKind::Mass));
    assert!(has(&codes(r), "requirement.verify_conflict"));

    let mut r = requirement();
    r.verify = verify(None, None, Some(BudgetKind::Power));
    assert!(has(&codes(r), "requirement.verify_dimension"));
    let mut r = requirement();
    r.verify = verify(None, None, Some(BudgetKind::Mass));
    assert!(codes(r).is_empty());
}
