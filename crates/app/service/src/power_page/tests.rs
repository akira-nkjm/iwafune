use super::*;
use crate::analysis;
use crate::dto::{AnalysisCaseSource, RunAnalysisRequest};
use std::path::Path;

fn demo() -> Workspace {
    Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
        .unwrap()
}

#[test]
fn whole_page_collects_unchanged_analysis_results_and_merges_modes() {
    let ws = demo();
    for case in [None, Some("hot_imaging")] {
        let page = power_page(&ws, case).unwrap();
        assert_eq!(page.analyses.len(), 4);
        for analysis in &page.analyses {
            let direct = analysis::run_analysis(
                &ws,
                &analysis.result.id,
                &RunAnalysisRequest {
                    case: case.map(str::to_owned),
                    ..Default::default()
                },
            )
            .unwrap();
            // 結果は解析の入口と同じ。概要の行だけは電源の棒グラフと隠す表が足される（主な結果と判定のまとまりは同じ）
            let without_overview = |r: &crate::dto::AnalysisRunResult| {
                let mut json = serde_json::to_value(r).unwrap();
                json.as_object_mut().unwrap().remove("overview");
                json
            };
            let json = without_overview(&analysis.result);
            assert!(json.get("power_page").is_none());
            assert_eq!(json, without_overview(&direct));
            assert_eq!(analysis.result.overview.main, direct.overview.main);
            let scopes = |r: &crate::dto::AnalysisRunResult| -> Vec<_> {
                r.overview.checks.iter().map(|g| g.scope.clone()).collect()
            };
            assert_eq!(scopes(&analysis.result), scopes(&direct));
            assert!(direct.overview.chart.is_none() && direct.overview.hidden_tables.is_empty());
            if case.is_some() {
                assert_eq!(
                    analysis.result.case_source,
                    Some(AnalysisCaseSource::Explicit)
                );
            }
        }
        let mode = page.modes.iter().find(|m| m.sunlit.is_some()).unwrap();
        assert!(mode.eclipse.is_some());
        assert_eq!(mode.charts.len(), 3);
    }
}

#[test]
fn missing_analyses_still_return_raw_missing_inputs() {
    let ws = demo();
    let snapshot = ws.snapshot().unwrap();
    let mut runs = Runs {
        load: crate::analysis::run_typed_in(&snapshot, None).unwrap(),
        orbit: crate::analysis::run_typed_in(&snapshot, None).unwrap(),
        balance: crate::analysis::run_typed_in(&snapshot, None).unwrap(),
        sizing: crate::analysis::run_typed_in(&snapshot, None).unwrap(),
    };
    assert!(!assemble(&runs).modes.is_empty());
    // どの解析も出力が無い（不足入力・失敗）ときは、表示は空で、元の結果だけが返る
    (runs.load.output, runs.load.input) = (None, None);
    (runs.orbit.output, runs.orbit.input) = (None, None);
    (runs.balance.output, runs.balance.input) = (None, None);
    (runs.sizing.output, runs.sizing.input) = (None, None);
    assert!(assemble(&runs).modes.is_empty());
    assert_eq!(analyses(runs).unwrap().len(), 4);
}
