//! カタログの出所の Op（`AddCatalogSource`・`RemoveCatalogSource`。fixtures/demo-6u に対して計画を立てる）。

use fune_ops::{Op, OpError, plan, to_cli};
use fune_store::{FileSet, Snapshot, apply_changes, format::format_text, load, read::design_of};
use std::path::PathBuf;

fn snapshot() -> Snapshot {
    load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")).unwrap()
}

fn with_files(snap: &Snapshot, files: FileSet) -> Snapshot {
    Snapshot {
        root: snap.root.clone(),
        design: design_of(&files),
        files,
    }
}

/// 計画の変更を当てた後のスナップショット（次の計画の元にする）。
fn applied(snap: &Snapshot, ops: &[Op]) -> Snapshot {
    let p = plan(snap, ops).unwrap();
    let mut files = snap.files.clone();
    apply_changes(&mut files, &p.changes);
    with_files(snap, files)
}

fn add(name: &str, git: &str, rev: Option<&str>) -> Op {
    Op::AddCatalogSource {
        name: name.into(),
        git: git.into(),
        rev: rev.map(str::to_owned),
    }
}

fn rm(name: &str) -> Op {
    Op::RemoveCatalogSource { name: name.into() }
}

const GIT: &str = "https://example.com/catalog.git";

#[test]
fn add_appends_a_shared_catalog_in_canonical_form() {
    let snap = snapshot();
    let op = add("lab", GIT, Some("main"));
    // 足した直後は lock に記録が無い（catalog.not_locked）が、取得で直る状態なので編集は拒まない
    let p = plan(&snap, std::slice::from_ref(&op)).unwrap();
    assert_eq!(p.changes.len(), 1);
    let change = &p.changes[0];
    assert_eq!(change.path, "fune.toml");
    let after = change.after.as_deref().unwrap();
    assert_eq!(format_text("fune.toml", after).unwrap(), after, "正規形");
    let before = change.before.as_deref().unwrap();
    let added = "[[catalog]]\nname = \"lab\"\ngit  = \"https://example.com/catalog.git\"\nrev  = \"main\"\n";
    // 今の [[catalog]] の後ろ（[modes] より前）に入り、ほかの行（先頭のコメントも）はそのまま
    let local = "[[catalog]]\nname = \"local\"\npath = \"catalog\"\n";
    assert!(
        after.contains(&format!("{local}\n{added}\n[modes]")),
        "{after}"
    );
    assert_eq!(after.replace(&format!("{added}\n"), ""), before);
    assert!(
        p.diagnostics
            .iter()
            .any(|d| d.code == "catalog.not_locked" && d.message.contains("`lab`"))
    );
    assert_eq!(
        to_cli(&op),
        ["fune catalog source add lab --git https://example.com/catalog.git --rev main"]
    );
    let no_rev = plan(&snap, &[add("team", GIT, None)]).unwrap();
    let text = no_rev.changes[0].after.as_deref().unwrap();
    assert!(!text.contains("rev"), "{text}");
    assert_eq!(
        to_cli(&add("team", GIT, None)),
        ["fune catalog source add team --git https://example.com/catalog.git"]
    );
}

#[test]
fn add_refuses_bad_names_duplicates_and_empty_urls() {
    let snap = applied(&snapshot(), &[add("lab", GIT, None)]);
    // 既にある名前（このプロジェクトのカタログ local も・大文字小文字の違いだけのものも）
    for name in ["local", "lab", "LAB"] {
        match plan(&snap, &[add(name, GIT, None)]) {
            Err(OpError::AlreadyExists { what }) => assert!(what.contains("カタログ"), "{what}"),
            other => panic!(
                "{name}: AlreadyExists のはずが {:?}",
                other.map(|p| p.changes)
            ),
        }
    }
    for (name, git, rev, needle) in [
        ("a/b", GIT, None, "名前"),
        ("", GIT, None, "名前"),
        ("team", "  ", None, "git"),
        ("team", GIT, Some(""), "rev"),
    ] {
        match plan(&snap, &[add(name, git, rev)]) {
            Err(OpError::Invalid { message }) => assert!(message.contains(needle), "{message}"),
            other => panic!("{name}: Invalid のはずが {:?}", other.map(|p| p.changes)),
        }
    }
}

#[test]
fn remove_restores_the_file_and_refuses_local_and_unknown_catalogs() {
    let base = snapshot();
    let snap = applied(&base, &[add("lab", GIT, Some("v1"))]);
    let p = plan(&snap, &[rm("lab")]).unwrap();
    assert_eq!(p.changes.len(), 1);
    assert_eq!(
        p.changes[0].after.as_deref(),
        base.files.get("fune.toml"),
        "足して外すと元のバイト列に戻る"
    );
    assert_eq!(to_cli(&rm("lab")), ["fune catalog source rm lab"]);
    match plan(&snap, &[rm("local")]) {
        Err(OpError::Invalid { message }) => assert!(message.contains("path"), "{message}"),
        other => panic!("Invalid のはずが {:?}", other.map(|p| p.changes)),
    }
    assert!(matches!(
        plan(&snap, &[rm("nope")]),
        Err(OpError::NotFound { .. })
    ));
}

#[test]
fn remove_refuses_while_parts_or_harnesses_use_its_items() {
    let snap = applied(&snapshot(), &[add("lab", GIT, None)]);
    let mut files = snap.files.clone();
    let replace = |files: &mut FileSet, path: &str, from: &str, to: &str| {
        let text = files.get(path).unwrap().replace(from, to);
        files.insert(path, text);
    };
    replace(
        &mut files,
        "parts/aocs/STT.toml",
        "\"local/stt_nano\"",
        "\"lab/stt_nano\"",
    );
    replace(
        &mut files,
        "harness/H_OBC_CAM.toml",
        "\"local/awg24_ptfe\"",
        "\"lab/awg24_ptfe\"",
    );
    let snap = with_files(&snap, files);
    match plan(&snap, &[rm("lab")]) {
        Err(OpError::Invalid { message }) => {
            assert!(message.contains("部品 STT"), "{message}");
            assert!(message.contains("ハーネス H_OBC_CAM"), "{message}");
        }
        other => panic!("Invalid のはずが {:?}", other.map(|p| p.changes)),
    }
}
