//! 出典のディレクトリ（`source set --subdir`・`source mv`）。fixtures/demo-6u に対して計画を立て、結果を次の計画の元にする。

use fune_model::design::{EntityKey, Source, SourceKind};
use fune_ops::{Op, OpError, Plan, plan, to_cli};
use fune_store::read::design_of;
use fune_store::write::apply_changes;
use fune_store::{Snapshot, format, load};
use std::path::PathBuf;

const ID: &str = "ds_rw10";

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

fn mv(id: &str, dir: &str) -> Op {
    Op::MoveSource {
        id: id.into(),
        dir: dir.into(),
    }
}

fn set(id: &str, dir: Option<&str>) -> Op {
    Op::SetSource {
        source: Source::new(id, SourceKind::Judgement),
        dir: dir.map(Into::into),
    }
}

fn file_of(snap: &Snapshot, id: &str) -> String {
    snap.design
        .source_map
        .file_of(&EntityKey::Source(id.into()))
        .unwrap()
        .to_string()
}

#[test]
fn set_a_new_source_with_a_subdir_writes_under_it() {
    let snap = snapshot();
    let p = plan(&snap, &[set("src_new", Some("notes/a"))]).unwrap();
    assert_eq!(paths(&p), [("sources/notes/a/src_new.toml", true)]);
    assert!(p.design.sources.contains_key("src_new"));
    assert_eq!(
        to_cli(&set("src_new", Some("notes/a"))),
        ["fune source set src_new --kind judgement --subdir notes/a"]
    );
    // 既定は今まで通り直下（`.` と同じ出力。コマンドにも `--subdir` は出ない）
    let flat = plan(&snap, &[set("src_new", None)]).unwrap();
    let root = plan(&snap, &[set("src_new", Some("."))]).unwrap();
    assert_eq!(paths(&flat), [("sources/src_new.toml", true)]);
    assert_eq!(flat.changes, root.changes);
    assert!(!to_cli(&set("src_new", None))[0].contains("--subdir"));
    for bad in ["../x", "1a", "a//b", ""] {
        assert!(matches!(
            plan(&snap, &[set("src_new", Some(bad))]),
            Err(OpError::Invalid { .. })
        ));
    }
}

#[test]
fn mv_moves_the_file_and_keeps_the_id() {
    let snap = snapshot();
    let to_dir = plan(&snap, &[mv(ID, "datasheets")]).unwrap();
    assert_eq!(
        paths(&to_dir),
        [
            ("sources/datasheets/ds_rw10.toml", true),
            ("sources/ds_rw10.toml", false)
        ]
    );
    // ID も、参照している品の出典も変わらない
    assert!(to_dir.design.sources.contains_key(ID));
    assert_eq!(
        to_cli(&mv(ID, "datasheets")),
        ["fune source mv ds_rw10 datasheets"]
    );
    let moved = after(&snap, &to_dir);
    assert_eq!(file_of(&moved, ID), "sources/datasheets/ds_rw10.toml");
    let deeper = after(&moved, &plan(&moved, &[mv(ID, "x/y")]).unwrap());
    assert_eq!(file_of(&deeper, ID), "sources/x/y/ds_rw10.toml");
    let back = plan(&deeper, &[mv(ID, ".")]).unwrap();
    let restored = after(&deeper, &back);
    assert_eq!(
        restored.files.get("sources/ds_rw10.toml"),
        snap.files.get("sources/ds_rw10.toml")
    );
    assert!(!restored.files.paths().any(|p| p.contains("/x/")));
    // 今いるディレクトリ・規則に合わない名前・無い出典は拒む
    assert!(matches!(
        plan(&snap, &[mv(ID, ".")]),
        Err(OpError::Invalid { .. })
    ));
    for bad in ["../x", ""] {
        assert!(matches!(
            plan(&snap, &[mv(ID, bad)]),
            Err(OpError::Invalid { .. })
        ));
    }
    assert!(matches!(
        plan(&snap, &[mv("none", "a")]),
        Err(OpError::NotFound { .. })
    ));
}

#[test]
fn edits_to_an_existing_source_write_in_place() {
    let snap = snapshot();
    let moved = after(&snap, &plan(&snap, &[mv(ID, "datasheets")]).unwrap());
    let file = "sources/datasheets/ds_rw10.toml";
    let mut edited = moved.design.sources[ID].clone();
    edited.note = Some("edited".into());
    let op = |dir: Option<&str>| Op::SetSource {
        source: edited.clone(),
        dir: dir.map(Into::into),
    };
    // 書き換えは今のファイルに（`--subdir` なし、または同じディレクトリ）
    for dir in [None, Some("datasheets")] {
        let p = plan(&moved, &[op(dir)]).unwrap();
        assert_eq!(paths(&p), [(file, true)]);
        assert!(format::format_files(&after(&moved, &p).files).is_empty());
    }
    // 今と違うディレクトリは拒み、`fune source mv` を案内する
    for dir in [Some("other"), Some(".")] {
        match plan(&moved, &[op(dir)]) {
            Err(OpError::Invalid { message }) => {
                assert!(message.contains("fune source mv"), "{message}");
            }
            other => panic!("{other:?}"),
        }
    }
    // 直下の出典に `.` を指定するのは今と同じ場所
    assert!(plan(&snap, &[op(Some("."))]).is_ok());
    // 消すときもそのファイルを消す（参照されていない出典）
    let added = after(
        &snap,
        &plan(&snap, &[set("src_tmp", Some("notes"))]).unwrap(),
    );
    let rm = Op::RemoveSource {
        id: "src_tmp".into(),
    };
    assert_eq!(
        paths(&plan(&added, &[rm]).unwrap()),
        [("sources/notes/src_tmp.toml", false)]
    );
}

#[test]
fn shared_catalog_sources_cannot_be_moved_or_set_with_a_dir() {
    let snap = snapshot();
    let mut files = snap.files.clone();
    let project = files.get("fune.toml").unwrap().to_string();
    files.insert("fune.toml", format!("{project}\n[[catalog]]\nname = \"lab\"\ngit  = \"https://git.example.com/example-lab/parts-catalog\"\nrev  = \"v2026.11\"\n"));
    let shared = Snapshot {
        root: snap.root.clone(),
        design: design_of(&files),
        files,
    };
    for op in [set("lab/ds_new", Some("a")), mv("lab/ds_rw", "a")] {
        match plan(&shared, &[op]) {
            Err(OpError::Invalid { message }) => {
                assert!(message.contains("shared catalog"), "{message}");
            }
            other => panic!("{other:?}"),
        }
    }
}
