//! グループ（系の下の入れ子のフォルダ）への部品の出し入れ（fixtures/demo-6u に対して。
//! デモでは反応ホイールが `aocs/RW`、`GPS`・`STT` は `aocs` の直下にある）。

use fune_model::design::NodeRef;
use fune_ops::{Op, OpError, plan};
use fune_store::{Snapshot, load};
use std::path::PathBuf;

fn snapshot() -> Snapshot {
    load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")).unwrap()
}

fn mv(id: &str, to: Option<&str>, under: &[&str]) -> Op {
    Op::MovePart {
        id: id.into(),
        to: to.map(Into::into),
        under: under.iter().map(|s| s.to_string()).collect(),
    }
}

/// 反応ホイール 3 つ。
const LOOSE: [&str; 2] = ["GPS", "STT"];

fn group_all() -> Vec<Op> {
    LOOSE
        .iter()
        .map(|id| mv(id, Some("NAV"), &["aocs"]))
        .collect()
}

#[test]
fn moving_into_a_new_group_creates_it_under_the_subsystem() {
    let snap = snapshot();
    let p = plan(&snap, &group_all()).unwrap();
    for id in LOOSE {
        let path = format!("parts/aocs/NAV/{id}.toml");
        assert!(
            p.changes
                .iter()
                .any(|c| c.path == path && c.before.is_none()),
            "{path}"
        );
        assert!(
            p.changes
                .iter()
                .any(|c| c.path == format!("parts/aocs/{id}.toml") && c.after.is_none())
        );
        let part = &p.design.parts[id];
        assert_eq!(part.parent, Some(NodeRef::Group("NAV".into())));
        let ancestors = p.design.ancestors(&NodeRef::Group("NAV".into()));
        assert_eq!(ancestors, vec![NodeRef::Group("aocs".into())]);
    }
    assert!(p.design.groups.contains_key("NAV"));
}

#[test]
fn moving_out_the_last_part_removes_the_group_and_its_directory() {
    let snap = snapshot();
    let mut ops = group_all();
    ops.extend(LOOSE.iter().map(|id| mv(id, Some("aocs"), &[])));
    let p = plan(&snap, &ops).unwrap();
    assert!(!p.design.groups.contains_key("NAV"));
    assert!(p.changes.iter().all(|c| !c.path.contains("/NAV/")));
    // 元の場所に戻る（ファイルの内容は変わらない）
    assert!(p.changes.iter().all(|c| c.before == c.after));
}

#[test]
fn moving_into_an_existing_group_by_path_or_by_id() {
    let snap = snapshot();
    // デモの `RW` はもう `aocs` の下にある（連鎖あり・無しのどちらでも入れられる）
    let ops = vec![mv("GPS", Some("RW"), &["aocs"]), mv("STT", Some("RW"), &[])];
    let p = plan(&snap, &ops).unwrap();
    for id in LOOSE {
        assert_eq!(p.design.parts[id].parent, Some(NodeRef::Group("RW".into())));
    }
    assert!(p.changes.iter().any(|c| c.path == "parts/aocs/RW/GPS.toml"));
}

#[test]
fn nested_group_chain_is_created_in_one_move() {
    let snap = snapshot();
    let p = plan(&snap, &[mv("GPS", Some("inner"), &["aocs", "RW"])]).unwrap();
    assert!(
        p.changes
            .iter()
            .any(|c| c.path == "parts/aocs/RW/inner/GPS.toml")
    );
    assert_eq!(
        p.design.ancestors(&NodeRef::Group("inner".into())),
        vec![NodeRef::Group("RW".into()), NodeRef::Group("aocs".into())]
    );
}

#[test]
fn create_part_into_a_new_group() {
    let snap = snapshot();
    let op = Op::CreatePart {
        id: "RW_S".into(),
        group: Some("RW".into()),
        group_under: vec!["aocs".into()],
        catalog: Some("local/rw_10mNms".into()),
        kind: None,
        count: None,
        label: None,
        mode_state: None,
    };
    let p = plan(&snap, &[op]).unwrap();
    assert!(
        p.changes
            .iter()
            .any(|c| c.path == "parts/aocs/RW/RW_S.toml")
    );
}

#[track_caller]
fn rejected(ops: &[Op]) -> OpError {
    match plan(&snapshot(), ops) {
        Err(e) => e,
        Ok(_) => panic!("should be rejected"),
    }
}

#[test]
fn invalid_targets_are_rejected_with_clear_errors() {
    // 連鎖の先頭が無い（最上位に新しいフォルダは作らせない）
    assert!(matches!(
        rejected(&[mv("GPS", Some("NAV"), &["nothing"])]),
        OpError::NotFound { .. }
    ));
    // 部品の下にグループは作れない
    let e = rejected(&[mv("GPS", Some("NAV"), &["BUS_FRAME"])]);
    assert!(matches!(e, OpError::Invalid { .. }), "{e}");
    // 既存の部品の ID と（大文字小文字を除いて）同じ名前のグループは作れない
    for name in ["STT", "stt"] {
        assert!(matches!(
            rejected(&[mv("GPS", Some(name), &["aocs", "new"])]),
            OpError::AlreadyExists { .. }
        ));
    }
    // 別の系の下にもう同じ名前のグループがある
    assert!(matches!(
        rejected(&[mv("SAP_PY", Some("RW"), &["power"])]),
        OpError::AlreadyExists { .. }
    ));
    // 連鎖に同じ ID が 2 回
    assert!(matches!(
        rejected(&[mv("GPS", Some("aocs"), &["aocs"])]),
        OpError::Invalid { .. }
    ));
    // 置き先が無いのに連鎖だけある
    assert!(matches!(
        rejected(&[mv("GPS", None, &["aocs"])]),
        OpError::Invalid { .. }
    ));
    // ID の規則に合わない名前
    assert!(matches!(
        rejected(&[mv("GPS", Some("a b"), &["aocs"])]),
        OpError::Invalid { .. }
    ));
}

#[test]
fn cannot_move_a_part_under_itself_through_a_chain() {
    let e = rejected(&[mv("BUS_FRAME", Some("g"), &["BUS_FRAME"])]);
    assert!(matches!(e, OpError::Invalid { .. }), "{e}");
}
