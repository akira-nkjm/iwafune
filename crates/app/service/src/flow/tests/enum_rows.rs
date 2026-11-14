//! 設定の列挙の値と真偽の値の行（語彙の表示名）と、列挙の欄の表の漏れ。

use super::demo_flow;
use crate::workspace::Workspace;
use std::path::Path;

/// 設定の列挙の値と真偽の値は、語彙の表示名（日英）で出す。列挙でない文字はそのまま。
#[test]
fn setting_enum_values_show_their_display_names() {
    let flow = demo_flow();
    let card = |id: &str| flow.nodes.iter().find(|n| n.id == id).unwrap();
    let row = |card: &crate::dto::FlowNode, id: &str| {
        let r = card.rows.iter().find(|r| r.id == id).unwrap();
        (r.display.clone(), r.display_en.clone())
    };
    let orbit = card("setting:conditions.orbit");
    assert_eq!(
        row(orbit, "propagator"),
        (Some("J2 永年項".to_owned()), Some("J2 secular".to_owned()))
    );
    assert_eq!(
        row(orbit, "sun_synchronous"),
        (Some("はい".to_owned()), Some("Yes".to_owned()))
    );
    // 列挙でない文字は英語も同じ（display_en は null）
    assert_eq!(row(orbit, "ltdn"), (Some("10:30".to_owned()), None));
    let aocs = card("setting:analysis.aocs");
    let (ja, en) = row(aocs, "law.MISSION");
    assert!(ja.unwrap().starts_with("地心指向 · "));
    assert!(en.unwrap().starts_with("Nadir pointing · "));
}

/// デモの設定の文字の値が、どれかの列挙の綴りなら、列挙の欄の表（`setting_enums`）にある（表の漏れを見つける）。
#[test]
fn demo_text_settings_spelled_like_enums_are_in_the_table() {
    use fune_model::builtin::labels;
    use fune_model::builtin::setting_enums::setting_enum;
    use fune_model::value::SettingValue;
    let ws =
        Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
            .unwrap();
    let s = ws.snapshot().unwrap();
    for e in s.design.setting_entries() {
        let Some(SettingValue::Text(text)) = &e.value else {
            continue;
        };
        let spelled = labels()
            .enums
            .iter()
            .any(|(ty, values)| ty != "bool" && values.contains_key(text));
        if spelled {
            assert!(setting_enum(&e.path).is_some(), "{} = {text}", e.path);
        }
    }
}
