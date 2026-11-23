//! 打ち上げ機・ディスペンサーの選択（`conditions/launch.toml` の `launcher`・`dispenser`）: 参照の索引、参照先の検査、
//! カタログ品の名前の変更と削除、部品にできない種別。fixtures/demo-6u に対して計画だけを立てる。

use fune_model::design::{LaunchInterface, RefSite, Referent};
use fune_model::value::SettingValue;
use fune_ops::{Op, OpError, plan};
use fune_store::{Snapshot, load};
use std::path::PathBuf;

fn snapshot() -> Snapshot {
    load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")).unwrap()
}

fn set(key: &str, value: Option<&str>) -> Op {
    Op::SetSetting {
        path: format!("conditions.launch.{key}"),
        value: value.map(|v| SettingValue::Text(v.to_owned())),
    }
}

fn rejected_codes(ops: &[Op]) -> Vec<String> {
    match plan(&snapshot(), ops) {
        Err(OpError::Rejected { diagnostics }) => diagnostics.into_iter().map(|d| d.code).collect(),
        other => panic!("{:?}", other.map(|p| p.changes.len())),
    }
}

/// 参照の索引は、選んでいる品を参照元として挙げる（名前の変更・削除・カタログの取り外しが同じ索引を引く）。
#[test]
fn the_reference_index_lists_the_selection() {
    let snap = snapshot();
    let d = &snap.design;
    let sites = d.references(Referent::Catalog("local/disp_6u_rs"));
    assert_eq!(
        sites,
        [RefSite::LaunchSelection {
            which: LaunchInterface::Dispenser
        }]
    );
    assert_eq!(
        d.catalog_users("local/lv_alpha"),
        ["conditions.launch.launcher"]
    );
    let by_catalog = d.references(Referent::CatalogSource("local"));
    assert!(by_catalog.contains(&RefSite::LaunchSelection {
        which: LaunchInterface::Launcher
    }));
    // 部品の一覧には出ない（衛星の部品ではない）
    assert!(d.catalog_part_users("local/lv_alpha").is_empty());
}

/// 定義されていない品を選ぶ編集は、検査の `ref.launcher`・`ref.dispenser` で拒まれる。
#[test]
fn a_dangling_selection_is_rejected() {
    assert!(
        rejected_codes(&[set("launcher", Some("local/nope"))]).contains(&"ref.launcher".into())
    );
    assert!(
        rejected_codes(&[set("dispenser", Some("other/nope"))]).contains(&"ref.dispenser".into())
    );
}

/// 打ち上げ機の欄にディスペンサーの種別の品を選ぶと、`launch.kind` で拒まれる。
#[test]
fn the_kind_must_match_the_slot() {
    assert!(
        rejected_codes(&[set("launcher", Some("local/disp_6u_rs"))])
            .contains(&"launch.kind".into())
    );
    // 部品の種別にもできない
    let part = Op::CreatePart {
        id: "LV_T".into(),
        group: Some("aocs".into()),
        group_under: vec![],
        catalog: Some("local/lv_alpha".into()),
        kind: None,
        count: None,
        label: None,
        mode_state: None,
    };
    assert!(rejected_codes(&[part]).contains(&"part.launch_kind".into()));
}

/// 選択を外せる（`fune unset`）。外すと、その品は消せる。
#[test]
fn the_selection_can_be_unset() {
    let snap = snapshot();
    let p = plan(
        &snap,
        &[
            set("dispenser", None),
            Op::RemoveCatalogItem {
                id: "local/disp_6u_rs".into(),
            },
        ],
    )
    .unwrap();
    assert!(p.design.conditions.launch.unwrap().dispenser.is_none());
    assert!(!p.design.catalog.contains_key("local/disp_6u_rs"));
}

/// 選んでいる品の削除は、使っている場所を挙げて拒む。
#[test]
fn removing_a_selected_item_is_refused() {
    match plan(
        &snapshot(),
        &[Op::RemoveCatalogItem {
            id: "local/lv_alpha".into(),
        }],
    ) {
        Err(OpError::Invalid { message }) => {
            assert!(message.contains("conditions.launch.launcher"), "{message}")
        }
        other => panic!("{:?}", other.map(|p| p.changes.len())),
    }
}

/// 品の名前を変えると、`conditions/launch.toml` の選択も書き換わる。
#[test]
fn renaming_a_selected_item_rewrites_the_selection() {
    let p = plan(
        &snapshot(),
        &[Op::RenameCatalogItem {
            from: "local/lv_alpha".into(),
            to: "local/lv_beta".into(),
        }],
    )
    .unwrap();
    let launch = p.design.conditions.launch.as_ref().unwrap();
    assert_eq!(launch.launcher.as_deref(), Some("local/lv_beta"));
    assert_eq!(launch.dispenser.as_deref(), Some("local/disp_6u_rs"));
    let file = p
        .changes
        .iter()
        .find(|c| c.path == "conditions/launch.toml")
        .unwrap();
    assert!(
        file.after
            .as_deref()
            .unwrap()
            .contains("launcher               = \"local/lv_beta\"")
    );
}
