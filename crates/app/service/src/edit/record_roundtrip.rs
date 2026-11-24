//! 編集の欄（GUI のダイアログ・`POST /ops/from_command`）が、レコードの欄を黙って落とさないことの確認。
//! レコードを編集の欄の初期値（`query::*::*_text`。ダイアログが開くときの値）にし、それをそのまま Op（`*_op`）に戻すと、
//! 同じレコードになる。デモの全レコードで確かめる。欄を落とす型（見えない欄を持つビューから作る置き換え）があると、
//! この比較が崩れる（`SetCaseEpoch` を足したのは、ケースにこの落とし穴があったため）。

use super::{maneuver_op, phase_op, requirement_op};
use crate::query::{maneuvers::maneuver_text, phases::phase_text, requirements::requirement_text};
use crate::workspace::Workspace;
use fune_ops::Op;
use std::path::Path;

fn design() -> fune_model::Design {
    Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
        .unwrap()
        .snapshot()
        .unwrap()
        .design
}

#[test]
fn maneuver_dialog_text_keeps_every_field() {
    let d = design();
    assert!(!d.maneuvers.is_empty());
    for (id, m) in &d.maneuvers {
        let Ok(Op::SetManeuver { maneuver }) = maneuver_op(&maneuver_text(m), false) else {
            panic!("{id}");
        };
        assert_eq!(&maneuver, m, "{id}");
    }
}

#[test]
fn requirement_dialog_text_keeps_every_field() {
    let d = design();
    assert!(!d.requirements.is_empty());
    for (id, r) in &d.requirements {
        let Ok(Op::SetRequirement { requirement }) = requirement_op(&requirement_text(r), false)
        else {
            panic!("{id}");
        };
        assert_eq!(&requirement, r, "{id}");
    }
}

/// フェーズのダイアログは見出しの欄だけを持ち、軌道と姿勢則の上書きは `SetPhase` が今のまま残す（Op の約束）。
#[test]
fn phase_dialog_text_keeps_every_heading_field() {
    let d = design();
    assert!(!d.phases.is_empty());
    for (id, p) in &d.phases {
        let Ok(Op::SetPhase { phase }) = phase_op(&phase_text(p), false) else {
            panic!("{id}");
        };
        let mut headings = p.clone();
        headings.orbit = None;
        headings.law.clear();
        assert_eq!(phase, headings, "{id}");
    }
}

/// デモに無い欄（日付つきの遷移・干渉の制約・太陽側の入力など）も、全部の欄を埋めたマヌーバと要求で確かめる
/// （欄を足したときに、編集の欄の対応を足し忘れると、ここが崩れる）。
#[test]
fn fully_populated_records_survive_the_dialog_text() {
    use fune_model::design::{
        CentralBodyKind, Constraint, ConstraintKind, DepartureBy, Maneuver, ManeuverCategory,
        ManeuverConstraints, ManeuverMethod, ManeuverSteering, ManeuverTarget, ManeuverTransfer,
        Quantification, Requirement, RequirementInput, Verification,
    };
    use fune_model::value::Quantity;
    let q = |v: f64, u: &str| Some(Quantity::new(v, u));
    let m = Maneuver {
        id: "MARS".into(),
        label: Some("Mars insertion".into()),
        category: ManeuverCategory::Insertion,
        method: ManeuverMethod::Impulsive,
        thruster: Some("EPT".into()),
        after: Some("RAISE".into()),
        via_altitude: q(500.0, "km"),
        delta_v: q(1.5, "km/s"),
        target: ManeuverTarget {
            body: Some(CentralBodyKind::Mars),
            altitude: None,
            perigee_altitude: q(300.0, "km"),
            apogee_altitude: q(900.0, "km"),
            inclination: q(40.0, "deg"),
        },
        constraints: ManeuverConstraints {
            sunlit_only: Some(true),
            mode: Some("NOMINAL".into()),
            duty: Some(0.25),
            steering: Some(ManeuverSteering::Inertial),
        },
        transfer: Some(ManeuverTransfer {
            departure: Some("2028-12-10T00:00:00Z".into()),
            arrival: Some("2029-09-25T00:00:00Z".into()),
            departure_by: Some(DepartureBy::Launcher),
            flybys: Vec::new(),
        }),
    };
    let Ok(Op::SetManeuver { maneuver }) = maneuver_op(&maneuver_text(&m), false) else {
        panic!("maneuver");
    };
    assert_eq!(maneuver, m);
    let r = Requirement {
        id: "REQ_LIFE".into(),
        label: Some("Lifetime".into()),
        statement: "The mission shall last".into(),
        statement_en: Some("The mission shall last".into()),
        quantity: "Time".into(),
        source: Some("sys_judgement".into()),
        input: Some(RequirementInput::MissionDuration),
        constraint: Constraint {
            kind: ConstraintKind::Range,
            value: q(1.0, "yr"),
            tolerance: q(30.0, "d"),
            min: q(1.0, "yr"),
            max: q(2.0, "yr"),
        },
        quantify: Quantification {
            all_cases: false,
            cases: vec!["hot_imaging".into()],
        },
        verify: Verification {
            analysis: Some("orbit.decay".into()),
            check: Some("lifetime".into()),
            budget: Some(fune_model::design::BudgetKind::Mass),
        },
    };
    let op = requirement_op(&requirement_text(&r), false);
    let Ok(Op::SetRequirement { requirement }) = op else {
        panic!("requirement: {op:?}");
    };
    assert_eq!(requirement, r);
}
