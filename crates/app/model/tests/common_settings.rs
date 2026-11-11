//! 共通解析設定のケース参照と、未設定の入力欄の検査。
use fune_model::design::{Case, CommonSettings};
use fune_model::path::SettingPath;
use fune_model::{Design, Severity, Target};

#[test]
fn default_case_must_exist() {
    let mut design = Design::default();
    design.analysis.common = Some(CommonSettings {
        default_case: Some("selected".to_owned()),
    });
    let target = Some(Target::from(SettingPath::analysis(&[
        "common",
        "default_case",
    ])));
    let diagnostics = fune_model::check(&design);
    assert!(
        diagnostics.iter().any(|d| {
            d.severity == Severity::Error && d.code == "ref.case" && d.target == target
        })
    );
    let mut case: Case = serde_json::from_value(serde_json::json!({})).unwrap();
    case.id = "selected".into();
    design.cases.insert(case.id.clone(), case);
    assert!(
        !fune_model::check(&design)
            .iter()
            .any(|d| d.target == target)
    );
}

#[test]
fn unset_default_case_is_listed() {
    let design = Design::default();
    let path = SettingPath::analysis(&["common", "default_case"]);
    assert!(
        design
            .setting_entries()
            .iter()
            .any(|e| e.path == path && e.value.is_none())
    );
}
