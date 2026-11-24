//! 設定の一覧の欄の表（`fune_model::design::setting_lists`）が、設計データの一覧の値を取りこぼさないこと。
//! 表に無い欄の一覧を文字列から読むと文字列になってしまうので、設計データにある一覧の値は、どれも表に当たらなければならない。
use fune_model::value::SettingValue;
use fune_service::Workspace;
use fune_service::edit::{SetText, set_op};
use std::path::Path;

fn demo() -> Workspace {
    Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
        .unwrap()
}

#[test]
fn every_list_in_the_demo_is_a_listed_setting() {
    let ws = demo();
    let design = ws.snapshot().unwrap().design;
    let lists: Vec<_> = design
        .setting_entries()
        .into_iter()
        .filter(|e| matches!(e.value, Some(SettingValue::List(_))))
        .collect();
    assert!(!lists.is_empty());
    for e in lists {
        assert!(
            e.path.is_list(),
            "{} is a list but is not in LIST_SETTINGS",
            e.path
        );
    }
}

/// 値の型は欄の形で決まる: 書かれていない一覧の欄でも一覧になり、一覧でない欄は（カンマがあっても）文字列のまま。
#[test]
fn the_type_comes_from_the_key_not_the_text() {
    let ws = demo();
    let read = |path: &str, value: &str| {
        let t = SetText {
            path: path.into(),
            value: value.into(),
            ..Default::default()
        };
        match set_op(&ws, &t).unwrap() {
            fune_service::dto::Op::SetSetting { value, .. } => value.unwrap(),
            other => panic!("{other:?}"),
        }
    };
    assert_eq!(
        read("analysis.thermal.nodes.NEW", "A"),
        SettingValue::List(vec!["A".into()])
    );
    assert_eq!(
        read("analysis.common.default_case", "a, b"),
        SettingValue::Text("a, b".into())
    );
}
