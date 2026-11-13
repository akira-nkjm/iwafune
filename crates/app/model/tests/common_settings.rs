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

#[test]
fn mode_duration_requires_time_dimension() {
    use fune_model::design::Mode;
    use fune_model::value::Quantity;
    let mut design = Design::default();
    let mut mode = Mode {
        id: "TEST".into(),
        max_duration: Some(Quantity::new(90.0, "min")),
        ..Default::default()
    };
    design.modes.insert(mode.id.clone(), mode.clone());
    assert!(
        !fune_model::check(&design)
            .iter()
            .any(|d| d.code == "unit.dimension")
    );
    mode.max_duration = Some(Quantity::new(90.0, "W"));
    design.modes.insert(mode.id.clone(), mode);
    assert!(
        fune_model::check(&design)
            .iter()
            .any(|d| d.code == "unit.dimension" && d.severity == Severity::Error)
    );
}

#[test]
fn mode_population_errors_are_design_errors() {
    use fune_model::design::{Mode, Part, Redundancy, StateAssign};
    let mut design = Design::default();
    let mut part = Part::new("HTR");
    part.kind = Some("heater".into()); // off / on、warm の既定の予備状態は無い
    part.count = 4;
    for (redundancy, active, code) in [
        (Redundancy::Cold, 5, "mode.active"),
        (Redundancy::None, 3, "mode.redundancy"),
        (Redundancy::Warm, 3, "mode.spare_state"),
    ] {
        part.redundancy = redundancy;
        design.parts.insert(part.id.clone(), part.clone());
        let mut row = StateAssign::new("on");
        row.active = Some(active);
        let mode = Mode {
            id: "TEST".into(),
            states: [(part.id.clone(), row)].into(),
            ..Default::default()
        };
        design.modes.insert(mode.id.clone(), mode);
        assert!(
            fune_model::check(&design)
                .iter()
                .any(|d| d.code == code && d.severity == Severity::Error),
            "{code}"
        );
    }
}

/// 電源の基本のモード（`analysis.power.base_mode`）は既存のモードを指す。
#[test]
fn power_base_mode_must_name_a_mode() {
    use fune_model::design::{Mode, PowerSettings};
    let mut design = Design::default();
    design.analysis.power = Some(PowerSettings {
        base_mode: Some("NOMINAL".to_owned()),
        ..Default::default()
    });
    let target = Some(Target::from(SettingPath::analysis(&["power", "base_mode"])));
    let errors = |d: &Design| {
        fune_model::check(d)
            .into_iter()
            .filter(|x| x.severity == Severity::Error && x.code == "ref.mode" && x.target == target)
            .count()
    };
    assert_eq!(errors(&design), 1);
    let mode = Mode {
        id: "NOMINAL".into(),
        ..Default::default()
    };
    design.modes.insert(mode.id.clone(), mode);
    assert_eq!(errors(&design), 0);
}
