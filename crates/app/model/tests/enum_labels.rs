//! 組み込みの語彙の表（labels.toml の `[enum.<型>.<値>]`）に、列挙のすべての値の表示名がある（余分な値も無い）。
//! 型の名前（`enum_label(ty, …)` の `ty`）と列挙の対応はこの表だけ。列挙を足したら、ここと labels.toml に足す。

use fune_model::Severity;
use fune_model::builtin::labels;
use fune_model::design::{
    AttitudeLawKind, BudgetKind, LinkDirection, Maturity, NetKind, PropagatorKind, SourceKind,
    UncertaintySide,
};
use fune_model::resolve::{Evidence, Origin};
use fune_model::value::ValueType;

/// 表示名を持つ列挙（labels.toml の型の名前 → 値の綴りの一覧）。
const LABELLED: &[(&str, &[&str])] = &[
    ("maturity", Maturity::NAMES),
    // 真偽の値（設定の `true`・`false` の表示名。`builtin::setting_enums::setting_value_label`）
    ("bool", &["false", "true"]),
    ("redundancy", fune_model::design::Redundancy::NAMES),
    ("net_kind", NetKind::NAMES),
    ("budget_kind", BudgetKind::NAMES),
    ("source_kind", SourceKind::NAMES),
    ("distribution", fune_model::design::Distribution::NAMES),
    (
        "shared_catalog_state",
        fune_model::design::SharedCatalogState::NAMES,
    ),
    ("evidence", Evidence::NAMES),
    ("origin", Origin::NAMES),
    ("severity", Severity::NAMES),
    ("attitude_law", AttitudeLawKind::NAMES),
    (
        "thermal_preset",
        fune_model::design::ThermalPresetKind::NAMES,
    ),
    ("propagator", PropagatorKind::NAMES),
    ("link_direction", LinkDirection::NAMES),
    ("uncertainty_side", UncertaintySide::NAMES),
    ("value_type", ValueType::NAMES),
    (
        "maneuver_category",
        fune_model::design::ManeuverCategory::NAMES,
    ),
    ("maneuver_method", fune_model::design::ManeuverMethod::NAMES),
    ("central_body", fune_model::design::CentralBodyKind::NAMES),
    (
        "maneuver_steering",
        fune_model::design::ManeuverSteering::NAMES,
    ),
];

#[test]
fn every_enum_value_has_a_label() {
    for (ty, values) in LABELLED {
        let mut values = values.to_vec();
        values.sort_unstable();
        let labelled: Vec<&str> = labels().enums[*ty].keys().map(String::as_str).collect();
        assert_eq!(labelled, values, "enum.{ty}");
        for v in &values {
            let label = &labels().enums[*ty][*v];
            assert!(
                !label.label.is_empty() && label.label_en.is_some(),
                "enum.{ty}.{v}"
            );
        }
    }
    let mut types: Vec<&str> = LABELLED.iter().map(|(ty, _)| *ty).collect();
    types.sort_unstable();
    let listed: Vec<&str> = labels().enums.keys().map(String::as_str).collect();
    assert_eq!(listed, types, "表示名の表にある列挙の型");
}

/// 系の表示名と並び（labels.toml の `[section.<系>]`・`section_order`）と、役割 → 系（`role_system`）は、
/// 解析の系の一覧（`AnalysisSystem`）と一致する。系でない区分（`basic`・`mass`）は表示名と並びにだけある。
#[test]
fn sections_and_role_systems_name_analysis_systems() {
    use fune_model::design::AnalysisSystem;
    let l = labels();
    for system in AnalysisSystem::ALL {
        let name = system.as_str();
        assert!(l.section.contains_key(name), "section.{name}");
        assert!(
            l.section_order.iter().any(|s| s == name),
            "section_order に {name}"
        );
    }
    for (role, system) in &l.role_system {
        let Some(system) = system else { continue };
        assert!(
            system.parse::<AnalysisSystem>().is_ok(),
            "role_system.{role} = {system} は解析の系ではありません"
        );
    }
    let mut order = l.section_order.clone();
    order.sort();
    let sections: Vec<String> = l.section.keys().cloned().collect();
    assert_eq!(
        order, sections,
        "section_order はすべての区分を 1 回ずつ並べる"
    );
}
