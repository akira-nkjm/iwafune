//! 定義・フィールド・取付の Op（fixtures/demo-6u に対して計画だけを立てる）。

use fune_model::design::{AttributeDef, KindDef, Maturity, PartField};
use fune_model::value::{Quaternion, ValueType, Vector3};
use fune_model::{Id, Severity};
use fune_ops::{Op, OpError, plan, to_cli};
use fune_store::{Snapshot, load};
use std::path::PathBuf;

fn snapshot() -> Snapshot {
    load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")).unwrap()
}

fn attr_def(id: &str, dimension: &str, units: &[&str]) -> AttributeDef {
    AttributeDef {
        id: Id::new(id),
        label: "残留磁気".into(),
        label_en: Some("Residual dipole".into()),
        description: None,
        value_type: ValueType::Scalar,
        by_state: false,
        dimension: Some(dimension.into()),
        units: units.iter().map(|u| u.to_string()).collect(),
        display_unit: None,
        min: None,
        max: None,
        symbols: vec![],
        budget: None,
        default: None,
        unit_vector: false,
        fov_axis: None,
        section: Some("aocs".into()),
    }
}

#[test]
fn create_attribute_checks_dimension_and_units() {
    let snap = snapshot();
    let good = Op::CreateAttribute {
        def: attr_def(
            "residual_moment",
            "MagneticDipoleMoment",
            &["A*m^2", "mA*m^2"],
        ),
    };
    let p = plan(&snap, std::slice::from_ref(&good)).unwrap();
    let text = p.changes[0].after.clone().unwrap();
    assert_eq!(p.changes[0].path, "defs/attribute/residual_moment.toml");
    assert!(text.starts_with("label     = \"残留磁気\"\nlabel_en  = \"Residual dipole\"\ntype      = \"scalar\"\ndimension = \"MagneticDipoleMoment\""), "{text}");
    assert!(p.design.attributes.contains_key("residual_moment"));
    assert!(to_cli(&good)[0].starts_with(
        "fune attr add residual_moment --label \"残留磁気\" --label-en \"Residual dipole\""
    ));
    let bad = Op::CreateAttribute {
        def: attr_def("bad_one", "MagneticDipoleMoment", &["A*m^2", "W"]),
    };
    assert!(matches!(plan(&snap, &[bad]), Err(OpError::Invalid { .. })));
    let taken = Op::CreateAttribute {
        def: attr_def("mass", "Mass", &["kg"]),
    };
    assert!(matches!(
        plan(&snap, &[taken]),
        Err(OpError::AlreadyExists { .. })
    ));
}

/// 部品・カタログ品のフィールド名と同じ ID の属性は作れない（規則は検査の `attribute_def_problems` の 1 か所）。
#[test]
fn create_attribute_rejects_field_names() {
    let snap = snapshot();
    for id in ["count", "label", "part_number", "Candidates"] {
        let op = Op::CreateAttribute {
            def: attr_def(id, "MagneticDipoleMoment", &["A*m^2"]),
        };
        match plan(&snap, &[op]) {
            Err(OpError::Invalid { message }) => {
                assert!(message.contains("field name"), "{id}: {message}");
            }
            other => panic!("attribute `{id}` was created: {other:?}"),
        }
    }
}

#[test]
fn create_kind_and_use_it() {
    let snap = snapshot();
    let kind = KindDef {
        id: Id::new("deorbit_sail"),
        label: "展開膜".into(),
        label_en: Some("Deorbit sail".into()),
        description: None,
        extends: Some(Id::new("component")),
        role: None,
        states: vec!["stowed".into(), "deployed".into()],
        required: vec![Id::new("area")],
        optional: vec![],
        ports: Default::default(),
    };
    let ops = vec![
        Op::CreateKind { def: kind },
        Op::CreatePart {
            id: "SAIL".into(),
            group: Some("structure".into()),
            group_under: vec![],
            catalog: None,
            kind: Some("deorbit_sail".into()),
            count: None,
            label: None,
            mode_state: Some("stowed".into()),
        },
    ];
    let p = plan(&snap, &ops).unwrap();
    assert_eq!(p.design.modes["NOMINAL"].states["SAIL"].state, "stowed");
    // 必須の属性（mass・area）が未設定なので warning が出るが、error ではない。
    assert!(
        p.diagnostics
            .iter()
            .any(|d| d.code == "attr.missing" && d.severity == Severity::Warning)
    );
}

#[test]
fn set_field_place_and_unset() {
    let snap = snapshot();
    let ops = vec![
        Op::SetField {
            part: "STX".into(),
            field: PartField::Label(Some("S バンド送信機".into())),
        },
        Op::SetField {
            part: "STX".into(),
            field: PartField::Maturity(Some(Maturity::Modified)),
        },
        Op::Place {
            part: "GPS".into(),
            on: Some("PANEL_PZ".into()),
            point: Some("CAM_PORT".into()),
            position: Some(Vector3 {
                x: 30.0,
                y: 0.0,
                z: 0.0,
                unit: Some("mm".into()),
                frame: None,
            }),
            orientation: Some(Quaternion::IDENTITY),
        },
        Op::UnsetValue {
            part: "RW_X".into(),
            attr: "mass".into(),
            state: None,
        },
    ];
    let p = plan(&snap, &ops).unwrap();
    let d = &p.design;
    assert_eq!(d.parts["STX"].label.as_deref(), Some("S バンド送信機"));
    assert_eq!(d.parts["STX"].maturity, Some(Maturity::Modified));
    assert_eq!(
        d.parts["GPS"].mount.as_ref().map(|m| m.on.to_string()),
        Some("PANEL_PZ".into())
    );
    assert!(!d.parts["RW_X"].attr.contains_key("mass"));
    let resolved = fune_model::resolve(d);
    assert!((resolved.parts["RW_X"].scalar("mass").unwrap() - 0.150).abs() < 1e-12);
    let gps = p
        .changes
        .iter()
        .find(|c| c.path == "parts/aocs/GPS.toml")
        .unwrap();
    let after = gps.after.as_deref().unwrap();
    assert!(after.contains("[mount]\non    = \"PANEL_PZ\"\npoint = \"CAM_PORT\"\n\n[pose]\nposition    = { x = 30.0, y = 0.0, z = 0.0, unit = \"mm\" }"), "{after}");
    let again = Op::UnsetValue {
        part: "RW_Y".into(),
        attr: "mass".into(),
        state: None,
    };
    assert!(matches!(
        plan(&snap, &[again]),
        Err(OpError::NotFound { .. })
    ));
    let dangling = Op::Place {
        part: "GPS".into(),
        on: Some("NOPE".into()),
        point: None,
        position: None,
        orientation: None,
    };
    assert!(matches!(
        plan(&snap, &[dangling]),
        Err(OpError::Rejected { .. })
    ));
}
