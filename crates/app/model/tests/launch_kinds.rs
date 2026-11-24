//! 打ち上げ機・ディスペンサーの種別と属性: 組み込みの定義が属性 ID の定数と合い、部品の種別にならない（`component` を継承しない）こと。

use fune_model::builtin;
use fune_model::design::LaunchInterface;
use fune_model::design::launch::attr;

#[test]
fn built_in_kinds_list_every_launch_attribute() {
    for which in LaunchInterface::NEAREST_FIRST {
        let kind = builtin::kinds().get(which.kind_id()).unwrap();
        assert!(kind.extends.is_none(), "{which} must not be a part kind");
        assert!(kind.required.is_empty(), "every attribute is optional");
        for id in attr::ALL {
            assert!(builtin::attributes().contains_key(id), "{id}");
            assert!(
                kind.optional.iter().any(|a| a.as_str() == id),
                "{which}: {id}"
            );
        }
        assert!(!kind.label.is_empty() && kind.label_en.is_some());
    }
}

#[test]
fn the_kind_test_follows_inheritance() {
    let mut design = fune_model::Design::default();
    assert!(design.is_launch_kind("launcher"));
    assert!(design.is_launch_kind_of("dispenser", LaunchInterface::Dispenser));
    assert!(!design.is_launch_kind_of("dispenser", LaunchInterface::Launcher));
    assert!(!design.is_launch_kind("reaction_wheel"));
    // プロジェクトの種別が打ち上げ機を継承すれば、同じく部品にできない
    let custom: fune_model::design::KindDef =
        serde_json::from_value(serde_json::json!({"label": "Custom", "extends": "launcher"}))
            .unwrap();
    design
        .kinds
        .insert(fune_model::Id::new("custom_lv"), custom);
    assert!(design.is_launch_kind("custom_lv"));
    assert!(design.is_launch_kind_of("custom_lv", LaunchInterface::Launcher));
}
