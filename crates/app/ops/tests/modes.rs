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

/// 移動と名前変更・削除・追加を同じ計画で行い、一覧の参照と順序を保つ。
#[test]
fn mode_edits_preserve_display_order() {
    use fune_ops::ModePosition;
    let mut snap = snapshot();
    snap.files = fune_store::FileSet::new();
    snap.files.insert("fune.toml", "[project]\nname = \"sample\"\n[modes]\norder = [\"NOMINAL\", \"MISSION\", \"DOWNLINK\", \"SAFE\"]\n");
    for id in ["NOMINAL", "MISSION", "DOWNLINK", "SAFE"] {
        snap.files.insert(format!("modes/{id}.toml"), "[states]\n");
    }
    snap.design = fune_store::read::design_of(&snap.files);
    let result = plan(
        &snap,
        &[
            Op::MoveMode {
                id: "SAFE".into(),
                position: ModePosition::First,
            },
            Op::RenameMode {
                id: "SAFE".into(),
                to: "SURVIVAL".into(),
            },
            Op::RemoveMode {
                id: "MISSION".into(),
            },
            Op::CreateMode {
                id: "COMMISSION".into(),
                label: None,
                description: None,
                copy_from: Some("NOMINAL".into()),
            },
        ],
    )
    .unwrap();
    assert_eq!(
        result
            .design
            .ordered_modes()
            .iter()
            .map(|m| m.id.as_str())
            .collect::<Vec<_>>(),
        ["SURVIVAL", "NOMINAL", "DOWNLINK", "COMMISSION"]
    );
    assert_eq!(
        result.design.project.modes.as_ref().unwrap().order,
        ["SURVIVAL", "NOMINAL", "DOWNLINK", "COMMISSION"]
    );
    let text = result
        .changes
        .iter()
        .find(|c| c.path == "fune.toml")
        .unwrap()
        .after
        .as_ref()
        .unwrap();
    assert!(!text.contains("SAFE") && !text.contains("\"MISSION\""));
    assert!(matches!(
        plan(
            &snap,
            &[Op::MoveMode {
                id: "NO_SUCH_MODE".into(),
                position: ModePosition::First
            }]
        ),
        Err(OpError::NotFound { .. })
    ));
    assert!(
        plan(
            &snap,
            &[Op::MoveMode {
                id: "SAFE".into(),
                position: ModePosition::Before("SAFE".into())
            }]
        )
        .is_err()
    );
    assert!(
        plan(
            &snap,
            &[Op::MoveMode {
                id: "SAFE".into(),
                position: ModePosition::After("MISSING".into())
            }]
        )
        .is_err()
    );
}

/// 省略した一覧は追加時にも作らない。
#[test]
fn create_mode_without_order_keeps_it_omitted() {
    let mut snap = snapshot();
    let text = snap.files.get("fune.toml").unwrap();
    let mut doc = fune_store::profile::parse("fune.toml", text).unwrap();
    doc.remove("modes");
    snap.files.insert("fune.toml", doc.to_string());
    snap.design = fune_store::read::design_of(&snap.files);
    let result = plan(
        &snap,
        &[Op::CreateMode {
            id: "COMMISSION".into(),
            label: None,
            description: None,
            copy_from: None,
        }],
    )
    .unwrap();
    assert!(result.design.project.modes.is_none());
    assert!(result.changes.iter().all(|c| c.path != "fune.toml"));
}
