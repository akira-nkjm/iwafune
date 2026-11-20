//! 定義（属性・種別）の置き換えと削除の Op（fixtures/demo-6u に対して計画を立てる）。

use fune_model::builtin;
use fune_model::design::{AttributeDef, KindDef};
use fune_ops::{Op, OpError, plan, to_cli};
use fune_store::{Snapshot, apply_changes, format::format_text, load, read::design_of};
use std::path::PathBuf;

fn snapshot() -> Snapshot {
    load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")).unwrap()
}

/// 計画の変更を当てた後のスナップショット（次の計画の元にする）。
fn applied(snap: &Snapshot, ops: &[Op]) -> Snapshot {
    let p = plan(snap, ops).unwrap();
    let mut files = snap.files.clone();
    apply_changes(&mut files, &p.changes);
    Snapshot {
        root: snap.root.clone(),
        design: design_of(&files),
        files,
    }
}

fn demo_attr() -> AttributeDef {
    snapshot().design.attributes["linear_density"].clone()
}

fn demo_kind() -> KindDef {
    snapshot().design.kinds["wire"].clone()
}

fn new_kind(id: &str, extends: Option<&str>) -> KindDef {
    let mut k: KindDef = serde_json::from_value(serde_json::json!({ "label": id })).unwrap();
    k.id = id.into();
    k.extends = extends.map(Into::into);
    k
}

fn message(result: Result<fune_ops::Plan, OpError>) -> String {
    match result {
        Err(OpError::Invalid { message }) => message,
        other => panic!("expected Invalid, got {:?}", other.map(|p| p.changes)),
    }
}

#[test]
fn update_attribute_rewrites_exactly_that_file() {
    let snap = snapshot();
    let mut def = demo_attr();
    def.label = "線材の単位長さ質量".into();
    def.label_en = Some("Linear density".into());
    let op = Op::UpdateAttribute { def };
    let p = plan(&snap, std::slice::from_ref(&op)).unwrap();
    assert_eq!(p.changes.len(), 1);
    let change = &p.changes[0];
    assert_eq!(change.path, "defs/attribute/linear_density.toml");
    let after = change.after.as_deref().unwrap();
    assert!(
        after.contains("label        = \"線材の単位長さ質量\""),
        "{after}"
    );
    assert_eq!(format_text(&change.path, after).unwrap(), after);
    assert_eq!(
        p.design.attributes["linear_density"].label,
        "線材の単位長さ質量"
    );
    assert_eq!(
        to_cli(&op)[0],
        "fune attr replace linear_density --label \"線材の単位長さ質量\" --label-en \"Linear density\" \
         --type scalar --units g/m,kg/m --display-unit g/m --min 0 --section mass"
    );
}

#[test]
fn update_attribute_validates_like_create() {
    let snap = snapshot();
    let mut def = demo_attr();
    def.units = vec!["g/m".into(), "W".into()];
    let text = message(plan(&snap, &[Op::UpdateAttribute { def }]));
    assert!(text.contains("W"), "{text}");
}

/// 既存の値と合わなくなる置き換えは、ほかの Op と同じく適用後の検査（新しい error）で拒まれる。
#[test]
fn update_attribute_that_breaks_existing_values_is_rejected() {
    let snap = snapshot();
    let mut def = demo_attr();
    def.units = vec!["kg".into()];
    def.display_unit = Some("kg".into());
    assert!(matches!(
        plan(&snap, &[Op::UpdateAttribute { def }]),
        Err(OpError::Rejected { .. })
    ));
}

#[test]
fn update_kind_rewrites_exactly_that_file() {
    let snap = snapshot();
    let mut def = demo_kind();
    def.label_en = Some("Wire stock".into());
    let op = Op::UpdateKind { def };
    let p = plan(&snap, std::slice::from_ref(&op)).unwrap();
    assert_eq!(p.changes.len(), 1);
    assert_eq!(p.changes[0].path, "defs/kind/wire.toml");
    let after = p.changes[0].after.as_deref().unwrap();
    assert!(after.contains("label_en = \"Wire stock\""), "{after}");
    assert_eq!(format_text(&p.changes[0].path, after).unwrap(), after);
    assert!(
        to_cli(&op)[0].starts_with("fune kind replace wire --label Wire --label-en \"Wire stock\"")
    );
}

#[test]
fn delete_removes_the_file_when_unused() {
    let snap = applied(
        &snapshot(),
        &[
            Op::CreateKind {
                def: new_kind("spare_kind", None),
            },
            Op::CreateAttribute {
                def: AttributeDef {
                    id: "spare_attr".into(),
                    ..demo_attr()
                },
            },
        ],
    );
    for (op, path, cli) in [
        (
            Op::DeleteKind {
                id: "spare_kind".into(),
            },
            "defs/kind/spare_kind.toml",
            "fune kind rm spare_kind",
        ),
        (
            Op::DeleteAttribute {
                id: "spare_attr".into(),
            },
            "defs/attribute/spare_attr.toml",
            "fune attr rm spare_attr",
        ),
    ] {
        let p = plan(&snap, std::slice::from_ref(&op)).unwrap();
        assert_eq!(p.changes.len(), 1);
        assert_eq!(p.changes[0].path, path);
        assert!(p.changes[0].after.is_none());
        assert_eq!(to_cli(&op), [cli]);
    }
}

#[test]
fn delete_attribute_is_refused_while_used() {
    let snap = snapshot();
    // 種別が挙げ、カタログ品が値を書いている
    let text = message(plan(
        &snap,
        &[Op::DeleteAttribute {
            id: "linear_density".into(),
        }],
    ));
    assert!(
        text.contains("kind wire") && text.contains("catalog item local/awg24_ptfe"),
        "{text}"
    );
    assert!(text.contains("cannot be deleted"), "{text}");
    // 部品が値を書いている
    let ops = [
        Op::CreateAttribute {
            def: AttributeDef {
                id: "spare_attr".into(),
                ..demo_attr()
            },
        },
        Op::SetValue {
            part: "RW_X".into(),
            attr: "spare_attr".into(),
            state: None,
            value: fune_model::value::Value::Scalar(fune_model::value::Scalar::new(1.0, "g/m")),
            from: None,
        },
        Op::DeleteAttribute {
            id: "spare_attr".into(),
        },
    ];
    let text = message(plan(&snap, &ops));
    assert!(text.contains("part RW_X"), "{text}");
}

#[test]
fn delete_kind_is_refused_while_used() {
    let snap = snapshot();
    let text = message(plan(&snap, &[Op::DeleteKind { id: "wire".into() }]));
    assert!(text.contains("catalog item local/awg24_ptfe"), "{text}");
    let ops = [
        Op::CreateKind {
            def: new_kind("sail", None),
        },
        Op::CreatePart {
            id: "SAIL".into(),
            group: Some("structure".into()),
            group_under: vec![],
            catalog: None,
            kind: Some("sail".into()),
            count: None,
            label: None,
            mode_state: None,
        },
        Op::DeleteKind { id: "sail".into() },
    ];
    assert!(message(plan(&snap, &ops)).contains("part SAIL"));
    let ops = [
        Op::CreateKind {
            def: new_kind("base_kind", None),
        },
        Op::CreateKind {
            def: new_kind("child_kind", Some("base_kind")),
        },
        Op::DeleteKind {
            id: "base_kind".into(),
        },
    ];
    assert!(message(plan(&snap, &ops)).contains("kind child_kind"));
}

#[test]
fn builtin_definitions_cannot_be_changed_or_deleted() {
    let snap = snapshot();
    let ops = [
        Op::UpdateAttribute {
            def: builtin::attributes()["mass"].clone(),
        },
        Op::DeleteAttribute { id: "mass".into() },
        Op::UpdateKind {
            def: builtin::kinds()["heater"].clone(),
        },
        Op::DeleteKind {
            id: "heater".into(),
        },
    ];
    for op in ops {
        let text = message(plan(&snap, &[op]));
        assert!(text.contains("built-in"), "{text}");
    }
}

#[test]
fn unknown_definitions_are_not_found() {
    let snap = snapshot();
    let ops = [
        Op::UpdateAttribute {
            def: AttributeDef {
                id: "nope".into(),
                ..demo_attr()
            },
        },
        Op::DeleteAttribute { id: "nope".into() },
        Op::UpdateKind {
            def: new_kind("nope", None),
        },
        Op::DeleteKind { id: "nope".into() },
    ];
    for op in ops {
        assert!(matches!(plan(&snap, &[op]), Err(OpError::NotFound { .. })));
    }
}
