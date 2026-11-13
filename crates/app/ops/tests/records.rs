//! 取付点・解析ケース・出典・ハーネスの Op（fixtures/demo-6u に対して計画だけを立てる）。

use fune_model::Id;
use fune_model::design::{Case, Pose, Source, SourceKind, UncertaintySide};
use fune_model::value::{Quantity, Vector3};
use fune_ops::{Op, OpError, plan, to_cli};
use fune_store::{Snapshot, load};
use std::path::PathBuf;

fn snapshot() -> Snapshot {
    load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")).unwrap()
}

fn one_line_change(p: &fune_ops::Plan) -> (String, String) {
    assert_eq!(p.changes.len(), 1);
    let c = &p.changes[0];
    let before: Vec<&str> = c.before.as_deref().unwrap_or("").lines().collect();
    let after: Vec<&str> = c.after.as_deref().unwrap_or("").lines().collect();
    let removed: Vec<&&str> = before.iter().filter(|l| !after.contains(l)).collect();
    let added: Vec<&&str> = after.iter().filter(|l| !before.contains(l)).collect();
    assert_eq!(
        (removed.len(), added.len()),
        (1, 1),
        "{removed:?} {added:?}"
    );
    (removed[0].to_string(), added[0].to_string())
}

#[test]
fn cases_are_created_and_updated_minimally() {
    let snap = snapshot();
    let mut case = snap.design.cases["eclipse_worst"].clone();
    case.orbits = Some(30);
    let p = plan(&snap, &[Op::SetCase { case: case.clone() }]).unwrap();
    let (_, added) = one_line_change(&p);
    assert_eq!(added, "orbits       = 30");
    let new_case = Case {
        id: Id::new("launch_day"),
        label: Some("打ち上げ当日".into()),
        description: None,
        mode: Some(Id::new("SAFE")),
        side: Some(UncertaintySide::Max),
        thermal_case: None,
        beta: Some(Quantity::new(20.0, "deg")),
        orbits: None,
    };
    let op = Op::SetCase { case: new_case };
    let p = plan(&snap, std::slice::from_ref(&op)).unwrap();
    assert_eq!(
        p.changes[0].after.as_deref(),
        Some(
            "label = \"打ち上げ当日\"\nmode  = \"SAFE\"\nside  = \"max\"\nbeta  = { value = 20.0, unit = \"deg\" }\n"
        )
    );
    assert_eq!(
        to_cli(&op),
        vec![
            "fune case set launch_day --label \"打ち上げ当日\" --mode SAFE --side max --beta \"20 deg\""
        ]
    );
    assert!(
        plan(
            &snap,
            &[Op::RemoveCase {
                id: "cold_safe".into()
            }]
        )
        .is_ok()
    );
}

#[test]
fn sources_and_harnesses() {
    let snap = snapshot();
    let source = Source {
        id: Id::new("tvac_2027_01"),
        kind: SourceKind::Measured,
        label: Some("熱真空試験".into()),
        reference: Some("TR-TVAC-01".into()),
        date: Some("2027-01-20".into()),
        ..Source::new("tvac_2027_01", SourceKind::Measured)
    };
    let p = plan(&snap, &[Op::SetSource { source }]).unwrap();
    assert!(
        p.changes[0]
            .after
            .as_deref()
            .unwrap()
            .contains("ref   = \"TR-TVAC-01\"")
    );
    // 値から参照されている出典は消せない（ref.source の error が増える）。
    let used = plan(
        &snap,
        &[Op::RemoveSource {
            id: "em_test_2026_10".into(),
        }],
    );
    assert!(matches!(used, Err(OpError::Rejected { .. })));
    // 出典の付いた長さは `[length]` の下に 1 行 1 要素なので、値だけを変えると 1 行だけが変わる。
    let mut h = snap.design.harnesses["H_PCDU_RWX"].clone();
    if let Some(length) = h.length.as_mut() {
        length.value = 0.42;
    }
    let p = plan(&snap, &[Op::SetHarness { harness: h.clone() }]).unwrap();
    let (_, added) = one_line_change(&p);
    assert_eq!(added, "value       = 0.42");
    // 出典を外すと 1 行のインライン表に戻る（`=` の揃えも fmt と同じ）。
    h.length = Some(Quantity::new(0.42, "m"));
    let p = plan(&snap, &[Op::SetHarness { harness: h }]).unwrap();
    let text = p.changes[0].after.as_deref().unwrap();
    assert!(
        text.ends_with("cores  = 2\nlength = { value = 0.42, unit = \"m\" }\n"),
        "{text}"
    );
    assert_eq!(
        fune_store::format::format_text(&p.changes[0].path, text).unwrap(),
        text
    );
    assert!(
        plan(
            &snap,
            &[Op::RemoveHarness {
                id: "H_OBC_CAM".into()
            }]
        )
        .is_ok()
    );
}

#[test]
fn mount_points_are_added_and_removed() {
    let snap = snapshot();
    let pose = Pose {
        position: Some(Vector3 {
            x: 10.0,
            y: 20.0,
            z: 30.0,
            unit: Some("mm".into()),
            frame: None,
        }),
        orientation: None,
    };
    let op = Op::SetMountPoint {
        part: "PANEL_PZ".into(),
        point: "ANT".into(),
        pose: Some(pose),
    };
    let p = plan(&snap, std::slice::from_ref(&op)).unwrap();
    assert!(p.design.parts["PANEL_PZ"].mount_points.contains_key("ANT"));
    assert_eq!(
        to_cli(&op),
        vec!["fune part point PANEL_PZ ANT --position \"10 20 30 mm\""]
    );
    let rm = Op::SetMountPoint {
        part: "PANEL_PZ".into(),
        point: "CAM_PORT".into(),
        pose: None,
    };
    // CAM が CAM_PORT に付いているので、消すと ref.mount_point の error が増える。
    assert!(matches!(plan(&snap, &[rm]), Err(OpError::Rejected { .. })));
}

#[test]
fn every_op_has_a_cli_form() {
    let snap = snapshot();
    let d = &snap.design;
    let ops = vec![
        Op::RemovePart { id: "X".into() },
        Op::RenamePart {
            id: "X".into(),
            to: "Y".into(),
        },
        Op::MovePart {
            id: "X".into(),
            to: None,
        },
        Op::UnsetValue {
            part: "X".into(),
            attr: "mass".into(),
            state: None,
        },
        Op::SetPort {
            part: "X".into(),
            port: "P".into(),
            net: None,
        },
        Op::SetMountPoint {
            part: "X".into(),
            point: "P".into(),
            pose: None,
        },
        Op::CreateMode {
            id: "M".into(),
            label: None,
            description: None,
            copy_from: None,
        },
        Op::RemoveMode { id: "M".into() },
        Op::RenameMode {
            id: "M".into(),
            to: "N".into(),
        },
        Op::CreateNet {
            net: d.nets["P5V"].clone(),
        },
        Op::UpdateNet {
            net: d.nets["CAN_BUS"].clone(),
        },
        Op::RemoveNet { id: "P5V".into() },
        Op::SetCase {
            case: d.cases["cold_safe"].clone(),
        },
        Op::RemoveCase {
            id: "cold_safe".into(),
        },
        Op::SetSource {
            source: d.sources["tbd"].clone(),
        },
        Op::RemoveSource { id: "tbd".into() },
        Op::SetHarness {
            harness: d.harnesses["H_OBC_CAM"].clone(),
        },
        Op::RemoveHarness {
            id: "H_OBC_CAM".into(),
        },
        Op::CreateKind {
            def: fune_model::builtin::kinds()["heater"].clone(),
        },
        Op::CreateAttribute {
            def: fune_model::builtin::attributes()["mass"].clone(),
        },
        Op::SetSetting {
            path: "analysis.cdh.storage_margin".into(),
            value: None,
        },
        Op::RemoveCatalogItem {
            id: "local/x".into(),
        },
        Op::RenameCatalogItem {
            from: "local/x".into(),
            to: "local/y".into(),
        },
    ];
    for op in &ops {
        let lines = to_cli(op);
        assert!(
            !lines.is_empty() && lines.iter().all(|l| l.starts_with("fune ")),
            "{op:?} -> {lines:?}"
        );
    }
}

/// 共通設定も既存の SetSetting と正規形で往復し、未設定に戻せる（デモは hot_imaging なので cold_safe に替える）。
#[test]
fn default_analysis_case_setting_round_trips() {
    use fune_model::value::SettingValue;
    let mut snap = snapshot();
    let op = Op::SetSetting {
        path: "analysis.common.default_case".to_owned(),
        value: Some(SettingValue::Text("cold_safe".to_owned())),
    };
    let p = plan(&snap, &[op]).unwrap();
    let change = &p.changes[0];
    assert_eq!(change.path, "analysis/common.toml");
    let text = change.after.as_deref().unwrap();
    assert_eq!(text, "default_case = \"cold_safe\"\n");
    assert_eq!(
        fune_store::format::format_text(&change.path, text).unwrap(),
        text
    );
    fune_store::apply_changes(&mut snap.files, &p.changes);
    snap.design = fune_store::read::design_of(&snap.files);
    assert_eq!(
        snap.design
            .analysis
            .common
            .as_ref()
            .unwrap()
            .default_case
            .as_deref(),
        Some("cold_safe")
    );
    assert!(
        plan(
            &snap,
            &[Op::RemoveCase {
                id: "cold_safe".to_owned()
            }]
        )
        .is_err()
    );
    let p = plan(
        &snap,
        &[Op::SetSetting {
            path: "analysis.common.default_case".to_owned(),
            value: None,
        }],
    )
    .unwrap();
    assert_eq!(
        p.design.analysis.common.as_ref().unwrap().default_case,
        None
    );
}
