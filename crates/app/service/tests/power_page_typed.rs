//! 電源のページ: 下書きの `ops` を受けること、型付きで回した結果が汎用の解析入口と同じこと、モード別の電力の集計が
//! バジェットとモード表で同じこと。
use fune_model::value::SettingValue;
use fune_service::dto::{
    AnalysisRunStatus, BudgetKind, BudgetRequest, Op, PartViewRequest, PowerPageQuery,
    RunAnalysisRequest,
};
use fune_service::{Workspace, analysis, budget, power_page, system_modes};
use sat_analysis::spec::System;
use std::path::Path;

fn demo() -> Workspace {
    Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
        .unwrap()
}

fn dod_limit(page: &fune_service::dto::PowerPageView) -> f64 {
    let balance = page
        .analyses
        .iter()
        .find(|a| a.result.id == "power.energy_balance")
        .unwrap();
    balance.result.output.as_ref().unwrap()["dod_limit"]
        .as_f64()
        .unwrap()
}

#[test]
fn draft_ops_apply_to_the_page_without_writing() {
    let ws = demo();
    let saved = power_page::power_page(&ws, None).unwrap();
    assert!((dod_limit(&saved) - 0.3).abs() < 1e-12);
    let draft = PowerPageQuery {
        case: None,
        ops: vec![Op::SetSetting {
            path: "analysis.power.dod_max".to_owned(),
            value: Some(SettingValue::Number(0.2)),
        }],
    };
    let page = power_page::power_page_with(&ws, &draft).unwrap();
    assert!((dod_limit(&page) - 0.2).abs() < 1e-12);
    // 書き込まない: 保存済みの設計の数値は同じ
    let again = power_page::power_page(&ws, None).unwrap();
    assert!((dod_limit(&again) - 0.3).abs() < 1e-12);
    // 下書きの DoD の上限は、同じ編集を当てた汎用の解析入口の結果と同じ
    let req = RunAnalysisRequest {
        ops: draft.ops.clone(),
        ..Default::default()
    };
    let direct = analysis::run_analysis(&ws, "power.energy_balance", &req).unwrap();
    assert_eq!(direct.output.as_ref().unwrap()["dod_limit"], 0.2);
}

#[test]
fn typed_runs_match_the_generic_entry_for_every_case() {
    let ws = demo();
    for case in [None, Some("hot_imaging"), Some("eclipse_worst")] {
        let page = power_page::power_page(&ws, case).unwrap();
        assert_eq!(page.analyses.len(), 4);
        for a in &page.analyses {
            let req = RunAnalysisRequest {
                case: case.map(str::to_owned),
                ..Default::default()
            };
            let direct = analysis::run_analysis(&ws, &a.result.id, &req).unwrap();
            assert_eq!(a.result.status, direct.status, "{}", a.result.id);
            assert_eq!(a.result.input, direct.input, "{}", a.result.id);
            assert_eq!(a.result.output, direct.output, "{}", a.result.id);
            assert_eq!(a.result.dims, direct.dims, "{}", a.result.id);
            assert_eq!(a.result.labels, direct.labels, "{}", a.result.id);
            assert_eq!(a.result.kpis.len(), direct.kpis.len(), "{}", a.result.id);
            assert_eq!(a.result.status, AnalysisRunStatus::Ok);
        }
    }
}

/// バジェットのモード別の行（日照中）とモード表（日照中）は、同じ集計（`budget::mode_rollup`）の値で、
/// 解析（`power.mode_load`）の成長余裕込みの合計とも同じ。
#[test]
fn mode_power_aggregation_agrees_between_budget_and_mode_table() {
    let ws = demo();
    for case in [None, Some("hot_imaging")] {
        let b = budget::budget(
            &ws,
            BudgetKind::Power,
            &BudgetRequest {
                case: case.map(str::to_owned),
                ..Default::default()
            },
        )
        .unwrap();
        let table =
            system_modes::system_modes(&ws, System::Power, &PartViewRequest::default()).unwrap();
        let load =
            analysis::run_analysis(&ws, "power.mode_load", &RunAnalysisRequest::default()).unwrap();
        let load_modes = load.output.as_ref().unwrap()["modes"].as_array().unwrap();
        assert!(!b.modes.is_empty());
        let mut with_growth_differs = false;
        for row in &b.modes {
            let id = row.node.as_deref().unwrap();
            let mode = table.modes.iter().find(|m| m.id == id).unwrap();
            let sunlit = &mode.sunlit;
            assert_eq!(
                Some(row.with_growth.si),
                sunlit.total.as_ref().map(|t| t.si),
                "{id}"
            );
            assert_eq!(
                row.allocated.as_ref().map(|q| q.si),
                sunlit.available.as_ref().map(|q| q.si),
                "{id}"
            );
            assert_eq!(
                row.margin.as_ref().map(|q| q.si),
                sunlit.margin.as_ref().map(|q| q.si),
                "{id}"
            );
            let analysed = load_modes.iter().find(|m| m["id"] == id).unwrap();
            let total = analysed["total_with_growth"].as_f64().unwrap();
            assert!((row.with_growth.si - total).abs() <= 1e-9 * total.abs().max(1.0));
            with_growth_differs |= row.with_growth.si > row.current.si;
        }
        // 成長余裕が効いている設計で確かめている
        assert!(with_growth_differs);
    }
}
