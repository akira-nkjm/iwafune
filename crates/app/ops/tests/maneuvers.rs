//! マヌーバの Op（fixtures/demo-6u に対して計画だけを立てる）。

use fune_model::design::{
    Maneuver, ManeuverCategory, ManeuverConstraints, ManeuverMethod, ManeuverTarget,
};
use fune_model::value::Quantity;
use fune_ops::{Op, OpError, plan, to_cli};
use fune_store::{Snapshot, load};
use std::path::PathBuf;

fn snapshot() -> Snapshot {
    load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")).unwrap()
}

fn maneuver(id: &str, after: Option<&str>) -> Maneuver {
    Maneuver {
        id: id.into(),
        label: Some("試験".into()),
        category: ManeuverCategory::Insertion,
        method: ManeuverMethod::Impulsive,
        thruster: Some("RW_X".into()),
        after: after.map(str::to_owned),
        via_altitude: None,
        target: ManeuverTarget {
            altitude: Some(Quantity::new(600.0, "km")),
            ..ManeuverTarget::default()
        },
        constraints: ManeuverConstraints {
            sunlit_only: None,
            mode: Some("NOMINAL".into()),
            ..ManeuverConstraints::default()
        },
    }
}

fn after_of<'a>(p: &'a fune_ops::Plan, file: &str) -> &'a str {
    p.changes
        .iter()
        .find(|c| c.path == file)
        .and_then(|c| c.after.as_deref())
        .unwrap_or_default()
}

#[test]
fn maneuvers_are_created_once_and_written_in_canonical_form() {
    let snap = snapshot();
    let op = Op::CreateManeuver {
        maneuver: maneuver("T1", None),
    };
    let p = plan(&snap, std::slice::from_ref(&op)).unwrap();
    assert_eq!(
        after_of(&p, "maneuvers/T1.toml"),
        "label    = \"試験\"\ncategory = \"insertion\"\nmethod   = \"impulsive\"\nthruster = \"RW_X\"\n\n[target]\naltitude = { value = 600.0, unit = \"km\" }\n\n[constraints]\nmode = \"NOMINAL\"\n"
    );
    assert_eq!(
        to_cli(&op),
        [
            "fune maneuver add T1 --category insertion --method impulsive --label \"試験\" --thruster RW_X --altitude \"600 km\" --mode NOMINAL"
        ]
    );
    // 同じ ID をもう一度足すのは競合。set は書き換え
    let twice = plan(&snap, &[op.clone(), op]);
    assert!(
        matches!(twice, Err(OpError::AlreadyExists { .. })),
        "{twice:?}"
    );
    let set = Op::SetManeuver {
        maneuver: maneuver("T1", None),
    };
    assert!(plan(&snap, &[set.clone(), set]).is_ok());
}

#[test]
fn unknown_references_are_rejected() {
    let snap = snapshot();
    let mut m = maneuver("T1", None);
    m.thruster = Some("NO_SUCH_PART".into());
    let bad = plan(&snap, &[Op::SetManeuver { maneuver: m }]);
    assert!(matches!(bad, Err(OpError::Rejected { .. })), "{bad:?}");
    let mut m = maneuver("T1", None);
    m.constraints.mode = Some("NO_SUCH_MODE".into());
    assert!(plan(&snap, &[Op::SetManeuver { maneuver: m }]).is_err());
    // after がたどって自分に戻るのは誤り
    let cycle = plan(
        &snap,
        &[
            Op::SetManeuver {
                maneuver: maneuver("T1", Some("T2")),
            },
            Op::SetManeuver {
                maneuver: maneuver("T2", Some("T1")),
            },
        ],
    );
    assert!(cycle.is_err());
}

#[test]
fn removing_a_maneuver_relinks_its_successor() {
    let snap = snapshot();
    let p = plan(
        &snap,
        &[
            Op::SetManeuver {
                maneuver: maneuver("T1", None),
            },
            Op::SetManeuver {
                maneuver: maneuver("T2", Some("T1")),
            },
            Op::SetManeuver {
                maneuver: maneuver("T3", Some("T2")),
            },
            Op::RemoveManeuver { id: "T2".into() },
        ],
    )
    .unwrap();
    assert!(after_of(&p, "maneuvers/T3.toml").contains("after    = \"T1\""));
    assert!(p.changes.iter().all(|c| c.path != "maneuvers/T2.toml"));
    let missing = plan(&snap, &[Op::RemoveManeuver { id: "T9".into() }]);
    assert!(matches!(missing, Err(OpError::NotFound { .. })));
}

#[test]
fn renaming_parts_and_modes_rewrites_maneuvers() {
    let snap = snapshot();
    let p = plan(
        &snap,
        &[
            Op::SetManeuver {
                maneuver: maneuver("T1", None),
            },
            Op::RenamePart {
                id: "RW_X".into(),
                to: "RW_A".into(),
            },
            Op::RenameMode {
                id: "NOMINAL".into(),
                to: "NOMINAL2".into(),
            },
        ],
    )
    .unwrap();
    let text = after_of(&p, "maneuvers/T1.toml");
    assert!(text.contains("thruster = \"RW_A\""), "{text}");
    assert!(text.contains("mode = \"NOMINAL2\""), "{text}");
}
