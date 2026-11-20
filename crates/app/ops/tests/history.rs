//! 取り消し・やり直し（fixtures/demo-6u の写しに対して適用する）。

use fune_model::value::{Scalar, Value};
use fune_ops::{ApplyOptions, Op, OpError, apply, redo, undo};
use fune_store::read::scan;
use fune_store::{FileSet, load};
use std::path::{Path, PathBuf};

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

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let demo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    copy_dir(&demo, dir.path());
    dir
}

fn files(root: &Path) -> FileSet {
    scan(root).unwrap()
}

fn go(root: &Path, op: Op) {
    apply(root, &[op], &ApplyOptions::default()).unwrap();
}

fn set_mass(part: &str, grams: f64) -> Op {
    Op::SetValue {
        part: part.into(),
        attr: "mass".into(),
        state: None,
        value: Value::Scalar(Scalar::new(grams, "g")),
        from: None,
    }
}

#[test]
fn undo_restores_bytes_and_redo_reapplies() {
    let dir = project();
    let root = dir.path();
    let original = files(root);
    let rev0 = load(root).unwrap().design.revision;
    go(root, set_mass("RW_Y", 151.0));
    let edited = files(root);
    let rev1 = load(root).unwrap().design.revision;
    assert_ne!(original, edited);

    let out = undo(root).unwrap();
    assert_eq!(files(root), original);
    assert_eq!(out.revision, rev0);
    assert_eq!(out.previous_revision, rev1);
    assert!(out.cli.iter().any(|c| c.contains("RW_Y")), "{:?}", out.cli);
    assert_eq!(load(root).unwrap().design.revision, rev0);

    let out = redo(root).unwrap();
    assert_eq!(files(root), edited);
    assert_eq!(out.revision, rev1);
}

#[test]
fn two_edits_undo_in_order() {
    let dir = project();
    let root = dir.path();
    let original = files(root);
    go(root, set_mass("RW_Y", 151.0));
    let after_one = files(root);
    go(root, set_mass("RW_Y", 152.0));
    undo(root).unwrap();
    assert_eq!(files(root), after_one);
    undo(root).unwrap();
    assert_eq!(files(root), original);
    assert!(matches!(
        undo(root),
        Err(OpError::NothingToStep { redo: false })
    ));
}

#[test]
fn new_edit_clears_redo() {
    let dir = project();
    let root = dir.path();
    go(root, set_mass("RW_Y", 151.0));
    undo(root).unwrap();
    go(root, set_mass("RW_Y", 153.0));
    assert!(matches!(
        redo(root),
        Err(OpError::NothingToStep { redo: true })
    ));
}

#[test]
fn dry_run_and_rejected_edits_leave_no_history() {
    let dir = project();
    let root = dir.path();
    let dry = ApplyOptions {
        dry_run: true,
        ..ApplyOptions::default()
    };
    apply(root, &[set_mass("RW_Y", 151.0)], &dry).unwrap();
    assert!(matches!(
        undo(root),
        Err(OpError::NothingToStep { redo: false })
    ));
}

#[test]
fn undo_is_refused_when_files_changed_elsewhere() {
    let dir = project();
    let root = dir.path();
    go(root, set_mass("RW_Y", 151.0));
    let path = root.join("fune.toml");
    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str("\n# edited by hand\n");
    std::fs::write(&path, &text).unwrap();
    let err = undo(root).unwrap_err();
    assert!(matches!(err, OpError::HistoryMismatch { redo: false, .. }));
    assert!(err.to_string().contains("changed outside the edit history"));
    // 何も書かない
    assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
}

/// 名前の変更（ファイルの移動と参照の書き換え）・部品の削除（ファイルの削除）・部品の作成（ファイルの新規）も戻る。
#[test]
fn undo_handles_renamed_deleted_and_created_files() {
    let dir = project();
    let root = dir.path();
    let original = files(root);
    go(
        root,
        Op::RenamePart {
            id: "PCDU".into(),
            to: "EPS_PCDU".into(),
        },
    );
    let renamed = files(root);
    assert!(renamed.contains("parts/power/EPS_PCDU.toml"));
    undo(root).unwrap();
    assert_eq!(files(root), original);
    redo(root).unwrap();
    assert_eq!(files(root), renamed);
    undo(root).unwrap();

    // 作成（新規ファイル）→ 削除（ファイルの削除）
    go(
        root,
        Op::CreatePart {
            id: "RW_S".into(),
            group: Some("aocs".into()),
            group_under: vec![],
            catalog: Some("local/rw_10mNms".into()),
            kind: None,
            count: None,
            label: None,
            mode_state: None,
        },
    );
    let created = files(root);
    assert!(created.paths().any(|p| p.ends_with("RW_S.toml")));
    go(root, Op::RemovePart { id: "RW_S".into() });
    assert!(!files(root).paths().any(|p| p.ends_with("RW_S.toml")));
    undo(root).unwrap();
    assert_eq!(files(root), created);
    undo(root).unwrap();
    assert_eq!(files(root), original);
    redo(root).unwrap();
    redo(root).unwrap();
    assert!(!files(root).paths().any(|p| p.ends_with("RW_S.toml")));
}

#[test]
fn history_is_bounded() {
    let dir = project();
    let root = dir.path();
    for n in 0..fune_store::journal::MAX_ENTRIES + 3 {
        go(root, set_mass("RW_Y", 100.0 + n as f64));
    }
    let mut undone = 0;
    while undo(root).is_ok() {
        undone += 1;
    }
    assert_eq!(undone, fune_store::journal::MAX_ENTRIES);
}
