//! モードの行列の編集（Op → 行ごとの差分）。

use fune_ops::{ModeCell, Op, OpError, plan, to_cli};
use fune_store::{FileChange, Snapshot, load};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn snapshot() -> Snapshot {
    load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")).unwrap()
}

/// 変わった行の数（前後の行の多重集合の差）。
fn changed_lines(change: &FileChange) -> (Vec<String>, Vec<String>) {
    let before: Vec<&str> = change.before.as_deref().unwrap_or("").lines().collect();
    let after: Vec<&str> = change.after.as_deref().unwrap_or("").lines().collect();
    let removed = before
        .iter()
        .filter(|l| !after.contains(l))
        .map(|l| l.to_string())
        .collect();
    let added = after
        .iter()
        .filter(|l| !before.contains(l))
        .map(|l| l.to_string())
        .collect();
    (removed, added)
}

#[test]
fn mode_table_edit_writes_one_row_per_cell() {
    let snap = snapshot();
    let op = Op::SetModeStates {
        cells: vec![
            ModeCell {
                mode: "NOMINAL".into(),
                part: "UHF_TRX".into(),
                state: Some("rx".into()),
                duty: BTreeMap::from([("tx".to_string(), 0.05)]),
                active: None,
                spare_state: None,
                eclipse: false,
            },
            ModeCell {
                mode: "SAFE".into(),
                part: "GPS".into(),
                state: Some("on".into()),
                duty: BTreeMap::new(),
                active: None,
                spare_state: None,
                eclipse: false,
            },
        ],
    };
    let p = plan(&snap, std::slice::from_ref(&op)).unwrap();
    assert_eq!(p.changes.len(), 2);
    for c in &p.changes {
        let (removed, added) = changed_lines(c);
        assert_eq!((removed.len(), added.len()), (1, 1), "{}", c.path);
    }
    assert_eq!(p.design.modes["NOMINAL"].states["UHF_TRX"].duty["tx"], 0.05);
    assert_eq!(
        to_cli(&op),
        vec![
            "fune mode set NOMINAL UHF_TRX rx --duty tx=0.05",
            "fune mode set SAFE GPS on"
        ]
    );
    let bad = Op::SetModeStates {
        cells: vec![ModeCell {
            mode: "SAFE".into(),
            part: "GPS".into(),
            state: Some("tx".into()),
            duty: BTreeMap::new(),
            active: None,
            spare_state: None,
            eclipse: false,
        }],
    };
    assert!(matches!(plan(&snap, &[bad]), Err(OpError::Rejected { .. })));
}
