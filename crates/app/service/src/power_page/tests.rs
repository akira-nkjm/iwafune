use super::*;
use crate::dto::AnalysisCaseSource;
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
        assert_eq!(page.analyses.len(), ANALYSES.len());
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
            let json = serde_json::to_value(&analysis.result).unwrap();
            assert!(json.get("power_page").is_none());
            assert_eq!(json, serde_json::to_value(direct).unwrap());
            if case.is_some() {
                assert_eq!(
                    analysis.result.case_source,
                    Some(AnalysisCaseSource::Explicit)
                );
            }
        }
        let mode = page.modes.iter().find(|m| m.sunlit.is_some()).unwrap();
        assert!(mode.eclipse.is_some());
        assert_eq!(
            mode.bars.iter().map(|b| b.key.as_str()).collect::<Vec<_>>(),
            ["dod", "energy_balance", "area", "capacity"]
        );
        assert_eq!(mode.charts.len(), 3);
    }
}

#[test]
fn missing_analyses_still_return_raw_missing_inputs() {
    let ws = demo();
    let mut page = power_page(&ws, None).unwrap();
    for analysis in &mut page.analyses {
        analysis.result.output = None;
        analysis.result.status = crate::dto::AnalysisRunStatus::MissingInputs;
    }
    let page = assemble(page.analyses);
    assert!(page.modes.is_empty());
    assert_eq!(page.analyses.len(), 4);
}
