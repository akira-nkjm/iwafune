//! `fune.toml` の `[project]` の Op（`SetProjectField`）と、このプロジェクトの中のカタログ（`AddLocalCatalog`）。
//! fixtures/demo-6u に対して計画を立てる。

use fune_model::design::ProjectField;
use fune_ops::{Op, OpError, plan, to_cli};
use fune_store::format::format_text;
use fune_store::{FileSet, Snapshot, apply_changes, load};
use std::path::PathBuf;

fn snapshot() -> Snapshot {
    load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")).unwrap()
}

fn set(field: ProjectField) -> Op {
    Op::SetProjectField { field }
}

fn project_text(snap: &Snapshot, ops: &[Op]) -> String {
    let p = plan(snap, ops).unwrap();
    let mut files: FileSet = snap.files.clone();
    apply_changes(&mut files, &p.changes);
    files.get("fune.toml").unwrap().to_owned()
}

#[test]
fn set_name_and_label_keep_comments_and_key_order() {
    let snap = snapshot();
    let before = snap.files.get("fune.toml").unwrap().to_owned();
    let after = project_text(
        &snap,
        &[
            set(ProjectField::Name("renamed-demo".into())),
            set(ProjectField::Label(Some("Renamed demo".into()))),
        ],
    );
    assert_eq!(
        after,
        before
            .replace("\"iwafune-6u-demo\"", "\"renamed-demo\"")
            .replace("\"IWAFUNE 6U (demo)\"", "\"Renamed demo\"")
    );
    assert_eq!(format_text("fune.toml", &after).unwrap(), after);
    let p = plan(&snap, &[set(ProjectField::Name("renamed-demo".into()))]).unwrap();
    assert_eq!(p.design.project.meta.name, "renamed-demo");
}

#[test]
fn unset_label_removes_the_line_and_set_label_adds_it_back_in_place() {
    let snap = snapshot();
    let before = snap.files.get("fune.toml").unwrap().to_owned();
    let without = project_text(&snap, &[set(ProjectField::Label(None))]);
    assert!(!without.contains("label"), "{without}");
    assert!(without.contains("# IWAFUNE 6U (demo)"), "the comment stays");
    assert_eq!(format_text("fune.toml", &without).unwrap(), without);
    // 外してから足すと、元の位置（`name` の次）に戻る
    let mut files: FileSet = snap.files.clone();
    files.insert("fune.toml", without);
    let bare = Snapshot {
        root: snap.root.clone(),
        design: fune_store::read::design_of(&files),
        files,
    };
    let back = project_text(
        &bare,
        &[set(ProjectField::Label(Some("IWAFUNE 6U (demo)".into())))],
    );
    assert_eq!(back, before);
}

#[test]
fn set_refuses_empty_and_multi_line_text() {
    let snap = snapshot();
    for field in [
        ProjectField::Name("  ".into()),
        ProjectField::Name("a\nb".into()),
        ProjectField::Label(Some(String::new())),
        ProjectField::Label(Some("x\ty\u{7}".into())),
    ] {
        assert!(
            matches!(
                plan(&snap, &[set(field.clone())]),
                Err(OpError::Invalid { .. })
            ),
            "{field:?}"
        );
    }
    assert_eq!(
        to_cli(&set(ProjectField::Name("a b".into()))),
        ["fune project set name \"a b\""]
    );
    assert_eq!(
        to_cli(&set(ProjectField::Label(None))),
        ["fune project unset label"]
    );
}

fn add_local(name: &str, path: &str) -> Op {
    Op::AddLocalCatalog {
        name: name.into(),
        path: path.into(),
    }
}

#[test]
fn add_local_catalog_appends_after_the_existing_entries() {
    let snap = snapshot();
    let before = snap.files.get("fune.toml").unwrap().to_owned();
    let after = project_text(&snap, &[add_local("vendor", "vendor/parts")]);
    let added = "[[catalog]]\nname = \"vendor\"\npath = \"vendor/parts\"\n";
    let local = "[[catalog]]\nname = \"local\"\npath = \"catalog\"\n";
    assert!(
        after.contains(&format!("{local}\n{added}\n[modes]")),
        "{after}"
    );
    assert_eq!(after.replace(&format!("{added}\n"), ""), before);
    assert_eq!(format_text("fune.toml", &after).unwrap(), after);
    let p = plan(&snap, &[add_local("vendor", "vendor/parts")]).unwrap();
    let c = p
        .design
        .project
        .catalogs
        .iter()
        .find(|c| c.name == "vendor")
        .unwrap();
    assert_eq!(c.local_dir(), Some("vendor/parts"));
    assert_eq!(
        to_cli(&add_local("vendor", "vendor/parts")),
        ["fune catalog source add-local vendor --path vendor/parts"]
    );
}

/// 追加したカタログに品を足せて、品があるあいだは外せない。品を消せば外せて、元のバイト列に戻る。
#[test]
fn a_local_catalog_takes_items_and_is_removed_only_when_empty() {
    let snap = snapshot();
    let item = Op::CreateCatalogItem {
        id: "vendor/x_one".into(),
        kind: "wire".into(),
        label: None,
        label_en: None,
        maturity: None,
        copy_from_part: None,
        dir: None,
    };
    let with_item = plan(&snap, &[add_local("vendor", "vendor_parts"), item.clone()]).unwrap();
    assert!(
        with_item
            .changes
            .iter()
            .any(|c| c.path == "vendor_parts/x_one.toml"),
        "{:?}",
        with_item.changes
    );
    let rm = Op::RemoveCatalogSource {
        name: "vendor".into(),
    };
    match plan(
        &snap,
        &[add_local("vendor", "vendor_parts"), item, rm.clone()],
    ) {
        Err(OpError::Invalid { message }) => {
            assert!(message.contains("1 item"), "{message}")
        }
        other => panic!("expected Invalid, got {:?}", other.map(|p| p.changes)),
    }
    let p = plan(&snap, &[add_local("vendor", "vendor_parts"), rm]).unwrap();
    assert!(
        p.changes.is_empty(),
        "add then remove leaves no change: {:?}",
        p.changes
    );
}

#[test]
fn add_local_catalog_refuses_bad_names_paths_and_overlaps() {
    let snap = snapshot();
    for (name, path, needle) in [
        ("vendor", "", "directory"),
        ("vendor", ".", "root"),
        ("vendor", "parts", "layout"),
        ("vendor", "parts/sub", "layout"),
        ("vendor", "sources", "layout"),
        ("vendor", "../outside", "directory"),
        ("vendor", "/abs", "directory"),
        ("vendor", "a b", "directory"),
        ("vendor", "catalog", "overlaps"),
        ("vendor", "catalog/sub", "overlaps"),
        ("bad name", "vendor_parts", "catalog name"),
    ] {
        match plan(&snap, &[add_local(name, path)]) {
            Err(OpError::Invalid { message }) => {
                assert!(message.contains(needle), "{path:?}: {message}")
            }
            other => panic!(
                "{path:?}: expected Invalid, got {:?}",
                other.map(|p| p.changes)
            ),
        }
    }
    for name in ["local", "LOCAL"] {
        assert!(matches!(
            plan(&snap, &[add_local(name, "vendor_parts")]),
            Err(OpError::AlreadyExists { .. })
        ));
    }
}
