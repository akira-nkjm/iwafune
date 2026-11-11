//! 参照の索引（`Design::references`）と、ops が検査の規則（`check`・`id::clash`）をそのまま使うこと。
//! fixtures/demo-6u に対して計画だけを立てる。

use fune_model::Id;
use fune_model::design::{HarnessEnd, NetEnd, RefSite, Referent};
use fune_ops::{Op, OpError, ViewNodePos, plan};
use fune_store::{Snapshot, load};
use std::path::PathBuf;

fn snapshot() -> Snapshot {
    load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")).unwrap()
}

/// 索引は、名前の変更で書き換える場所と同じものを返す。カタログ品の一覧と削除を拒む文も同じ索引から作る。
#[test]
fn the_reference_index_lists_every_site() {
    let snap = snapshot();
    let d = &snap.design;
    let sites = d.references(Referent::Part("PCDU"));
    for mode in d.modes.keys() {
        let row = RefSite::ModeRow {
            mode: mode.to_string(),
        };
        assert!(sites.contains(&row), "{mode}");
    }
    assert!(sites.iter().any(|s| matches!(
        s,
        RefSite::NetPort { net, end: NetEnd::Source, .. } if net == "P5V"
    )));
    assert!(sites.iter().any(|s| matches!(
        s,
        RefSite::HarnessPort { harness, end: HarnessEnd::From, .. } if harness == "H_PCDU_RWX"
    )));
    assert!(
        sites
            .iter()
            .any(|s| matches!(s, RefSite::ThermalNode { .. }))
    );
    let mode = d.references(Referent::Mode("MISSION"));
    assert!(mode.contains(&RefSite::PowerModeShare));
    assert!(mode.contains(&RefSite::AttitudeLaw));
    assert!(mode.contains(&RefSite::CaseMode {
        case: "hot_imaging".into()
    }));
    let users = d.catalog_part_users("local/rw_10mNms");
    assert!(users.contains(&"RW_X".to_string()), "{users:?}");
    let wires: Vec<String> = d
        .references(Referent::Catalog("local/awg24_ptfe"))
        .iter()
        .filter_map(|s| match s {
            RefSite::HarnessWire { harness } => Some(format!("harness/{harness}")),
            _ => None,
        })
        .collect();
    assert!(!wires.is_empty());
    assert_eq!(d.catalog_users("local/awg24_ptfe"), wires);
}

/// 図の配置のキーは、グループと同じく部品の名前の変更でも書き換わる（同じ索引の `ViewNode`）。
#[test]
fn renaming_keeps_view_positions() {
    let snap = snapshot();
    let node = |id: &str| ViewNodePos {
        id: id.into(),
        x: 10,
        y: 20,
        w: None,
        h: None,
    };
    let p = plan(
        &snap,
        &[
            Op::SetViewLayout {
                view: "electrical".into(),
                nodes: vec![node("PCDU"), node("power")],
                remove: vec![],
            },
            Op::RenamePart {
                id: "PCDU".into(),
                to: "MAIN_PCDU".into(),
            },
        ],
    )
    .unwrap();
    let keys: Vec<&str> = p.design.views["electrical"]
        .nodes
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys, ["MAIN_PCDU", "power"]);
}

/// `catalog` も `kind` も無い部品は、検査の `part.no_definition` で拒まれる（ops に別の規則を持たない）。
#[test]
fn a_part_without_definition_is_rejected_by_the_check() {
    let undefined = Op::CreatePart {
        id: "RW_T".into(),
        group: Some("aocs".into()),
        catalog: None,
        kind: None,
        count: None,
        label: None,
        mode_state: None,
    };
    match plan(&snapshot(), &[undefined]) {
        Err(OpError::Rejected { diagnostics }) => assert!(
            diagnostics.iter().any(|d| d.code == "part.no_definition"),
            "{diagnostics:?}"
        ),
        other => panic!("{other:?}"),
    }
}

/// 手で引いた線の経路の形も、計画の最後の検査（`view.route`）で拒まれる。
#[test]
fn a_diagonal_route_is_rejected_by_the_check() {
    let bad = Op::SetViewEdge {
        view: "electrical".into(),
        key: "P5V:PCDU.OUT2>CAM.PWR".into(),
        edge: fune_model::design::ViewEdge {
            points: vec![[0, 0], [10, 10]],
            source_side: None,
            target_side: None,
            source_offset: None,
            target_offset: None,
        },
    };
    match plan(&snapshot(), &[bad]) {
        Err(OpError::Rejected { diagnostics }) => assert!(
            diagnostics.iter().any(|d| d.code == "view.route"),
            "{diagnostics:?}"
        ),
        other => panic!("{other:?}"),
    }
}

/// 大文字小文字だけが違う ID は、部品・ケース（出典・ハーネスも）で同じ規則で重ねない。
#[test]
fn ids_that_differ_only_in_case_are_taken() {
    let snap = snapshot();
    let part = Op::CreatePart {
        id: "rw_x".into(),
        group: Some("aocs".into()),
        catalog: Some("local/rw_10mNms".into()),
        kind: None,
        count: None,
        label: None,
        mode_state: None,
    };
    assert!(matches!(
        plan(&snap, &[part]),
        Err(OpError::AlreadyExists { .. })
    ));
    let mut case = snap.design.cases["cold_safe"].clone();
    case.id = Id::new("Cold_Safe");
    assert!(matches!(
        plan(&snap, &[Op::SetCase { case }]),
        Err(OpError::AlreadyExists { .. })
    ));
    // 同じ綴りなら書き換え
    let same = snap.design.cases["cold_safe"].clone();
    assert!(plan(&snap, &[Op::SetCase { case: same }]).is_ok());
}
