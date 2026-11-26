//! `orbit.trapped_flux` は、放射線帯のデータ（地図・古い地磁気モデル）が読み込まれていなければ不足入力になり、
//! 直し方（`fune data fetch radiation`）を出す。このファイルの試験は `install_cached_datasets` を呼ばない
//! （地図の登録はプロセスで一度きりなので、読み込む試験は別のファイル `trapped_flux.rs`）。

use fune_service::dto::{AnalysisRunStatus, RunAnalysisRequest};
use fune_service::{Workspace, analysis};
use std::path::Path;

#[test]
fn missing_radiation_data_is_a_missing_input_with_the_fix() {
    let ws =
        Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
            .unwrap();
    let result =
        analysis::run_analysis(&ws, "orbit.trapped_flux", &RunAnalysisRequest::default()).unwrap();
    assert_eq!(
        result.status,
        AnalysisRunStatus::MissingInputs,
        "{:?}",
        result.error
    );
    assert!(result.output.is_none());
    let reasons: Vec<&str> = result.missing.iter().map(|m| m.reason.as_str()).collect();
    assert!(
        reasons
            .iter()
            .any(|r| r.contains("fune data fetch radiation") && r.contains("ap8min")),
        "{reasons:?}"
    );
    assert!(result.missing.iter().all(|m| m.fixes.is_empty()));
}
