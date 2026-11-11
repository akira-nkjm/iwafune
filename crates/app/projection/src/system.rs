//! 解析の系の対応（設計の `fune_model::design::AnalysisSystem` ⇄ 解析の `sat_analysis::spec::System`）。
//!
//! 系の一覧の正本は model の `AnalysisSystem`。sat-analysis は fune-* に依存できないので同じ列挙を持つしかなく、
//! 両方が見えるここで漏れの無い `match` で対応させ（どちらかに系を足すとここがコンパイルできない）、綴りの一致と
//! 解析 ID の接頭辞をテストで確かめる。

use fune_model::design::AnalysisSystem;
use sat_analysis::spec::System;

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
                .any(|a| a.system == System::Payload)
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

    /// 解析 ID は `<系>.<名前>` で、系はその解析の `System`（docs/architecture/registry.md §4）。
    #[test]
    fn analysis_ids_start_with_their_system() {
        let catalog = sat_analysis::catalog::catalog();
        assert!(!catalog.is_empty());
        for info in catalog {
            let system = from_analysis(info.system);
            assert!(
                info.id.starts_with(&format!("{system}.")),
                "{} の系は {system}",
                info.id
            );
        }
    }
}
