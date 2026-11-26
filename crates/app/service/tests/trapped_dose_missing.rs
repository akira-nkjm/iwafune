//! `orbit.trapped_dose` と、環境の表の無い `orbit.total_dose` は、放射線帯のデータが読み込まれていなければ不足入力になり、
//! 直し方（`fune data fetch radiation`）を出す。`install_cached_datasets` を呼ばない（登録はプロセスで一度きりなので、読み込む試験は別のファイル）。

use fune_service::dto::{AnalysisRunStatus, RunAnalysisRequest};
use fune_service::{Workspace, analysis};
use std::path::Path;

fn run(id: &str) -> fune_service::dto::AnalysisRunResult {
    let ws =
        Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
            .unwrap();
    analysis::run_analysis(&ws, id, &RunAnalysisRequest::default()).unwrap()
}

#[test]
fn trapped_dose_without_the_data_is_a_missing_input_with_the_fix() {
    let result = run("orbit.trapped_dose");
    assert_eq!(result.status, AnalysisRunStatus::MissingInputs);
    assert!(result.output.is_none());
    let reasons: Vec<&str> = result.missing.iter().map(|m| m.reason.as_str()).collect();
    assert!(
        reasons
            .iter()
            .any(|r| r.contains("fune data fetch radiation") && r.contains("shieldose2")),
        "{reasons:?}"
    );
}

/// 環境の表の不足入力は、計算で代えられること（データ）も示す。
#[test]
fn total_dose_missing_environment_names_the_data_alternative() {
    let result = run("orbit.total_dose");
    assert_eq!(result.status, AnalysisRunStatus::MissingInputs);
    let table = result
        .missing
        .iter()
        .find(|m| m.path.ends_with("dose_rate_by_shielding"))
        .expect("the environment table is missing");
    assert!(
        table.reason.contains("fune data fetch radiation"),
        "{}",
        table.reason
    );
}
