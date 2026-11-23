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

    // DoD の判定は基準を `at_most` の無次元の値から取る
    let mut r = requirement();
    r.quantity = "Dimensionless".into();
    r.constraint.value = Some(Quantity::new(30.0, "%"));
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

/// 基準を要求から取る判定（`design::criteria::CRITERIA`）は、向きと次元が宣言と合う要求だけが基準になる。
#[test]
fn a_criterion_needs_the_declared_direction_and_dimension() {
    let dod = |kind, quantity: &str, value: Quantity| {
        let mut r = requirement();
        r.quantity = quantity.into();
        r.constraint.kind = kind;
        r.constraint.value = Some(value);
        r.verify = Verification {
            analysis: Some("power.energy_balance".into()),
            check: Some("dod".into()),
            budget: None,
        };
        r
    };
    let percent = || Quantity::new(30.0, "%");
    assert!(codes(dod(ConstraintKind::AtMost, "Dimensionless", percent())).is_empty());
    let wrong_kind = codes(dod(ConstraintKind::AtLeast, "Dimensionless", percent()));
    assert!(
        has(&wrong_kind, "requirement.criterion_kind"),
        "{wrong_kind:?}"
    );
    let wrong_dim = codes(dod(
        ConstraintKind::AtMost,
        "Mass",
        Quantity::new(1.0, "kg"),
    ));
    assert!(
        has(&wrong_dim, "requirement.criterion_dimension"),
        "{wrong_dim:?}"
    );
    // 判定のキーの族（`margin` は `margin:<回線>@<局>` のすべて）でも同じ
    let mut r = dod(
        ConstraintKind::AtMost,
        "Dimensionless",
        Quantity::new(3.0, "dB"),
    );
    r.verify.analysis = Some("comm.link_budget".into());
    r.verify.check = Some("margin".into());
    assert!(has(&codes(r), "requirement.criterion_kind"));
    // バジェットの上限は `at_most`
    let mut r = requirement();
    r.constraint.kind = ConstraintKind::AtLeast;
    r.verify.budget = Some(BudgetKind::Mass);
    assert!(has(&codes(r), "requirement.criterion_kind"));
}

/// 解析の入力を与える要求（ミッション期間）は、`at_least` の時間で、設計に 1 つ。
#[test]
fn a_mission_duration_requirement_is_an_at_least_time_and_unique() {
    let mission = |id: &str, kind, quantity: &str, value: Quantity| {
        let mut r = requirement();
        r.id = id.into();
        r.quantity = quantity.into();
        r.constraint.kind = kind;
        r.constraint.value = Some(value);
        r.input = Some(fune_model::design::RequirementInput::MissionDuration);
        r
    };
    let year = || Quantity::new(1.0, "yr");
    assert!(codes(mission("R1", ConstraintKind::AtLeast, "Time", year())).is_empty());
    let c = codes(mission("R1", ConstraintKind::AtMost, "Time", year()));
    assert!(has(&c, "requirement.input_kind"), "{c:?}");
    let c = codes(mission(
        "R1",
        ConstraintKind::AtLeast,
        "Mass",
        Quantity::new(1.0, "kg"),
    ));
    assert!(has(&c, "requirement.input_dimension"), "{c:?}");
    let mut design = Design::default();
    design
        .sources
        .insert("sys".into(), Source::new("sys", SourceKind::Judgement));
    for id in ["R1", "R2"] {
        let r = mission(id, ConstraintKind::AtLeast, "Time", year());
        design.requirements.insert(r.id.clone(), r);
    }
    let dup: Vec<_> = fune_model::check(&design)
        .into_iter()
        .filter(|d| d.code == "requirement.input_duplicate")
        .collect();
    // 先の要求（R1）が与え、後の要求（R2）が誤りになる
    assert_eq!(dup.len(), 1);
    assert!(dup[0].message.contains("R1"));
    assert_eq!(
        design.mission_duration().map(|c| c.requirement),
        Some("R1".to_owned())
    );
}
