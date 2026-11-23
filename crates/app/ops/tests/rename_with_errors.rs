//! 元から error のあるエンティティの名前の変更（`plan::renamed`）。適用前の error を新しい名前に写して比べるので、
//! 名前の変更だけでは拒まれず、名前の変更で増えた本当に新しい error は拒まれる。

use fune_model::check;
use fune_ops::{Op, OpError, plan};
use fune_store::{Snapshot, load};
use std::path::PathBuf;

fn snapshot() -> Snapshot {
    load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")).unwrap()
}

/// 部品 GPS の取付先を存在しない部品にして、GPS に元から error がある設計。
fn with_broken_gps() -> Snapshot {
    let mut snap = snapshot();
    let path = "parts/aocs/GPS.toml";
    let text = snap
        .files
        .get(path)
        .unwrap()
        .replace("\"BUS_FRAME\"", "\"NOPE\"");
    snap.files.insert(path, text);
    snap.design = fune_store::read::design_of(&snap.files);
    snap
}

fn errors_of(design: &fune_model::Design) -> Vec<String> {
    check(design)
        .iter()
        .filter(|d| d.is_error())
        .map(|d| format!("{} {:?} {}", d.code, d.path(), d.message))
        .collect()
}

#[test]
fn an_entity_with_an_existing_error_can_be_renamed() {
    let snap = with_broken_gps();
    let before = errors_of(&snap.design);
    assert!(before.iter().any(|e| e.contains("GPS")), "{before:?}");
    let p = plan(
        &snap,
        &[Op::RenamePart {
            id: "GPS".into(),
            to: "GPS_A".into(),
        }],
    )
    .expect("the pre-existing error must not block the rename");
    let after = errors_of(&p.design);
    assert!(after.iter().any(|e| e.contains("GPS_A")), "{after:?}");
}

#[test]
fn a_rename_that_adds_a_new_error_is_still_rejected() {
    let snap = with_broken_gps();
    // 壊れた GPS の名前の変更と同時に、別の部品のポートを存在しないネットにする（本当に新しい error）
    let ops = [
        Op::RenamePart {
            id: "GPS".into(),
            to: "GPS_A".into(),
        },
        Op::SetPort {
            part: "STT".into(),
            port: "PWR".into(),
            net: Some("NOPE_NET".into()),
        },
    ];
    match plan(&snap, &ops) {
        Err(OpError::Rejected { diagnostics }) => {
            let paths: Vec<_> = diagnostics.iter().filter_map(|d| d.path()).collect();
            assert_eq!(paths, ["STT.PWR"], "{diagnostics:?}");
        }
        other => panic!(
            "expected rejection, got {:?}",
            other.map(|p| p.changes.len())
        ),
    }
}
