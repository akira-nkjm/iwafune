//! カタログ品のディレクトリ（`add --subdir`・`mv`）。fixtures/demo-6u に対して計画を立て、結果を次の計画の元にする。

use fune_model::design::EntityKey;
use fune_model::value::{Scalar, Value};
use fune_ops::{Op, OpError, Plan, plan, to_cli};
use fune_store::read::design_of;
use fune_store::write::apply_changes;
use fune_store::{Snapshot, format, load};
use std::path::PathBuf;

const ITEM: &str = "local/rw_10mNms";

fn snapshot() -> Snapshot {
    load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")).unwrap()
}

/// 計画の変更を当てた次のスナップショット（ディスクには書かない）。
fn after(snap: &Snapshot, p: &Plan) -> Snapshot {
    let mut files = snap.files.clone();
    apply_changes(&mut files, &p.changes);
    Snapshot {
        root: snap.root.clone(),
        design: design_of(&files),
        files,
    }
}

fn paths(p: &Plan) -> Vec<(&str, bool)> {
    p.changes
        .iter()
        .map(|c| (c.path.as_str(), c.after.is_some()))
        .collect()
}

fn mv(dir: &str) -> Op {
    Op::MoveCatalogItem {
        item: ITEM.into(),
        dir: dir.into(),
    }
}

fn file_of(snap: &Snapshot, item: &str) -> String {
    snap.design
        .source_map
        .file_of(&EntityKey::Catalog(item.into()))
        .unwrap()
        .to_string()
}

#[test]
fn add_with_dir_writes_under_the_directory() {
    let snap = snapshot();
    let op = |dir: Option<&str>| Op::CreateCatalogItem {
        id: "rw_new".into(),
        kind: "reaction_wheel".into(),
        label: None,
        label_en: None,
        maturity: None,
        copy_from_part: None,
        dir: dir.map(Into::into),
    };
    let p = plan(&snap, &[op(Some("vendor_a/wheels"))]).unwrap();
    assert_eq!(paths(&p), [("catalog/vendor_a/wheels/rw_new.toml", true)]);
    // ID は変わらない（ディレクトリは参照に入らない）
    assert!(p.design.catalog.contains_key("local/rw_new"));
    assert_eq!(
        to_cli(&op(Some("vendor_a/wheels"))),
        ["fune catalog add rw_new --kind reaction_wheel --subdir vendor_a/wheels"]
    );
    // 既定は今まで通りカタログの直下（出力も同じ）
    let flat = plan(&snap, &[op(None)]).unwrap();
    let root = plan(&snap, &[op(Some("."))]).unwrap();
    assert_eq!(paths(&flat), [("catalog/rw_new.toml", true)]);
    assert_eq!(flat.changes, root.changes);
    assert!(!to_cli(&op(None))[0].contains("--dir"));
    // 名前の規則に従わないディレクトリ・空のディレクトリは拒む
    for bad in ["../x", "1a", "a//b", ""] {
        assert!(matches!(
            plan(&snap, &[op(Some(bad))]),
            Err(OpError::Invalid { .. })
        ));
    }
}

#[test]
fn mv_moves_the_file_and_keeps_the_id() {
    let snap = snapshot();
    let to_dir = plan(&snap, &[mv("vendor_a")]).unwrap();
    assert_eq!(
        paths(&to_dir),
        [
            ("catalog/rw_10mNms.toml", false),
            ("catalog/vendor_a/rw_10mNms.toml", true)
        ]
    );
    // 部品の参照も、解決した値も変わらない
    assert_eq!(
        fune_model::resolve(&to_dir.design).parts["RW_X"].scalar("mass"),
        fune_model::resolve(&snap.design).parts["RW_X"].scalar("mass")
    );
    assert_eq!(
        to_cli(&mv("vendor_a")),
        ["fune catalog mv local/rw_10mNms vendor_a"]
    );
    let moved = after(&snap, &to_dir);
    assert_eq!(file_of(&moved, ITEM), "catalog/vendor_a/rw_10mNms.toml");
    // さらに深いディレクトリへ、そして直下へ戻す
    let deeper = after(&moved, &plan(&moved, &[mv("x/y")]).unwrap());
    assert_eq!(file_of(&deeper, ITEM), "catalog/x/y/rw_10mNms.toml");
    let back = plan(&deeper, &[mv(".")]).unwrap();
    assert_eq!(
        paths(&back),
        [
            ("catalog/rw_10mNms.toml", true),
            ("catalog/x/y/rw_10mNms.toml", false)
        ]
    );
    // 移して戻すとファイルの中身は元のまま
    let restored = after(&deeper, &back);
    assert_eq!(
        restored.files.get("catalog/rw_10mNms.toml"),
        snap.files.get("catalog/rw_10mNms.toml")
    );
    // 今いるディレクトリへの移動・規則に合わない名前・無い品は拒む
    assert!(matches!(
        plan(&snap, &[mv(".")]),
        Err(OpError::Invalid { .. })
    ));
    assert!(matches!(
        plan(&snap, &[mv("../x")]),
        Err(OpError::Invalid { .. })
    ));
    assert!(matches!(
        plan(&snap, &[mv("")]),
        Err(OpError::Invalid { .. })
    ));
    let missing = Op::MoveCatalogItem {
        item: "local/none".into(),
        dir: "a".into(),
    };
    assert!(matches!(
        plan(&snap, &[missing]),
        Err(OpError::NotFound { .. })
    ));
}

#[test]
fn edits_inside_a_directory_write_in_place() {
    let snap = snapshot();
    let moved = after(&snap, &plan(&snap, &[mv("vendor_a")]).unwrap());
    let file = "catalog/vendor_a/rw_10mNms.toml";
    let set = Op::SetCatalogValue {
        item: ITEM.into(),
        attr: "max_torque".into(),
        state: None,
        value: Value::Scalar(Scalar::new(1.2, "mN*m")),
        from: None,
    };
    let p = plan(&moved, &[set]).unwrap();
    assert_eq!(paths(&p), [(file, true)]);
    let unset = Op::UnsetCatalogValue {
        item: ITEM.into(),
        attr: "mass".into(),
        state: None,
    };
    assert_eq!(paths(&plan(&moved, &[unset]).unwrap()), [(file, true)]);
    // 名前の変更は同じディレクトリの中（部品の参照も書き換わる）
    let rename = Op::RenameCatalogItem {
        from: ITEM.into(),
        to: "local/rw_12mNms".into(),
    };
    let p = plan(&moved, &[rename]).unwrap();
    assert!(paths(&p).contains(&(file, false)));
    assert!(paths(&p).contains(&("catalog/vendor_a/rw_12mNms.toml", true)));
    assert_eq!(
        file_of(&after(&moved, &p), "local/rw_12mNms"),
        "catalog/vendor_a/rw_12mNms.toml"
    );
    // 書き込んだファイルは正規形のまま
    assert!(format::format_files(&after(&moved, &p).files).is_empty());
    // 消すときもそのファイルを消す（参照されていない品）
    let add = Op::CreateCatalogItem {
        id: "rw_tmp".into(),
        kind: "reaction_wheel".into(),
        label: None,
        label_en: None,
        maturity: None,
        copy_from_part: None,
        dir: Some("vendor_b".into()),
    };
    let with_tmp = after(&snap, &plan(&snap, &[add]).unwrap());
    let rm = Op::RemoveCatalogItem {
        id: "local/rw_tmp".into(),
    };
    assert_eq!(
        paths(&plan(&with_tmp, &[rm]).unwrap()),
        [("catalog/vendor_b/rw_tmp.toml", false)]
    );
}

#[test]
fn shared_catalog_items_cannot_be_moved_or_added_with_a_dir() {
    let snap = snapshot();
    let mut files = snap.files.clone();
    let project = files.get("fune.toml").unwrap().to_string();
    files.insert("fune.toml", format!("{project}\n[[catalog]]\nname = \"lab\"\ngit  = \"https://git.example.com/example-lab/parts-catalog\"\nrev  = \"v2026.11\"\n"));
    let shared = Snapshot {
        root: snap.root.clone(),
        design: design_of(&files),
        files,
    };
    let add = Op::CreateCatalogItem {
        id: "lab/new_rw".into(),
        kind: "reaction_wheel".into(),
        label: None,
        label_en: None,
        maturity: None,
        copy_from_part: None,
        dir: Some("a".into()),
    };
    let move_shared = Op::MoveCatalogItem {
        item: "lab/rw_18mNms".into(),
        dir: "a".into(),
    };
    for op in [add, move_shared] {
        match plan(&shared, &[op]) {
            Err(OpError::Invalid { message }) => {
                assert!(message.contains("shared catalog"), "{message}");
            }
            other => panic!("{other:?}"),
        }
    }
}
