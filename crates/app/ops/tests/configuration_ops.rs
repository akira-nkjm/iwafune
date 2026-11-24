//! 形態の Op（小さな設計に対して計画だけを立てる）: 足す・見出しを書く・消す・名前を変える・形態の中の配置とその取り消し・
//! 取付の付け替えの警告。どの Op のあとも、形態のファイルは正規形（`fune fmt` が何も変えない）。

use fune_model::value::{Quaternion, Vector3};
use fune_ops::{Op, OpError, Plan, plan};
use fune_store::read::design_of;
use fune_store::{FileSet, Snapshot, apply_changes};
use std::path::PathBuf;

/// R（根）← A（取付点 P を持つ）← B（A の P に取付）。C は R に取付。
fn files() -> FileSet {
    let mut f = FileSet::new();
    f.insert("fune.toml", "[project]\nname = \"t\"\n");
    f.insert("parts/R.toml", "kind = \"structure\"\n");
    f.insert(
        "parts/A.toml",
        "kind = \"structure\"\n\n[mount]\non = \"R\"\n\n[mount_points.P]\nposition = { x = 0.0, y = 0.2, z = 0.0, unit = \"m\" }\n",
    );
    f.insert(
        "parts/B.toml",
        "kind = \"structure\"\n\n[mount]\non = \"A\"\npoint = \"P\"\n\n[pose]\nposition = { x = 0.1, y = 0.0, z = 0.0, unit = \"m\" }\n",
    );
    f.insert(
        "parts/C.toml",
        "kind = \"structure\"\n\n[mount]\non = \"R\"\n",
    );
    f
}

fn snapshot_of(files: FileSet) -> Snapshot {
    let design = design_of(&files);
    Snapshot {
        root: PathBuf::new(),
        files,
        design,
    }
}

/// 手で書いた部品のファイルを正規形にしてから始める（Op のあとの正規形の確認が、元のファイルの揃い方に左右されないように）。
fn snapshot() -> Snapshot {
    let mut files = files();
    let changes = fune_store::format::format_files(&files);
    apply_changes(&mut files, &changes);
    snapshot_of(files)
}

/// 計画の適用後のファイル。
fn applied(snap: &Snapshot, p: &Plan) -> FileSet {
    let mut files = snap.files.clone();
    apply_changes(&mut files, &p.changes);
    files
}

fn text(files: &FileSet, path: &str) -> String {
    files
        .get(path)
        .unwrap_or_else(|| panic!("{path} missing"))
        .to_string()
}

/// Op の列を順に計画して適用し、そのたびにファイルが正規形のままであることを確かめる。
fn run(snap: Snapshot, ops: &[Op]) -> Snapshot {
    let mut snap = snap;
    for op in ops {
        let p = plan(&snap, std::slice::from_ref(op)).unwrap_or_else(|e| panic!("{op:?}: {e}"));
        let files = applied(&snap, &p);
        let not_canonical = fune_store::format::format_files(&files);
        assert!(
            not_canonical.is_empty(),
            "{op:?} left files not in canonical form: {:?}",
            not_canonical.iter().map(|c| &c.path).collect::<Vec<_>>()
        );
        snap = snapshot_of(files);
    }
    snap
}

fn mm(x: f64, y: f64, z: f64) -> Vector3 {
    Vector3 {
        x,
        y,
        z,
        unit: Some("mm".into()),
        frame: None,
    }
}

fn create(id: &str) -> Op {
    Op::CreateConfiguration {
        id: id.into(),
        label: Some("Stowed".into()),
        description: Some("Paddle folded".into()),
    }
}

fn place_in(
    part: &str,
    cfg: &str,
    position: Option<Vector3>,
    orientation: Option<Quaternion>,
) -> Op {
    Op::Place {
        part: part.into(),
        on: None,
        point: None,
        position,
        orientation,
        configuration: Some(cfg.into()),
    }
}

fn unset(cfg: &str, part: &str) -> Op {
    Op::UnsetConfigurationPose {
        configuration: cfg.into(),
        part: part.into(),
    }
}

#[test]
fn create_writes_the_file_once_and_refuses_a_duplicate() {
    let snap = snapshot();
    let p = plan(&snap, &[create("STOWED")]).unwrap();
    assert_eq!(
        text(&applied(&snap, &p), "configurations/STOWED.toml"),
        "label       = \"Stowed\"\ndescription = \"Paddle folded\"\n"
    );
    let snap = run(snap, &[create("STOWED")]);
    assert!(matches!(
        plan(&snap, &[create("STOWED")]),
        Err(OpError::AlreadyExists { .. })
    ));
    // 大文字小文字だけが違う ID も重ねない
    assert!(plan(&snap, &[create("stowed")]).is_err());
}

#[test]
fn set_replaces_the_heading_and_keeps_the_poses() {
    let snap = run(
        snapshot(),
        &[
            create("STOWED"),
            place_in("B", "STOWED", Some(mm(2.6, -1.5, 0.0)), None),
        ],
    );
    let before = text(&snap.files, "configurations/STOWED.toml");
    let set = Op::SetConfiguration {
        id: "STOWED".into(),
        label: Some("Folded".into()),
        description: None,
    };
    let snap = run(snap, std::slice::from_ref(&set));
    let after = text(&snap.files, "configurations/STOWED.toml");
    assert_eq!(
        after,
        before
            .replace("\"Stowed\"", "\"Folded\"")
            .replace("description = \"Paddle folded\"\n", "")
            // 欄を 1 つ消すと、残りの `=` の位置は label の長さに揃う
            .replace("label       =", "label =")
    );
    assert!(after.contains("[pose.B]"));
    // 無い形態への `set` は作る
    let snap = run(
        snap,
        &[Op::SetConfiguration {
            id: "NEW".into(),
            label: Some("New".into()),
            description: None,
        }],
    );
    assert_eq!(
        text(&snap.files, "configurations/NEW.toml"),
        "label = \"New\"\n"
    );
}

#[test]
fn place_in_a_configuration_writes_only_the_configuration_file() {
    let snap = run(snapshot(), &[create("STOWED")]);
    let op = place_in(
        "B",
        "STOWED",
        Some(mm(2.6, -1.5, 0.0)),
        Some(Quaternion::IDENTITY),
    );
    let p = plan(&snap, std::slice::from_ref(&op)).unwrap();
    let paths: Vec<&str> = p.changes.iter().map(|c| c.path.as_str()).collect();
    assert_eq!(
        paths,
        ["configurations/STOWED.toml"],
        "the part file is untouched"
    );
    let snap = run(snap, &[op]);
    assert_eq!(
        text(&snap.files, "configurations/STOWED.toml"),
        "label       = \"Stowed\"\ndescription = \"Paddle folded\"\n\n[pose.B]\nposition    = { x = 2.6, y = -1.5, z = 0.0, unit = \"mm\" }\norientation = { w = 1.0, x = 0.0, y = 0.0, z = 0.0 }\n"
    );
    // 部品自身の `[pose]` は基準のまま
    assert_eq!(
        snap.design.parts["B"]
            .pose
            .as_ref()
            .unwrap()
            .position
            .as_ref()
            .unwrap()
            .x,
        0.1
    );
    // 置き直しは `[pose.B]` を丸ごと置き換える（キーごとの合成はしない）
    let snap = run(
        snap,
        &[place_in("B", "STOWED", Some(mm(1.0, 0.0, 0.0)), None)],
    );
    let c = &snap.design.configurations["STOWED"];
    assert_eq!(c.pose["B"].position.as_ref().unwrap().x, 1.0);
    assert!(c.pose["B"].orientation.is_none());
}

#[test]
fn a_mount_given_in_a_configuration_must_be_the_base_mount() {
    let snap = run(snapshot(), &[create("STOWED")]);
    let with = |on: Option<&str>, point: Option<&str>| Op::Place {
        part: "B".into(),
        on: on.map(Into::into),
        point: point.map(Into::into),
        position: Some(mm(1.0, 0.0, 0.0)),
        orientation: None,
        configuration: Some("STOWED".into()),
    };
    // 同じ取付先・取付点なら書いてよい
    assert!(plan(&snap, &[with(Some("A"), Some("P"))]).is_ok());
    assert!(plan(&snap, &[with(Some("A"), None)]).is_ok());
    // 違えば拒む（形態ごとの付け替えはできない）
    for op in [
        with(Some("C"), None),
        with(Some("A"), Some("Q")),
        with(None, Some("Q")),
    ] {
        let err = plan(&snap, std::slice::from_ref(&op)).unwrap_err();
        assert!(
            matches!(&err, OpError::Invalid { .. }) && err.to_string().contains("not its mount"),
            "{op:?}: {err}"
        );
    }
}

#[test]
fn refusals_name_what_is_wrong() {
    let snap = run(snapshot(), &[create("STOWED")]);
    let msg = |op: Op| plan(&snap, &[op]).unwrap_err().to_string();
    // 取付の無い部品（根）
    assert!(msg(place_in("R", "STOWED", Some(mm(1.0, 0.0, 0.0)), None)).contains("has no mount"));
    // 無い形態・無い部品
    assert!(matches!(
        plan(
            &snap,
            &[place_in("B", "NOPE", Some(mm(1.0, 0.0, 0.0)), None)]
        ),
        Err(OpError::NotFound { .. })
    ));
    assert!(matches!(
        plan(
            &snap,
            &[place_in("NOPE", "STOWED", Some(mm(1.0, 0.0, 0.0)), None)]
        ),
        Err(OpError::NotFound { .. })
    ));
    // 位置も姿勢も無い
    assert!(msg(place_in("B", "STOWED", None, None)).contains("position or an orientation"));
    // 上書きの無い部品を消す・無い形態から消す
    assert!(matches!(
        plan(&snap, &[unset("STOWED", "B")]),
        Err(OpError::NotFound { .. })
    ));
    assert!(matches!(
        plan(&snap, &[unset("NOPE", "B")]),
        Err(OpError::NotFound { .. })
    ));
}

#[test]
fn unset_removes_the_table_and_keeps_the_heading() {
    let snap = run(
        snapshot(),
        &[
            create("STOWED"),
            place_in("B", "STOWED", Some(mm(2.6, -1.5, 0.0)), None),
            place_in("C", "STOWED", Some(mm(0.0, 3.0, 0.0)), None),
        ],
    );
    let both = text(&snap.files, "configurations/STOWED.toml");
    assert!(both.contains("[pose.B]") && both.contains("[pose.C]"));
    let snap = run(snap, &[unset("STOWED", "B")]);
    let one = text(&snap.files, "configurations/STOWED.toml");
    assert!(
        !one.contains("[pose.B]") && one.contains("[pose.C]"),
        "{one}"
    );
    let snap = run(snap, &[unset("STOWED", "C")]);
    assert_eq!(
        text(&snap.files, "configurations/STOWED.toml"),
        "label       = \"Stowed\"\ndescription = \"Paddle folded\"\n"
    );
    // もう上書きは無いので、2 回目は拒む（黙って成功にはしない）
    assert!(matches!(
        plan(&snap, &[unset("STOWED", "C")]),
        Err(OpError::NotFound { .. })
    ));
}

#[test]
fn rename_moves_the_file_and_changes_nothing_else() {
    let snap = run(
        snapshot(),
        &[
            create("STOWED"),
            place_in("B", "STOWED", Some(mm(2.6, -1.5, 0.0)), None),
        ],
    );
    let before = text(&snap.files, "configurations/STOWED.toml");
    let op = Op::RenameConfiguration {
        id: "STOWED".into(),
        to: "FOLDED".into(),
    };
    let p = plan(&snap, std::slice::from_ref(&op)).unwrap();
    let mut paths: Vec<&str> = p.changes.iter().map(|c| c.path.as_str()).collect();
    paths.sort_unstable();
    assert_eq!(
        paths,
        ["configurations/FOLDED.toml", "configurations/STOWED.toml"]
    );
    let snap = run(snap, &[op]);
    assert_eq!(text(&snap.files, "configurations/FOLDED.toml"), before);
    assert!(snap.files.get("configurations/STOWED.toml").is_none());
    assert!(snap.design.configurations.contains_key("FOLDED"));
    // 無い形態・重なる ID
    let again = Op::RenameConfiguration {
        id: "NOPE".into(),
        to: "X".into(),
    };
    assert!(matches!(
        plan(&snap, &[again]),
        Err(OpError::NotFound { .. })
    ));
    let snap = run(snap, &[create("OTHER")]);
    let clash = Op::RenameConfiguration {
        id: "FOLDED".into(),
        to: "other".into(),
    };
    assert!(matches!(
        plan(&snap, &[clash]),
        Err(OpError::AlreadyExists { .. })
    ));
}

#[test]
fn remove_deletes_the_file() {
    let snap = run(
        snapshot(),
        &[
            create("STOWED"),
            place_in("B", "STOWED", Some(mm(2.6, -1.5, 0.0)), None),
        ],
    );
    let op = Op::RemoveConfiguration {
        id: "STOWED".into(),
    };
    let p = plan(&snap, std::slice::from_ref(&op)).unwrap();
    assert_eq!(p.changes.len(), 1);
    assert!(p.changes[0].after.is_none());
    let snap = run(snap, std::slice::from_ref(&op));
    assert!(snap.design.configurations.is_empty());
    assert!(matches!(plan(&snap, &[op]), Err(OpError::NotFound { .. })));
}

/// 配置の検査（単位・四元数の大きさ）は部品の `[pose]` と同じで、誤りなら計画ごと拒む。
#[test]
fn a_bad_pose_in_a_configuration_is_rejected_like_a_part_pose() {
    let snap = run(snapshot(), &[create("STOWED")]);
    let bad_unit = Vector3 {
        unit: Some("kg".into()),
        ..mm(1.0, 0.0, 0.0)
    };
    assert!(matches!(
        plan(&snap, &[place_in("B", "STOWED", Some(bad_unit), None)]),
        Err(OpError::Rejected { .. })
    ));
    let big = Quaternion {
        w: 2.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    assert!(matches!(
        plan(&snap, &[place_in("B", "STOWED", None, Some(big))]),
        Err(OpError::Rejected { .. })
    ));
}

fn base_place(on: Option<&str>, point: Option<&str>, position: Option<Vector3>) -> Op {
    Op::Place {
        part: "B".into(),
        on: on.map(Into::into),
        point: point.map(Into::into),
        position,
        orientation: None,
        configuration: None,
    }
}

/// 基準の取付先・取付点を `Place` で変え、その部品に形態の上書きがあれば、上書きが古い取付点に対するものになるので警告する。
#[test]
fn changing_the_base_mount_warns_when_a_configuration_overrides_the_part() {
    let snap = run(
        snapshot(),
        &[
            create("STOWED"),
            place_in("B", "STOWED", Some(mm(2.6, -1.5, 0.0)), None),
        ],
    );
    let warnings = |p: &Plan| -> Vec<(String, Option<String>)> {
        p.diagnostics
            .iter()
            .filter(|d| d.code == "configuration.mount_changed")
            .map(|d| (d.message.clone(), d.path()))
            .collect()
    };
    // 取付先を変える
    let p = plan(
        &snap,
        &[base_place(Some("C"), None, Some(mm(0.0, 0.0, 0.0)))],
    )
    .unwrap();
    let w = warnings(&p);
    assert_eq!(w.len(), 1, "{w:?}");
    assert_eq!(w[0].1.as_deref(), Some("configurations.STOWED.pose.B"));
    assert!(w[0].0.contains("from `A.P` to `C`"), "{}", w[0].0);
    let located = p
        .diagnostics
        .iter()
        .find(|d| d.code == "configuration.mount_changed")
        .unwrap();
    assert_eq!(located.file.as_deref(), Some("configurations/STOWED.toml"));
    // 取付点だけを変える
    let p = plan(&snap, &[base_place(Some("A"), None, None)]).unwrap();
    assert_eq!(warnings(&p).len(), 1);
    // 取付を外す（根にする）
    let p = plan(&snap, &[base_place(None, None, None)]).unwrap();
    assert_eq!(warnings(&p).len(), 1);
    // 取付が同じ（姿勢だけを変える）なら警告しない
    let p = plan(
        &snap,
        &[base_place(Some("A"), Some("P"), Some(mm(5.0, 0.0, 0.0)))],
    )
    .unwrap();
    assert!(warnings(&p).is_empty());
    // 上書きの無い部品の付け替えも警告しない
    let c = Op::Place {
        part: "C".into(),
        on: Some("A".into()),
        point: Some("P".into()),
        position: None,
        orientation: None,
        configuration: None,
    };
    assert!(warnings(&plan(&snap, &[c]).unwrap()).is_empty());
}
