//! 名前の変更の Op（参照の書き換え）。fixtures/demo-6u に対して計画だけを立てる。

use fune_model::check;
use fune_ops::{Op, plan};
use fune_store::{Snapshot, load};
use std::path::PathBuf;

fn snapshot() -> Snapshot {
    load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")).unwrap()
}

#[test]
fn rename_part_updates_references() {
    let snap = snapshot();
    let p = plan(
        &snap,
        &[Op::RenamePart {
            id: "PCDU".into(),
            to: "EPS_PCDU".into(),
        }],
    )
    .unwrap();
    let paths: Vec<&str> = p.changes.iter().map(|c| c.path.as_str()).collect();
    for expected in [
        "parts/power/PCDU.toml",
        "parts/power/EPS_PCDU.toml",
        "modes/SAFE.toml",
        "nets/P12V.toml",
        "harness/H_PCDU_RWX.toml",
        "analysis/thermal.toml",
    ] {
        assert!(paths.contains(&expected), "{expected} not in {paths:?}");
    }
    let d = &p.design;
    assert!(d.modes["NOMINAL"].states.contains_key("EPS_PCDU"));
    let source = d.nets["P5V"].source.as_ref().map(ToString::to_string);
    assert_eq!(source.as_deref(), Some("EPS_PCDU.OUT2"));
    assert_eq!(d.harnesses["H_PCDU_RWX"].from.to_string(), "EPS_PCDU.OUT3");
    assert!(check(d).iter().all(|x| !x.is_error()));
}

/// 表面材を書いた外面（`[surface.<部品>]` のキー）も部品の名前の変更で書き換わる。
#[test]
fn rename_part_updates_thermal_surface() {
    let snap = snapshot();
    let p = plan(
        &snap,
        &[Op::RenamePart {
            id: "PANEL_PX".into(),
            to: "PANEL_XP".into(),
        }],
    )
    .unwrap();
    let t = p.design.analysis.thermal.as_ref().unwrap();
    assert!(t.surface.contains_key("PANEL_XP"));
    assert!(!t.surface.contains_key("PANEL_PX"));
    assert!(check(&p.design).iter().all(|x| !x.is_error()));
}

#[test]
fn rename_assembly_moves_children() {
    let snap = snapshot();
    let p = plan(
        &snap,
        &[Op::RenamePart {
            id: "SAP_PY".into(),
            to: "SAP_1".into(),
        }],
    )
    .unwrap();
    let paths: Vec<&str> = p.changes.iter().map(|c| c.path.as_str()).collect();
    assert!(paths.contains(&"parts/power/SAP_1/CELLS.toml"), "{paths:?}");
    assert_eq!(
        p.design.parts["CELLS"]
            .mount
            .as_ref()
            .map(|m| m.on.to_string()),
        Some("SAP_1".into())
    );
    assert!(
        p.design.analysis.thermal.as_ref().unwrap().nodes["SAP"].contains(&"SAP_1".to_string())
    );
}

#[test]
fn rename_mode_updates_settings_and_cases() {
    let snap = snapshot();
    let p = plan(
        &snap,
        &[Op::RenameMode {
            id: "MISSION".into(),
            to: "IMAGING".into(),
        }],
    )
    .unwrap();
    let d = &p.design;
    assert!(d.modes.contains_key("IMAGING"));
    assert!(
        d.analysis
            .power
            .as_ref()
            .unwrap()
            .mode_share
            .contains_key("IMAGING")
    );
    assert!(
        d.analysis
            .aocs
            .as_ref()
            .unwrap()
            .law
            .contains_key("IMAGING")
    );
    assert_eq!(
        d.cases["hot_imaging"].mode.as_ref().map(|m| m.to_string()),
        Some("IMAGING".into())
    );
}

#[test]
fn view_layout_is_written_sorted_with_integers() {
    use fune_ops::ViewNodePos;
    let snap = snapshot();
    let node = |id: &str, x, y| ViewNodePos {
        id: id.into(),
        x,
        y,
        w: None,
        h: None,
    };
    let op = Op::SetViewLayout {
        view: "electrical".into(),
        nodes: vec![
            node("RW_X", 420, 120),
            node("PCDU", 100, 80),
            ViewNodePos {
                id: "power".into(),
                x: 0,
                y: 0,
                w: Some(640),
                h: Some(360),
            },
        ],
        remove: vec![],
    };
    let p = plan(&snap, std::slice::from_ref(&op)).unwrap();
    assert_eq!(p.changes.len(), 1);
    assert_eq!(
        p.changes[0].after.as_deref(),
        Some(
            "[nodes]\nPCDU  = { x = 100, y = 80 }\nRW_X  = { x = 420, y = 120 }\npower = { x = 0, y = 0, w = 640, h = 360 }\n"
        )
    );
    assert_eq!(p.design.views["electrical"].nodes["RW_X"].x, 420);
    assert_eq!(
        fune_ops::to_cli(&op)[0],
        "fune view set electrical RW_X --pos 420,120"
    );
    assert_eq!(
        fune_ops::to_cli(&op)[2],
        "fune view set electrical power --pos 0,0 --size 640,360"
    );
    let files = {
        let mut f = snap.files.clone();
        f.insert("view/electrical.toml", p.changes[0].after.clone().unwrap());
        f
    };
    assert!(
        fune_store::format::format_files(&files).is_empty(),
        "view file is not canonical"
    );
}

#[test]
fn edge_routes_are_toml_10_round_tripped_and_resettable() {
    use fune_model::design::{ViewEdge, ViewPortSide};
    let snap = snapshot();
    let key = r#"[["P12V"],"card:PCDU::OUT1","card:RW_X::PWR"]"#;
    let edge = ViewEdge {
        points: vec![[-20, 10], [80, 10], [80, 120]],
        source_side: Some(ViewPortSide::Right),
        target_side: None,
        source_offset: None,
        target_offset: None,
    };
    let op = Op::SetViewEdge {
        view: "electrical".into(),
        key: key.into(),
        edge: edge.clone(),
    };
    let p = plan(&snap, &[op]).unwrap();
    assert_eq!(p.design.views["electrical"].edge[key], edge);
    let mut updated = snap;
    fune_store::apply_changes(&mut updated.files, &p.changes);
    updated.design = fune_store::read::design_of(&updated.files);
    assert_eq!(updated.design.views["electrical"].edge[key], edge);
    let unset = plan(
        &updated,
        &[Op::UnsetViewEdge {
            view: "electrical".into(),
            key: key.into(),
        }],
    )
    .unwrap();
    assert!(unset.design.views["electrical"].edge.is_empty());
    let reset = plan(
        &updated,
        &[Op::ResetView {
            view: "electrical".into(),
        }],
    )
    .unwrap();
    assert!(reset.design.views["electrical"].edge.is_empty());
    assert!(reset.design.views["electrical"].nodes.is_empty());
    let bad = Op::SetViewEdge {
        view: "electrical".into(),
        key: key.into(),
        edge: ViewEdge {
            points: vec![[0, 0], [10, 10]],
            ..edge
        },
    };
    assert!(plan(&updated, &[bad]).is_err());
}

#[test]
fn removing_the_last_view_entries_drops_the_empty_tables() {
    use fune_model::design::ViewEdge;
    use fune_ops::ViewNodePos;
    let snap = snapshot();
    let key = "P12V:PCDU.OUT1>RW_X.PWR";
    // 書いてから消す（系統図で元に戻したとき）: 空の [nodes]・[edge] を残さない
    let ops = [
        Op::SetViewLayout {
            view: "electrical".into(),
            nodes: vec![ViewNodePos {
                id: "RW_X".into(),
                x: 420,
                y: 120,
                w: None,
                h: None,
            }],
            remove: vec![],
        },
        Op::SetViewEdge {
            view: "electrical".into(),
            key: key.into(),
            edge: ViewEdge {
                points: vec![[0, 0], [0, 40]],
                source_side: None,
                target_side: None,
                source_offset: None,
                target_offset: None,
            },
        },
        Op::SetViewLayout {
            view: "electrical".into(),
            nodes: vec![],
            remove: vec!["RW_X".into()],
        },
        Op::UnsetViewEdge {
            view: "electrical".into(),
            key: key.into(),
        },
    ];
    let p = plan(&snap, &ops).unwrap();
    let after = p.changes[0].after.as_deref().unwrap_or("");
    assert!(
        !after.contains("[nodes]") && !after.contains("[edge]"),
        "{after:?}"
    );
    assert!(p.design.views["electrical"].nodes.is_empty());
    assert!(p.design.views["electrical"].edge.is_empty());
}

#[test]
fn rename_system_to_an_arbitrary_group_is_rejected() {
    let snap = snapshot();
    let err = plan(
        &snap,
        &[Op::RenameGroup {
            id: "power".into(),
            to: "ELEC".into(),
        }],
    );
    assert!(err.is_err());
}

/// 入れ子のフォルダの名前はこれまでどおり変更でき、図のキーも追従する。
#[test]
fn rename_nested_group_moves_directory_and_view_keys() {
    let mut snap = snapshot();
    snap.files
        .insert("parts/power/spares/P.toml", "kind = \"component\"\n");
    snap.files
        .insert("view/nested.toml", "[nodes]\nspares = { x = 0, y = 0 }\n");
    snap.design = fune_store::read::design_of(&snap.files);
    let p = plan(
        &snap,
        &[Op::RenameGroup {
            id: "spares".into(),
            to: "backup".into(),
        }],
    )
    .unwrap();
    assert!(
        p.changes
            .iter()
            .any(|c| c.path == "parts/power/backup/P.toml")
    );
    assert!(p.design.groups.contains_key("backup"));
    assert!(p.design.views["nested"].nodes.contains_key("backup"));
    assert_eq!(
        p.design.system_of("P"),
        Some(fune_model::design::AnalysisSystem::Power)
    );
}

#[test]
fn rename_group_rejects_a_taken_or_missing_name() {
    let snap = snapshot();
    let taken = plan(
        &snap,
        &[Op::RenameGroup {
            id: "power".into(),
            to: "cdh".into(),
        }],
    );
    assert!(taken.is_err(), "大文字小文字を無視して重なる名前は使えない");
    let missing = plan(
        &snap,
        &[Op::RenameGroup {
            id: "NOPE".into(),
            to: "X".into(),
        }],
    );
    assert!(missing.is_err());
}

#[test]
fn rename_part_updates_eclipse_rows() {
    let p = plan(
        &snapshot(),
        &[
            Op::SetModeStates {
                cells: vec![fune_ops::ModeCell {
                    mode: "NOMINAL".into(),
                    part: "HTR_BAT".into(),
                    state: Some("on".into()),
                    duty: Default::default(),
                    active: None,
                    spare_state: None,
                    eclipse: true,
                }],
            },
            Op::RenamePart {
                id: "HTR_BAT".into(),
                to: "HTR_NEW".into(),
            },
        ],
    )
    .unwrap();
    let m = &p.design.modes["NOMINAL"];
    assert!(m.states.contains_key("HTR_NEW"));
    assert!(m.eclipse.contains_key("HTR_NEW"));
    assert!(!m.eclipse.contains_key("HTR_BAT"));
}
