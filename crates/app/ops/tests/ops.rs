//! Op の計画と適用（fixtures/demo-6u の写しに対して）。

use fune_model::design::{Net, NetKind, Revision};
use fune_model::value::{Quantity, Scalar, SettingValue, Value};
use fune_ops::{ApplyOptions, ModeCell, Op, OpError, apply, plan, to_cli};
use fune_store::{FileChange, Snapshot, load};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn demo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn snapshot() -> Snapshot {
    load(&demo()).unwrap()
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

fn set_mass(part: &str, value: f64, unit: &str) -> Op {
    Op::SetValue {
        part: part.into(),
        attr: "mass".into(),
        state: None,
        value: Value::Scalar(Scalar::new(value, unit)),
        from: None,
    }
}

#[test]
fn set_value_changes_exactly_one_line() {
    let snap = snapshot();
    let p = plan(&snap, &[set_mass("RW_X", 0.155, "kg")]).unwrap();
    assert_eq!(p.changes.len(), 1);
    assert_eq!(p.changes[0].path, "parts/AOCS/RW_X.toml");
    let (removed, added) = changed_lines(&p.changes[0]);
    assert_eq!(removed.len(), 1, "{removed:?}");
    // 出典を付けずに書いた値は、前の値の出典を引き継がない（EM 実測の値ではなくなるため）。
    let expected = r#"mass = { value = 0.155, unit = "kg" }"#;
    assert_eq!(added, vec![expected.to_string()]);
    assert!(added[0].contains("0.155"));
    let resolved = fune_model::resolve(&p.design);
    assert!((resolved.parts["RW_X"].scalar("mass").unwrap() - 0.155).abs() < 1e-12);
}

#[test]
fn new_override_line_is_aligned_and_alone() {
    let snap = snapshot();
    let p = plan(&snap, &[set_mass("RW_Y", 151.0, "g")]).unwrap();
    let (removed, added) = changed_lines(&p.changes[0]);
    assert!(removed.is_empty(), "{removed:?}");
    assert!(
        added.contains(&"mass = { value = 151.0, unit = \"g\" }".to_string()),
        "{added:?}"
    );
}

#[test]
fn state_cell_edit_is_one_line() {
    let snap = snapshot();
    let op = Op::SetValue {
        part: "HTR_BAT".into(),
        attr: "power".into(),
        state: Some("on".into()),
        value: Value::Scalar(Scalar::new(2.5, "W")),
        from: None,
    };
    let p = plan(&snap, &[op]).unwrap();
    let (removed, added) = changed_lines(&p.changes[0]);
    assert_eq!((removed.len(), added.len()), (1, 1));
    assert_eq!(added[0], "on  = { value = 2.5, unit = \"W\" }");
}

#[test]
fn unit_dimension_mismatch_is_rejected() {
    let snap = snapshot();
    match plan(&snap, &[set_mass("RW_X", 1.0, "W")]) {
        Err(OpError::Rejected { diagnostics }) => {
            assert!(diagnostics.iter().any(|d| d.code == "unit.dimension"
                && d.target == Some(fune_model::Target::attr("RW_X", "mass"))));
            assert_eq!(diagnostics[0].file.as_deref(), Some("parts/AOCS/RW_X.toml"));
            assert!(diagnostics[0].line.is_some());
        }
        other => panic!("{other:?}"),
    }
    let bad_unit = plan(&snap, &[set_mass("RW_X", 1.0, "kgg")]);
    assert!(matches!(bad_unit, Err(OpError::Rejected { .. })));
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
            },
            ModeCell {
                mode: "SAFE".into(),
                part: "GPS".into(),
                state: Some("on".into()),
                duty: BTreeMap::new(),
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
        }],
    };
    assert!(matches!(plan(&snap, &[bad]), Err(OpError::Rejected { .. })));
}

#[test]
fn create_move_and_remove_part() {
    let snap = snapshot();
    let create = Op::CreatePart {
        id: "RW_S".into(),
        group: Some("AOCS".into()),
        catalog: Some("local/rw_10mNms".into()),
        kind: None,
        count: None,
        label: None,
        mode_state: None,
    };
    let p = plan(&snap, std::slice::from_ref(&create)).unwrap();
    assert!(
        p.changes
            .iter()
            .any(|c| c.path == "parts/AOCS/RW_S.toml" && c.before.is_none())
    );
    assert_eq!(p.changes.len(), 5, "part file + 4 modes");
    assert_eq!(p.design.modes["SAFE"].states["RW_S"].state, "off");
    let after = Snapshot {
        root: snap.root.clone(),
        files: {
            let mut f = snap.files.clone();
            for c in &p.changes {
                if let Some(a) = &c.after {
                    f.insert(c.path.clone(), a.clone());
                }
            }
            f
        },
        design: p.design.clone(),
    };
    let moved = plan(
        &after,
        &[Op::MovePart {
            id: "RW_S".into(),
            to: Some("SPARE".into()),
        }],
    )
    .unwrap();
    assert!(moved.design.groups.contains_key("SPARE"));
    let removed = plan(&after, &[Op::RemovePart { id: "RW_S".into() }]).unwrap();
    assert!(!removed.design.parts.contains_key("RW_S"));
    assert!(!removed.design.modes["SAFE"].states.contains_key("RW_S"));
    assert!(matches!(
        plan(
            &snap,
            &[Op::RemovePart {
                id: "SAP_PY".into()
            }]
        ),
        Err(OpError::Invalid { .. })
    ));
    assert!(matches!(
        plan(
            &snap,
            &[Op::RemovePart {
                id: "BUS_FRAME".into()
            }]
        ),
        Err(OpError::Invalid { .. }) | Err(OpError::Rejected { .. })
    ));
}

#[test]
fn settings_and_nets() {
    let snap = snapshot();
    let ops = vec![
        Op::SetSetting {
            path: "analysis.power.dod_max".into(),
            value: Some(SettingValue::Number(0.25)),
        },
        Op::SetSetting {
            path: "mission.orbit.altitude".into(),
            value: Some(SettingValue::Quantity(Quantity::new(600.0, "km"))),
        },
        Op::CreateNet {
            net: {
                let mut n = Net::new("P3V3_PL", NetKind::Power);
                n.source = "PCDU.OUT1".parse().ok();
                n.rail = Some(Quantity::new(3.3, "V"));
                n
            },
        },
        Op::SetPort {
            part: "CAM".into(),
            port: "AUX".into(),
            net: Some("P3V3_PL".into()),
        },
    ];
    let p = plan(&snap, &ops).unwrap();
    assert_eq!(
        p.design.analysis.power.as_ref().unwrap().dod_max,
        Some(0.25)
    );
    assert_eq!(
        p.design
            .mission
            .orbit
            .as_ref()
            .unwrap()
            .altitude
            .as_ref()
            .unwrap()
            .value,
        600.0
    );
    assert_eq!(
        p.design.parts["CAM"].ports["AUX"]
            .net
            .as_ref()
            .map(|n| n.to_string()),
        Some("P3V3_PL".into())
    );
    let bad = Op::SetSetting {
        path: "mission.orbit.altitude".into(),
        value: Some(SettingValue::Quantity(Quantity::new(600.0, "kg"))),
    };
    assert!(matches!(plan(&snap, &[bad]), Err(OpError::Rejected { .. })));
    let removed = plan(
        &snap,
        &[Op::RemoveNet {
            id: "LVDS_STX".into(),
        }],
    )
    .unwrap();
    assert!(!removed.design.parts["STX"].ports.contains_key("DATA"));
}

#[test]
fn apply_writes_and_detects_conflicts() {
    let dir = tempfile::tempdir().unwrap();
    copy_dir(&demo(), dir.path());
    let before = load(dir.path()).unwrap();
    let stale = Revision("b3:stale".into());
    let conflict = apply(
        dir.path(),
        &[set_mass("RW_Z", 0.149, "kg")],
        &ApplyOptions {
            base_revision: Some(stale),
            dry_run: false,
        },
    );
    assert!(matches!(conflict, Err(OpError::Conflict { .. })));
    let dry = apply(
        dir.path(),
        &[set_mass("RW_Z", 0.149, "kg")],
        &ApplyOptions {
            base_revision: Some(before.design.revision.clone()),
            dry_run: true,
        },
    )
    .unwrap();
    assert!(!dry.written);
    assert_eq!(
        load(dir.path()).unwrap().design.revision,
        before.design.revision
    );
    let done = apply(
        dir.path(),
        &[set_mass("RW_Z", 0.149, "kg")],
        &ApplyOptions {
            base_revision: Some(before.design.revision.clone()),
            dry_run: false,
        },
    )
    .unwrap();
    assert!(done.written);
    let after = load(dir.path()).unwrap();
    assert_eq!(after.design.revision, done.plan.revision);
    assert!(after.design.parts["RW_Z"].attr.contains_key("mass"));
}

#[test]
fn net_field_edit_is_one_line() {
    use fune_model::design::NetField;
    let snap = snapshot();
    let op = Op::SetNetField {
        net: "P5V".into(),
        field: NetField::Rail(Some(Quantity::new(5.1, "V"))),
    };
    let p = plan(&snap, std::slice::from_ref(&op)).unwrap();
    assert_eq!(p.changes.len(), 1);
    let (removed, added) = changed_lines(&p.changes[0]);
    assert_eq!(
        (removed.len(), added),
        (
            1,
            vec!["rail   = { value = 5.1, unit = \"V\" }".to_string()]
        )
    );
    assert_eq!(to_cli(&op), vec!["fune net set P5V --rail \"5.1 V\""]);
    let unset = Op::SetNetField {
        net: "P5V".into(),
        field: NetField::Limit(None),
    };
    assert_eq!(to_cli(&unset), vec!["fune net unset P5V limit"]);
    let p = plan(&snap, &[unset]).unwrap();
    assert!(p.design.nets["P5V"].limit.is_none());
}

#[test]
fn view_edge_apply_persists_routes_without_changing_design_connections() {
    use fune_model::design::{ViewEdge, ViewPortSide};
    let dir = tempfile::tempdir().unwrap();
    copy_dir(&demo(), dir.path());
    let before = load(dir.path()).unwrap();
    let key = "P12V:PCDU.OUT3->RW_X.PWR";
    let edge = ViewEdge {
        points: vec![[0, 10], [80, 10], [80, 200]],
        source_side: None,
        target_side: Some(ViewPortSide::Top),
        source_offset: None,
        target_offset: None,
    };
    let op = Op::SetViewEdge {
        view: "electrical".into(),
        key: key.into(),
        edge: edge.clone(),
    };
    apply(dir.path(), &[op], &ApplyOptions::default()).unwrap();
    let after = load(dir.path()).unwrap();
    assert_eq!(after.design.views["electrical"].edge[key], edge);
    assert_eq!(before.design.nets, after.design.nets);
    assert_eq!(before.design.parts, after.design.parts);
    apply(
        dir.path(),
        &[Op::ResetView {
            view: "electrical".into(),
        }],
        &ApplyOptions::default(),
    )
    .unwrap();
    let reset = load(dir.path()).unwrap();
    assert!(reset.design.views["electrical"].edge.is_empty());
}
