//! 編集の履歴（取り消し・やり直し・一覧）を service の関数から使う。

use fune_service::dto::{ApplyRequest, FormatDesignRequest, HistoryAction};
use fune_service::{ErrorKind, ServiceError, Workspace, edit, format, history};
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

fn project() -> (tempfile::TempDir, Workspace) {
    let dir = tempfile::tempdir().unwrap();
    let demo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    copy_dir(&demo, dir.path());
    let ws = Workspace::open(dir.path()).unwrap();
    (dir, ws)
}

fn set_mass(ws: &Workspace, grams: f64) {
    let op = serde_json::json!({ "op": "set_value", "part": "RW_Y", "attr": "mass",
        "value": { "type": "scalar", "data": { "value": grams, "unit": "g" } } });
    let req = ApplyRequest {
        ops: vec![serde_json::from_value(op).unwrap()],
        base_revision: None,
        dry_run: false,
    };
    edit::apply(ws, &req).unwrap();
}

#[test]
fn view_follows_the_position_in_the_history() {
    let (_dir, ws) = project();
    let empty = history::view(&ws).unwrap();
    assert!(!empty.can_undo && !empty.can_redo && empty.entries.is_empty());
    set_mass(&ws, 151.0);
    set_mass(&ws, 152.0);
    let h = history::view(&ws).unwrap();
    assert_eq!(h.entries.len(), 2);
    assert!(h.can_undo && !h.can_redo);
    assert_eq!(h.entries[0].files, ["parts/aocs/RW_Y.toml"]);
    let undone = history::undo(&ws).unwrap();
    assert_eq!(undone.action, HistoryAction::Undo);
    assert_eq!(undone.changed.len(), 1);
    assert!(undone.edit_cli[0].contains("RW_Y"));
    let h = history::view(&ws).unwrap();
    assert!(h.can_undo && h.can_redo);
    assert!(h.entries[1].undone && !h.entries[0].undone);
}

#[test]
fn empty_history_and_outside_edits_have_their_own_errors() {
    let (dir, ws) = project();
    let err = history::undo(&ws).unwrap_err();
    assert!(matches!(
        err,
        ServiceError::NothingToStep { action: "undo" }
    ));
    assert_eq!(err.kind(), ErrorKind::NotFound);
    set_mass(&ws, 151.0);
    let path = dir.path().join("fune.toml");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, format!("{text}\n# by hand\n")).unwrap();
    let err = history::undo(&ws).unwrap_err();
    assert!(matches!(err, ServiceError::HistoryConflict { .. }));
    assert_eq!(err.kind(), ErrorKind::Conflict);
    assert!(err.to_string().starts_with("Cannot undo:"));
}

/// `fune fmt` は Op ではないので履歴に積まない（積むのは Op の適用だけ）。
#[test]
fn fmt_is_not_recorded() {
    let (dir, ws) = project();
    let path = dir.path().join("fune.toml");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replace(" = ", "   =   ")).unwrap();
    let r = format::format_design(&ws, &FormatDesignRequest { check: false }).unwrap();
    assert!(!r.changed.is_empty());
    assert!(history::view(&ws).unwrap().entries.is_empty());
}
