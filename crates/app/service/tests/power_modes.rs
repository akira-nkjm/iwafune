//! 電源のモード別の電力: バジェットとモード表（解析を含む見方）と解析の集計が同じこと、下書きの `ops` が解析の入口と
//! モードの見方の両方に同じに当たること。
use fune_service::dto::{BudgetKind, BudgetRequest, RunAnalysisRequest, SystemModesRequest};
use fune_service::{Workspace, analysis, budget, system_modes};
use sat_analysis::spec::System;
use std::path::Path;

fn demo() -> Workspace {
    Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
        .unwrap()
}

/// バジェットのモード別の行（日照中）とモード表（日照中）は、同じ集計（`budget::mode_rollup`）の値で、
/// 解析（`power.mode_load`）の成長余裕込みの合計とも同じ。モード表はケースを取らず nominal なので、`side` が max のケース
/// （`hot_imaging`）のバジェットはモード表ではなく、同じケースの解析と比べる（#162）。
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
            system_modes::system_modes(&ws, System::Power, &SystemModesRequest::default()).unwrap();
        let req = RunAnalysisRequest {
            case: case.map(str::to_owned),
            ..Default::default()
        };
        let load = analysis::run_analysis(&ws, "power.mode_load", &req).unwrap();
        let load_modes = load.output.as_ref().unwrap()["modes"].as_array().unwrap();
        assert!(!b.modes.is_empty());
        let mut with_growth_differs = false;
        for row in &b.modes {
            let id = row.node.as_deref().unwrap();
            let mode = table.modes.iter().find(|m| m.id == id).unwrap();
            let sunlit = &mode.sunlit;
            if case.is_none() {
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
            }
            let analysed = load_modes.iter().find(|m| m["id"] == id).unwrap();
            let total = analysed["total_with_growth"].as_f64().unwrap();
            assert!((row.with_growth.si - total).abs() <= 1e-9 * total.abs().max(1.0));
            with_growth_differs |= row.with_growth.si > row.current.si;
        }
        // 成長余裕が効いている設計で確かめている
        assert!(with_growth_differs);
    }
}
