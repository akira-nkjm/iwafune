//! まだ指されているフェーズ・モード・形態・出典・マヌーバを消そうとすると、指している場所を種類ごとに ID で挙げて拒む
//! （適用後の検査の「is not defined」を参照元の数だけ並べない）。fixtures/demo-6u に対して計画だけを立てる。

use fune_ops::{Op, OpError, plan};
use fune_store::{Snapshot, load};
use std::path::PathBuf;

fn snapshot() -> Snapshot {
    load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")).unwrap()
}

fn refusal(snap: &Snapshot, op: Op) -> String {
    match plan(snap, &[op]) {
        Err(OpError::Invalid { message }) => message,
        other => panic!("expected Invalid, got {:?}", other.map(|p| p.changes)),
    }
}

#[test]
fn a_referenced_phase_is_refused_once_with_its_cases() {
    let message = refusal(
        &snapshot(),
        Op::RemovePhase {
            id: "NOMINAL_OPS".into(),
        },
    );
    assert_eq!(
        message,
        "Phase `NOMINAL_OPS` cannot be removed: referred to by `phase` of case(s) \
         cold_safe, eclipse_worst, hot_imaging. Change those first"
    );
}

#[test]
fn a_referenced_mode_names_each_kind_of_referrer() {
    let message = refusal(
        &snapshot(),
        Op::RemoveMode {
            id: "NOMINAL".into(),
        },
    );
    assert!(
        message.starts_with(
            "Mode `NOMINAL` cannot be removed: referred to by `mode` of case(s) eclipse_worst; "
        ),
        "{message}"
    );
    for part in [
        "`mode_share` in analysis/power.toml",
        "named power sequence(s) in analysis/power.toml downlink, imaging",
        "`[law]` in analysis/aocs.toml",
        "`mode` of thermal case(s) cold_nominal",
        "`modes` of phase(s) DISPOSAL, LEOP, NOMINAL_OPS",
    ] {
        assert!(message.contains(part), "{part}: {message}");
    }
    assert!(message.ends_with("Change those first"), "{message}");
    assert!(!message.contains("is not defined"), "{message}");
}

#[test]
fn a_referenced_configuration_and_source_are_refused() {
    let snap = snapshot();
    let message = refusal(
        &snap,
        Op::RemoveConfiguration {
            id: "STOWED".into(),
        },
    );
    assert!(
        message.contains(
            "`configuration` in conditions/launch.toml; `configuration` of phase(s) LEOP"
        ),
        "{message}"
    );
    let message = refusal(&snap, Op::RemoveSource { id: "tbd".into() });
    assert!(
        message.starts_with("Source `tbd` cannot be removed: referred to by attribute value sources of part(s) HTR_BAT, HTR_CAM"),
        "{message}"
    );
}

#[test]
fn a_maneuver_used_by_an_orbit_is_refused_but_a_chain_is_relinked() {
    let snap = snapshot();
    // `after` でつながっているだけのマヌーバは、つなぎ直して消せる
    assert!(plan(&snap, &[Op::RemoveManeuver { id: "RAISE".into() }]).is_ok());
    // 軌道が遷移のマヌーバとして指しているものは、消すと軌道が決まらなくなるので拒む
    let mut snap = snap;
    let orbit = snap.files.get("conditions/orbit.toml").unwrap().to_owned();
    snap.files.insert(
        "conditions/orbit.toml".to_owned(),
        format!("maneuver = \"RAISE\"\n{orbit}"),
    );
    snap.design = fune_store::read::design_of(&snap.files);
    let message = refusal(&snap, Op::RemoveManeuver { id: "RAISE".into() });
    assert!(
        message.contains("`maneuver` in conditions/orbit.toml"),
        "{message}"
    );
}

#[test]
fn removing_after_the_referrers_are_changed_still_works() {
    let snap = snapshot();
    let ops = [
        Op::SetCasePhase {
            case: "cold_safe".into(),
            phase: None,
        },
        Op::SetCasePhase {
            case: "eclipse_worst".into(),
            phase: None,
        },
        Op::SetCasePhase {
            case: "hot_imaging".into(),
            phase: None,
        },
        Op::RemovePhase {
            id: "NOMINAL_OPS".into(),
        },
    ];
    let p = plan(&snap, &ops).unwrap();
    assert!(!p.design.phases.contains_key("NOMINAL_OPS"));
}
