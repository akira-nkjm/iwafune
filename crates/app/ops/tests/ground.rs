//! 地上局の Op（足す・消す・名前を変える）と、定義（属性・種別）の名前の変更。fixtures/demo-6u に対して計画だけを立てる。
//! 取り消し・やり直しは実際に書いて確かめる（履歴は Op の単位）。

use fune_model::value::Quantity;
use fune_model::{Design, check};
use fune_ops::{ApplyOptions, Op, OpError, apply, plan, redo, undo};
use fune_store::read::scan;
use fune_store::{Snapshot, load};
use std::path::{Path, PathBuf};

fn demo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")
}

fn snapshot() -> Snapshot {
    load(&demo()).unwrap()
}

fn create(id: &str) -> Op {
    Op::CreateGroundStation {
        id: id.into(),
        label: Some("Second station (example)".into()),
        latitude: Some(Quantity::new(78.2, "deg")),
        longitude: Some(Quantity::new(15.4, "deg")),
        altitude: Some(Quantity::new(450.0, "m")),
        min_elevation: Some(Quantity::new(5.0, "deg")),
    }
}

fn no_errors(d: &Design) {
    assert!(check(d).iter().all(|x| !x.is_error()), "{:?}", check(d));
}

#[test]
fn create_writes_a_canonical_file() {
    let p = plan(&snapshot(), &[create("SECOND_GS")]).unwrap();
    assert_eq!(p.changes.len(), 1);
    assert_eq!(p.changes[0].path, "conditions/ground/SECOND_GS.toml");
    let text = p.changes[0].after.as_deref().unwrap();
    assert!(
        text.starts_with("label         = \"Second station (example)\"\n"),
        "{text}"
    );
    assert!(
        text.contains("min_elevation = { value = 5.0, unit = \"deg\" }\n"),
        "{text}"
    );
    let g = &p.design.conditions.ground["SECOND_GS"];
    assert_eq!(g.latitude, Some(Quantity::new(78.2, "deg")));
    no_errors(&p.design);
}

#[test]
fn create_refuses_duplicates_bad_ids_and_wrong_dimensions() {
    let snap = snapshot();
    // 大文字小文字だけが違う ID も重なる
    for id in ["TOKYO_GS", "tokyo_gs"] {
        assert!(
            matches!(
                plan(&snap, &[create(id)]),
                Err(OpError::AlreadyExists { .. })
            ),
            "{id}"
        );
    }
    assert!(plan(&snap, &[create("bad id")]).is_err());
    let wrong = Op::CreateGroundStation {
        id: "X_GS".into(),
        label: None,
        latitude: Some(Quantity::new(10.0, "kg")),
        longitude: None,
        altitude: None,
        min_elevation: None,
    };
    assert!(matches!(
        plan(&snap, &[wrong]),
        Err(OpError::Rejected { .. })
    ));
}

#[test]
fn remove_works_unless_the_analysis_lists_the_station() {
    let snap = snapshot();
    // 解析が挙げている局は消せない
    let err = plan(
        &snap,
        &[Op::RemoveGroundStation {
            id: "TOKYO_GS".into(),
        }],
    )
    .unwrap_err();
    assert!(err.to_string().contains("analysis.comm.stations"), "{err}");
    // 挙げていない局は消せる
    let p = plan(
        &snap,
        &[
            create("SECOND_GS"),
            Op::RemoveGroundStation {
                id: "SECOND_GS".into(),
            },
        ],
    )
    .unwrap();
    assert!(
        p.changes.is_empty(),
        "create then remove leaves no change: {:?}",
        p.changes
    );
    assert!(matches!(
        plan(&snap, &[Op::RemoveGroundStation { id: "NOPE".into() }]),
        Err(OpError::NotFound { .. })
    ));
}

#[test]
fn rename_rewrites_the_station_list() {
    let p = plan(
        &snapshot(),
        &[Op::RenameGroundStation {
            id: "TOKYO_GS".into(),
            to: "TOKYO_2".into(),
        }],
    )
    .unwrap();
    let paths: Vec<&str> = p.changes.iter().map(|c| c.path.as_str()).collect();
    assert!(
        paths.contains(&"conditions/ground/TOKYO_2.toml"),
        "{paths:?}"
    );
    assert!(paths.contains(&"analysis/comm.toml"), "{paths:?}");
    let d = &p.design;
    assert_eq!(d.analysis.comm.as_ref().unwrap().stations, ["TOKYO_2"]);
    assert!(d.conditions.ground.contains_key("TOKYO_2"));
    assert!(!d.conditions.ground.contains_key("TOKYO_GS"));
    no_errors(d);
    // 既にある名前・同じ名前（大文字小文字違い）へは変えられない
    let clash = |to: &str| {
        plan(
            &snapshot(),
            &[
                create("SECOND_GS"),
                Op::RenameGroundStation {
                    id: "TOKYO_GS".into(),
                    to: to.into(),
                },
            ],
        )
    };
    assert!(matches!(
        clash("second_gs"),
        Err(OpError::AlreadyExists { .. })
    ));
}

#[test]
fn rename_attribute_rewrites_kinds_catalog_items_and_parts() {
    let p = plan(
        &snapshot(),
        &[Op::RenameAttribute {
            id: "linear_density".into(),
            to: "mass_per_length".into(),
        }],
    )
    .unwrap();
    let paths: Vec<&str> = p.changes.iter().map(|c| c.path.as_str()).collect();
    for expected in [
        "defs/attribute/linear_density.toml",
        "defs/attribute/mass_per_length.toml",
        "defs/kind/wire.toml",
        "catalog/awg24_ptfe.toml",
    ] {
        assert!(paths.contains(&expected), "{expected} not in {paths:?}");
    }
    let d = &p.design;
    assert!(d.attributes.contains_key("mass_per_length"));
    assert!(
        d.kinds["wire"]
            .required
            .iter()
            .any(|a| a.as_str() == "mass_per_length")
    );
    assert!(
        d.catalog["local/awg24_ptfe"]
            .attr
            .contains_key("mass_per_length")
    );
    no_errors(d);
}

#[test]
fn rename_kind_rewrites_catalog_items_and_extends() {
    let snap = snapshot();
    let ops = [
        Op::CreateKind {
            def: serde_json::from_value(serde_json::json!({
                "id": "thick_wire", "label": "Thick wire", "extends": "wire"
            }))
            .unwrap(),
        },
        Op::RenameKind {
            id: "wire".into(),
            to: "cable".into(),
        },
    ];
    let p = plan(&snap, &ops).unwrap();
    let d = &p.design;
    assert_eq!(d.catalog["local/awg24_ptfe"].kind.as_str(), "cable");
    assert_eq!(
        d.kinds["thick_wire"].extends.as_ref().map(|e| e.as_str()),
        Some("cable")
    );
    assert!(d.kinds.contains_key("cable") && !d.kinds.contains_key("wire"));
    no_errors(d);
}

#[test]
fn rename_definitions_refuses_builtins_clashes_and_bad_names() {
    let snap = snapshot();
    let attr = |id: &str, to: &str| Op::RenameAttribute {
        id: id.into(),
        to: to.into(),
    };
    let kind = |id: &str, to: &str| Op::RenameKind {
        id: id.into(),
        to: to.into(),
    };
    for (op, needle) in [
        (attr("mass", "weight"), "built-in attribute"),
        (kind("component", "part_kind"), "built-in kind"),
        (attr("linear_density", "mass"), "already exists"),
        (
            attr("linear_density", "resistance_per_length"),
            "already exists",
        ),
        (kind("wire", "heater"), "already exists"),
        (attr("nope", "x"), "not found"),
    ] {
        let err = plan(&snap, &[op]).unwrap_err().to_string();
        assert!(err.to_lowercase().contains(needle), "{needle}: {err}");
    }
    // 名前の規則（フィールド名との重なりは検査が拒む）
    assert!(plan(&snap, &[attr("linear_density", "bad name")]).is_err());
    assert!(plan(&snap, &[attr("linear_density", "label")]).is_err());
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

/// 足す・名前を変える・定義の名前を変える、をどれも取り消してやり直せる（元のバイト列に戻る）。
#[test]
fn undo_and_redo_restore_every_new_op() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    copy_dir(&demo(), root);
    let original = scan(root).unwrap();
    let ops = [
        create("SECOND_GS"),
        Op::RenameGroundStation {
            id: "SECOND_GS".into(),
            to: "SECOND_2".into(),
        },
        Op::RenameGroundStation {
            id: "TOKYO_GS".into(),
            to: "TOKYO_2".into(),
        },
        Op::RenameAttribute {
            id: "linear_density".into(),
            to: "mass_per_length".into(),
        },
        Op::RenameKind {
            id: "wire".into(),
            to: "cable".into(),
        },
        Op::RemoveGroundStation {
            id: "SECOND_2".into(),
        },
    ];
    let mut states = vec![original.clone()];
    for op in &ops {
        apply(root, std::slice::from_ref(op), &ApplyOptions::default()).unwrap();
        states.push(scan(root).unwrap());
    }
    for i in (0..ops.len()).rev() {
        undo(root).unwrap();
        assert_eq!(scan(root).unwrap(), states[i], "undo of op {i}");
    }
    for i in 0..ops.len() {
        redo(root).unwrap();
        assert_eq!(scan(root).unwrap(), states[i + 1], "redo of op {i}");
    }
}

/// 共有カタログ（読み取り専用）の品が使っている定義は、書き換えられないので名前を変えられない。
#[test]
fn rename_is_refused_while_a_shared_catalog_item_uses_the_definition() {
    use fune_model::design::{CatalogLock, LockedCatalog};
    use fune_store::FileSet;
    use fune_store::read::design_of;
    use fune_store::write::project::catalog_lock_text;
    let mut set = FileSet::new();
    set.insert(
        "fune.toml",
        "[project]\nname = \"t\"\nschema = 2\n\n[[catalog]]\nname = \"lab\"\ngit = \"https://example.com/c.git\"\nrev = \"v1\"\n\n[[catalog]]\nname = \"local\"\npath = \"catalog\"\n",
    );
    set.insert(
        "defs/kind/wire.toml",
        "label = \"Wire\"\nrequired = [\"gauge\"]\n",
    );
    set.insert(
        "defs/attribute/gauge.toml",
        "label = \"Gauge\"\ntype = \"scalar\"\nunits = [\"mm\"]\n",
    );
    let lock = CatalogLock {
        catalogs: vec![LockedCatalog {
            name: "lab".into(),
            git: "https://example.com/c.git".into(),
            rev: Some("v1".into()),
            commit: "0123456789abcdef0123456789abcdef01234567".into(),
        }],
    };
    set.insert("catalog.lock", catalog_lock_text(&lock).unwrap());
    let mut shared = FileSet::new();
    shared.insert(
        "catalog/w.toml",
        "kind = \"wire\"\n\n[attr.gauge]\nvalue = 1.0\nunit  = \"mm\"\n",
    );
    set.insert_shared("lab", shared);
    let snap = Snapshot {
        root: PathBuf::new(),
        design: design_of(&set),
        files: set,
    };
    assert!(
        snap.design.catalog.contains_key("lab/w"),
        "{:?} {:?}",
        snap.design.catalog.keys().collect::<Vec<_>>(),
        snap.design.load_diagnostics
    );
    for op in [
        Op::RenameKind {
            id: "wire".into(),
            to: "cable".into(),
        },
        Op::RenameAttribute {
            id: "gauge".into(),
            to: "diameter".into(),
        },
    ] {
        let err = plan(&snap, &[op]).unwrap_err().to_string();
        assert!(err.contains("shared catalog items (lab/w)"), "{err}");
    }
}
