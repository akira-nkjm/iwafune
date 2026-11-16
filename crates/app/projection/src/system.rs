//! 解析の系の対応（設計の `fune_model::design::AnalysisSystem` ⇄ 解析の `sat_analysis::spec::System`）。
//!
//! 系の一覧の正本は model の `AnalysisSystem`。sat-analysis は fune-* に依存できないので同じ列挙を持つしかなく、
//! 両方が見えるここで漏れの無い `match` で対応させ（どちらかに系を足すとここがコンパイルできない）、綴りの一致と
//! 解析 ID の接頭辞をテストで確かめる。

use fune_model::design::{AnalysisSystem, ConditionTopic};
use sat_analysis::spec::{AnalysisArea, ConditionTopic as AnalysisConditionTopic, System};

/// 設計の系 → 解析の系。
pub fn to_analysis(system: AnalysisSystem) -> System {
    match system {
        AnalysisSystem::Power => System::Power,
        AnalysisSystem::Aocs => System::Aocs,
        AnalysisSystem::Cdh => System::Cdh,
        AnalysisSystem::Comm => System::Comm,
        AnalysisSystem::Thermal => System::Thermal,
        AnalysisSystem::Structure => System::Structure,
        AnalysisSystem::Payload => System::Payload,
        AnalysisSystem::Propulsion => System::Propulsion,
    }
}

/// 解析の系 → 設計の系。
pub fn from_analysis(system: System) -> AnalysisSystem {
    match system {
        System::Power => AnalysisSystem::Power,
        System::Aocs => AnalysisSystem::Aocs,
        System::Cdh => AnalysisSystem::Cdh,
        System::Comm => AnalysisSystem::Comm,
        System::Thermal => AnalysisSystem::Thermal,
        System::Structure => AnalysisSystem::Structure,
        System::Payload => AnalysisSystem::Payload,
        System::Propulsion => AnalysisSystem::Propulsion,
    }
}

/// 設計の話題 → 解析の話題。
pub fn topic_to_analysis(topic: ConditionTopic) -> AnalysisConditionTopic {
    match topic {
        ConditionTopic::Orbit => AnalysisConditionTopic::Orbit,
    }
}

/// 解析の話題 → 設計の話題。
pub fn topic_from_analysis(topic: AnalysisConditionTopic) -> ConditionTopic {
    match topic {
        AnalysisConditionTopic::Orbit => ConditionTopic::Orbit,
    }
}

/// 解析の区分のキー（解析 ID の接頭辞）。
pub fn area_key(area: AnalysisArea) -> &'static str {
    match area {
        AnalysisArea::System(system) => from_analysis(system).as_str(),
        AnalysisArea::Condition(topic) => topic_from_analysis(topic).as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn systems_have_the_same_spelling_on_both_sides() {
        for system in AnalysisSystem::ALL {
            let analysis = to_analysis(*system);
            assert_eq!(from_analysis(analysis), *system);
            assert_eq!(
                serde_json::to_value(analysis).unwrap(),
                serde_json::Value::String(system.as_str().to_owned())
            );
        }
    }

    /// Payload は解析を持たず、旧綴りの系と解析 ID は受け付けない。
    #[test]
    fn payload_has_no_analyses_and_old_ids_are_rejected() {
        assert!(
            !sat_analysis::catalog::catalog()
                .iter()
                .any(|a| a.area == AnalysisArea::System(System::Payload))
        );
        assert!(serde_json::from_str::<System>(r#""attitude""#).is_err());
        for id in [
            "attitude.rw_unloading",
            "attitude.unload_time",
            "attitude.sensor_fov",
        ] {
            assert!(matches!(
                sat_analysis::catalog::run_json(id, serde_json::Value::Null),
                Err(sat_analysis::spec::AnalysisError::Unknown(_))
            ));
        }
    }

    /// 解析 ID は `<区分のキー>.<名前>` で、区分はその解析の `AnalysisArea`（docs/architecture/registry.md §4）。
    #[test]
    fn analysis_ids_start_with_their_area() {
        let catalog = sat_analysis::catalog::catalog();
        assert!(!catalog.is_empty());
        for info in catalog {
            let system = area_key(info.area);
            assert!(
                info.id.starts_with(&format!("{system}.")),
                "{} の系は {system}",
                info.id
            );
        }
    }
}

#[cfg(test)]
mod topic_tests {
    use super::*;

    #[test]
    fn topics_roundtrip_and_do_not_collide_with_systems() {
        for topic in ConditionTopic::ALL {
            let analysis = topic_to_analysis(*topic);
            assert_eq!(topic_from_analysis(analysis), *topic);
            assert_eq!(serde_json::to_value(analysis).unwrap(), topic.as_str());
            assert!(!AnalysisSystem::NAMES.contains(&topic.as_str()));
            assert_eq!(
                topic.setting_path().scope,
                fune_model::path::SettingScope::Conditions
            );
            assert!(!topic.label().is_empty());
            assert!(!topic.label_en().is_empty());
        }
        let area = AnalysisArea::Condition(AnalysisConditionTopic::Orbit);
        assert_eq!(area_key(area), ConditionTopic::Orbit.as_str());
        assert_eq!(
            serde_json::to_value(area).unwrap(),
            serde_json::json!({"kind": "condition", "value": "orbit"})
        );
    }
}
