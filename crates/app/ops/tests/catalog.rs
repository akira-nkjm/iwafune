//! カタログ品の Op（fixtures/demo-6u に対して計画だけを立てる）。

use fune_model::design::{CatalogField, Maturity};
use fune_model::value::{Scalar, Value};
use fune_ops::{Op, OpError, Plan, plan, to_cli};
use fune_store::read::design_of;
use fune_store::{Snapshot, format, load};
use std::path::PathBuf;

fn snapshot() -> Snapshot {
    load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")).unwrap()
}

fn one_line(p: &Plan) -> (String, String) {
    assert_eq!(
        p.changes.len(),
        1,
        "{:?}",
        p.changes.iter().map(|c| &c.path).collect::<Vec<_>>()
    );
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

fn set(item: &str, attr: &str, state: Option<&str>, value: f64, unit: &str) -> Op {
    Op::SetCatalogValue {
        item: item.into(),
        attr: attr.into(),
        state: state.map(Into::into),
        value: Value::Scalar(Scalar::new(value, unit)),
        from: None,
    }
}

#[test]
fn catalog_values_and_fields_change_one_line() {
    let snap = snapshot();
    let op = set("local/rw_10mNms", "max_torque", None, 1.2, "mN*m");
    let p = plan(&snap, std::slice::from_ref(&op)).unwrap();
    assert_eq!(p.changes[0].path, "catalog/rw_10mNms.toml");
    assert_eq!(
        one_line(&p).1,
        "max_torque    = { value = 1.2, unit = \"mN*m\" }"
    );
    let resolved = fune_model::resolve(&p.design);
    assert!((resolved.parts["RW_Y"].scalar("max_torque").unwrap() - 1.2e-3).abs() < 1e-15);
    assert_eq!(
        to_cli(&op),
        vec!["fune catalog set local/rw_10mNms max_torque \"1.2 mN*m\""]
    );
    let cell = set("rw_10mNms", "power", Some("on"), 0.65, "W");
    assert_eq!(
        one_line(&plan(&snap, &[cell]).unwrap()).1,
        "on   = { value = 0.65, unit = \"W\" }"
    );
    let field = Op::SetCatalogField {
        item: "local/rw_10mNms".into(),
        field: CatalogField::PartNumber(Some("RW-10B".into())),
    };
    assert_eq!(
        one_line(&plan(&snap, std::slice::from_ref(&field)).unwrap()).1,
        "part_number  = \"RW-10B\""
    );
    assert_eq!(
        to_cli(&field),
        vec!["fune catalog set local/rw_10mNms part_number RW-10B"]
    );
    let tags = Op::SetCatalogField {
        item: "local/rw_10mNms".into(),
        field: CatalogField::Tags(vec!["qualified".into(), "domestic".into()]),
    };
    // 見本の品はすでに別のタグを持つ。書き換えたあとのファイルに新しい並びがある
    let planned = plan(&snap, std::slice::from_ref(&tags)).unwrap();
    let after = planned.changes[0].after.as_deref().unwrap_or("");
    assert!(
        after.contains("tags         = [\"qualified\", \"domestic\"]"),
        "{after}"
    );
    assert_eq!(
        to_cli(&tags),
        vec!["fune catalog set local/rw_10mNms tags qualified,domestic"]
    );
    let no_tags = Op::SetCatalogField {
        item: "local/rw_10mNms".into(),
        field: CatalogField::Tags(vec![]),
    };
    assert_eq!(
        to_cli(&no_tags),
        vec!["fune catalog unset local/rw_10mNms tags"]
    );
    let unset = Op::UnsetCatalogValue {
        item: "local/rw_10mNms".into(),
        attr: "hk_rate".into(),
        state: None,
    };
    assert!(plan(&snap, std::slice::from_ref(&unset)).is_ok());
    assert_eq!(
        to_cli(&unset),
        vec!["fune catalog unset local/rw_10mNms hk_rate"]
    );
    let missing = Op::UnsetCatalogValue {
        item: "local/rw_10mNms".into(),
        attr: "thrust".into(),
        state: None,
    };
    assert!(matches!(
        plan(&snap, &[missing]),
        Err(OpError::NotFound { .. })
    ));
    let bad = set("local/rw_10mNms", "max_torque", None, 1.2, "kg");
    assert!(matches!(plan(&snap, &[bad]), Err(OpError::Rejected { .. })));
}

#[test]
fn create_from_a_part_and_remove() {
    let snap = snapshot();
    let create = Op::CreateCatalogItem {
        id: "rw_10mNms_em".into(),
        kind: "reaction_wheel".into(),
        label: Some("RW 10 mN·m·s（EM 実測）".into()),
        label_en: None,
        maturity: Some(Maturity::Modified),
        copy_from_part: Some("RW_X".into()),
        dir: None,
    };
    let p = plan(&snap, std::slice::from_ref(&create)).unwrap();
    assert_eq!(p.changes.len(), 1);
    assert_eq!(p.changes[0].path, "catalog/rw_10mNms_em.toml");
    let item = &p.design.catalog["local/rw_10mNms_em"];
    assert_eq!(item.source.as_deref(), Some("ds_vendor"));
    assert!(item.ports.contains_key("PWR"));
    let mass = match &item.attr["mass"].value {
        Value::Scalar(s) => s.value,
        other => panic!("{other:?}"),
    };
    assert_eq!(mass, 0.152);
    assert!(matches!(item.attr["power"].value, Value::ByState(_)));
    let mut files = snap.files.clone();
    files.insert(
        p.changes[0].path.clone(),
        p.changes[0].after.clone().unwrap(),
    );
    assert!(
        format::format_files(&files).is_empty(),
        "{}",
        p.changes[0].after.as_deref().unwrap()
    );
    assert!(to_cli(&create)[0].ends_with("--maturity modified --from-part RW_X"));
    let both = plan(
        &snap,
        &[
            create,
            Op::RemoveCatalogItem {
                id: "local/rw_10mNms_em".into(),
            },
        ],
    )
    .unwrap();
    assert!(both.changes.is_empty());
    match plan(
        &snap,
        &[Op::RemoveCatalogItem {
            id: "local/rw_10mNms".into(),
        }],
    ) {
        Err(OpError::Invalid { message }) => assert!(
            message.contains("RW_X") && message.contains("RW_Z"),
            "{message}"
        ),
        other => panic!("{other:?}"),
    }
}

#[test]
fn rename_rewrites_part_and_harness_references() {
    let snap = snapshot();
    let p = plan(
        &snap,
        &[Op::RenameCatalogItem {
            from: "local/sband_tx_2m".into(),
            to: "local/sband_tx_4m".into(),
        }],
    )
    .unwrap();
    let paths: Vec<&str> = p.changes.iter().map(|c| c.path.as_str()).collect();
    assert_eq!(
        paths,
        vec![
            "catalog/sband_tx_2m.toml",
            "catalog/sband_tx_4m.toml",
            "parts/comm/STX.toml"
        ]
    );
    let stx = &p.design.parts["STX"];
    assert_eq!(stx.catalog.as_deref(), Some("local/sband_tx_4m"));
    assert_eq!(stx.candidates, vec!["local/sband_tx_4m".to_string()]);
    let w = plan(
        &snap,
        &[Op::RenameCatalogItem {
            from: "local/awg24_ptfe".into(),
            to: "awg24_ptfe_v2".into(),
        }],
    )
    .unwrap();
    assert!(
        w.design
            .harnesses
            .values()
            .all(|h| h.wire.as_deref() == Some("local/awg24_ptfe_v2"))
    );
    let taken = Op::RenameCatalogItem {
        from: "local/mtq_0p2".into(),
        to: "local/STT_NANO".into(),
    };
    assert!(matches!(
        plan(&snap, &[taken]),
        Err(OpError::AlreadyExists { .. })
    ));
}

#[test]
fn shared_catalog_items_are_read_only() {
    let snap = snapshot();
    let mut files = snap.files.clone();
    let project = files.get("fune.toml").unwrap().to_string();
    files.insert("fune.toml", format!("{project}\n[[catalog]]\nname = \"lab\"\ngit  = \"https://git.example.com/example-lab/parts-catalog\"\nrev  = \"v2026.11\"\n"));
    let shared = Snapshot {
        root: snap.root.clone(),
        design: design_of(&files),
        files,
    };
    for op in [
        set("lab/rw_18mNms", "mass", None, 0.6, "kg"),
        Op::CreateCatalogItem {
            id: "lab/new_rw".into(),
            kind: "reaction_wheel".into(),
            label: None,
            label_en: None,
            maturity: None,
            copy_from_part: None,
            dir: None,
        },
        // 共有カタログの出典（`lab/<ID>`）も読み取り専用
        Op::RemoveSource {
            id: "lab/ds_rw".into(),
        },
        Op::SetSource {
            source: fune_model::design::Source::new(
                "lab/ds_rw",
                fune_model::design::SourceKind::Datasheet,
            ),
        },
    ] {
        match plan(&shared, &[op]) {
            Err(OpError::Invalid { message }) => assert!(
                message.contains("shared catalog") && message.contains("PR"),
                "{message}"
            ),
            other => panic!("{other:?}"),
        }
    }
}
