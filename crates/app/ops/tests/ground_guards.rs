//! 地上局の Op: 要求の `verify.check` が名指す局の削除の拒否と名前の変更でのキーの書き換え（#367）、
//! 設定の書き込みで局を作らないこと。fixtures/demo-6u に対して計画だけを立てる。

use fune_model::value::{Quantity, SettingValue};
use fune_ops::{Op, OpError, plan};
use fune_store::{Snapshot, load};
use std::path::PathBuf;

fn snapshot() -> Snapshot {
    load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")).unwrap()
}

fn create(id: &str) -> Op {
    Op::CreateGroundStation {
        id: id.into(),
        label: None,
        latitude: Some(Quantity::new(78.2, "deg")),
        longitude: Some(Quantity::new(15.4, "deg")),
        altitude: Some(Quantity::new(450.0, "m")),
        min_elevation: Some(Quantity::new(5.0, "deg")),
    }
}

/// 要求の `verify.check` が局を名指している間、削除は拒む。名前の変更は拒まず、キーを書き換える（`sat_checkkey`）。
#[test]
fn remove_is_refused_and_rename_rewrites_while_a_requirement_checks_the_station() {
    let snap = snapshot();
    let remove = Op::RemoveGroundStation {
        id: "TOKYO_GS".into(),
    };
    let err = plan(&snap, &[remove]).unwrap_err();
    assert!(matches!(err, OpError::Invalid { .. }), "{err}");
    let text = err.to_string();
    assert!(text.contains("R_LINK_S"), "{text}");
    assert!(
        text.contains("Update or remove those requirements"),
        "{text}"
    );
    let rename = Op::RenameGroundStation {
        id: "TOKYO_GS".into(),
        to: "TOKYO_2".into(),
    };
    let p = plan(&snap, &[rename]).unwrap();
    assert_eq!(
        p.design.requirements["R_LINK_S"].verify.check.as_deref(),
        Some("margin:s_down@TOKYO_2")
    );
    // 名指していない局は、要求があっても変えられる・消せる
    let p = plan(
        &snap,
        &[
            create("SECOND_GS"),
            Op::RenameGroundStation {
                id: "SECOND_GS".into(),
                to: "THIRD_GS".into(),
            },
            Op::RemoveGroundStation {
                id: "THIRD_GS".into(),
            },
        ],
    )
    .unwrap();
    assert!(p.changes.is_empty(), "{:?}", p.changes);
}

/// 局の値の書き込みは、局が既にあるときだけ（作るのは `CreateGroundStation`）。
#[test]
fn set_setting_does_not_create_stations() {
    let set = |path: &str| Op::SetSetting {
        path: path.into(),
        value: Some(SettingValue::Quantity(Quantity::new(5.0, "deg"))),
    };
    let snap = snapshot();
    for path in [
        "conditions.ground.NEW_GS.min_elevation",
        "conditions.ground.a/b.min_elevation",
    ] {
        let err = plan(&snap, &[set(path)]).unwrap_err();
        assert!(matches!(err, OpError::Invalid { .. }), "{err}");
        assert!(err.to_string().contains("fune ground add"), "{err}");
    }
    // 作ってからなら書ける
    let p = plan(
        &snap,
        &[
            create("SECOND_GS"),
            set("conditions.ground.SECOND_GS.min_elevation"),
        ],
    )
    .unwrap();
    assert!(p.design.conditions.ground.contains_key("SECOND_GS"));
    // 既存の局への書き込みは今までどおり
    plan(&snap, &[set("conditions.ground.TOKYO_GS.min_elevation")]).unwrap();
}
