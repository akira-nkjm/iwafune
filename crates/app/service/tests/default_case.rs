//! projection のケース方針が service の入力と選択元に反映されること。
use fune_model::value::SettingValue;
use fune_service::dto::{
    AnalysisCaseSource, AnalysisInputSource, AnalysisRunStatus, Op, RunAnalysisRequest,
};
use fune_service::{Workspace, analysis};
use std::path::Path;

fn demo() -> Workspace {
    Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
        .unwrap()
}

fn default_request() -> RunAnalysisRequest {
    RunAnalysisRequest {
        ops: vec![Op::SetSetting {
            path: "analysis.common.default_case".to_owned(),
            value: Some(SettingValue::Text("hot_imaging".to_owned())),
        }],
        ..Default::default()
    }
}

#[test]
fn default_case_supplies_the_missing_analysis_conditions() {
    let ws = demo();
    let original = ws.snapshot().unwrap().design.analysis.common.clone();
    for id in [
        "power.orbit_generation",
        "attitude.rw_unloading",
        "attitude.sensor_fov",
        "thermal.transient",
    ] {
        let result = analysis::run_analysis(&ws, id, &default_request()).unwrap();
        assert_eq!(result.case.as_deref(), Some("hot_imaging"), "{id}");
        assert_eq!(result.case_source, Some(AnalysisCaseSource::Default));
        assert_eq!(
            result.status,
            AnalysisRunStatus::Ok,
            "{id}: {:?}",
            result.error
        );
        assert!(!result.kpis.is_empty(), "{id}");
    }
    assert_eq!(ws.snapshot().unwrap().design.analysis.common, original);
}

#[test]
fn explicit_case_overrides_default_and_patched_input_uses_default() {
    let ws = demo();
    let mut req = default_request();
    req.case = Some("cold_safe".to_owned());
    let result = analysis::run_analysis(&ws, "thermal.transient", &req).unwrap();
    assert_eq!(result.case.as_deref(), Some("cold_safe"));
    assert_eq!(result.case_source, Some(AnalysisCaseSource::Explicit));
    assert_eq!(
        result.input.as_ref().unwrap()["environment"]["solar_flux"],
        1322.0
    );
    req.case = None;
    req.source = AnalysisInputSource::Patched {
        patch: serde_json::json!({}),
    };
    let result = analysis::run_analysis(&ws, "thermal.transient", &req).unwrap();
    assert_eq!(result.case.as_deref(), Some("hot_imaging"));
    assert_eq!(result.case_source, Some(AnalysisCaseSource::Default));
}

#[test]
fn absent_default_keeps_missing_inputs_and_given_input_has_no_default_case() {
    let ws = demo();
    let without_default = RunAnalysisRequest {
        ops: vec![Op::SetSetting {
            path: "analysis.common.default_case".to_owned(),
            value: None,
        }],
        ..Default::default()
    };
    let result = analysis::run_analysis(&ws, "power.orbit_generation", &without_default).unwrap();
    assert_eq!(result.status, AnalysisRunStatus::MissingInputs);
    assert_eq!((result.case, result.case_source), (None, None));
    let mut req = default_request();
    let projected = analysis::run_analysis(&ws, "thermal.transient", &req).unwrap();
    req.source = AnalysisInputSource::Given {
        input: projected.input.unwrap(),
    };
    let result = analysis::run_analysis(&ws, "thermal.transient", &req).unwrap();
    assert_eq!(result.status, AnalysisRunStatus::Ok);
    assert_eq!((result.case, result.case_source), (None, None));
}

/// 運用全体の集計には既定ケースを適用しない。明示した場合だけ対象モードに限定する。
#[test]
fn data_budget_keeps_all_modes_even_with_a_default_case() {
    let ws = demo();
    let mut req = default_request();
    let all = analysis::run_analysis(&ws, "cdh.data_budget", &req).unwrap();
    assert_eq!(all.status, AnalysisRunStatus::Ok);
    assert_eq!((all.case, all.case_source), (None, None));
    req.case = Some("hot_imaging".to_owned());
    let selected = analysis::run_analysis(&ws, "cdh.data_budget", &req).unwrap();
    assert_eq!(selected.status, AnalysisRunStatus::Ok);
    assert_eq!(selected.case_source, Some(AnalysisCaseSource::Explicit));
    assert_ne!(
        all.output.unwrap()["total_per_day"],
        selected.output.unwrap()["total_per_day"]
    );
}

/// 電力収支の既定は上段の姿勢則だけ。ケース・β・周回数は全体へ適用しない。
#[test]
fn energy_balance_uses_default_only_for_the_top_level_attitude() {
    let ws = demo();
    let mut req = default_request();
    let all = analysis::run_analysis(&ws, "power.energy_balance", &req).unwrap();
    assert_eq!(all.status, AnalysisRunStatus::Ok);
    assert_eq!(all.case, None);
    assert_eq!(all.case_source, Some(AnalysisCaseSource::DefaultAttitude));
    let input = all.input.unwrap();
    assert_eq!(
        input["cases"].as_array().unwrap().len(),
        ws.snapshot().unwrap().design.modes.len()
    );
    assert!(input["timeline"]["beta"].is_null());
    req.case = Some("hot_imaging".to_owned());
    let selected = analysis::run_analysis(&ws, "power.energy_balance", &req).unwrap();
    assert_eq!(selected.status, AnalysisRunStatus::Ok);
    assert_eq!(selected.case_source, Some(AnalysisCaseSource::Explicit));
    let selected = selected.input.unwrap();
    assert_eq!(selected["cases"].as_array().unwrap().len(), 1);
    assert_eq!(
        input["timeline"]["attitude"],
        selected["timeline"]["attitude"]
    );
    assert!(!selected["timeline"]["beta"].is_null());
}
