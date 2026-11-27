//! `orbit.trapped_dose` を service 経由で、本物の配布データ（キャッシュにあるとき。無ければ読み飛ばす）とデモの設計で。
//! 環境の読み込みは `data::install_cached_datasets`（CLI・HTTP の起動時と同じ）。

use fune_service::dto::{AnalysisRunStatus, RunAnalysisRequest};
use fune_service::{Workspace, analysis, data};
use std::path::Path;

fn demo() -> Workspace {
    Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
        .unwrap()
}

/// マップ・場・SHIELDOSE-2 の表がそろったら `true`。
fn data_installed() -> bool {
    data::install_cached_datasets();
    let missing = fune_projection::orbit::dose_data_missing();
    if !missing.is_empty() {
        eprintln!(
            "SKIPPED: the trapped-radiation data is not in the data cache (missing: {}); set IWAFUNE_DATA_DIR",
            missing.join(", ")
        );
    }
    missing.is_empty()
}

#[test]
fn demo_orbit_dose_with_the_real_data() {
    if !data_installed() {
        return;
    }
    let started = std::time::Instant::now();
    let result = analysis::run_analysis(
        &demo(),
        "orbit.trapped_dose",
        &RunAnalysisRequest::default(),
    )
    .unwrap();
    assert_eq!(result.status, AnalysisRunStatus::Ok, "{:?}", result.error);
    eprintln!("orbit.trapped_dose took {:?}", started.elapsed());
    let out = result.output.unwrap();
    let cases = out["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 2);
    assert_eq!(out["used"]["geometry"], "sphere_centre");
    assert_eq!(out["used"]["detector"], "silicon");
    assert_eq!(out["used"]["nuclear"], "none");
    for case in cases {
        let table = case["table"].as_array().unwrap();
        assert_eq!(table.len(), 13);
        let totals: Vec<f64> = table.iter().map(|p| p["total"].as_f64().unwrap()).collect();
        assert!(totals.iter().all(|t| *t > 0.0));
        assert!(totals.windows(2).all(|w| w[1] <= w[0]), "{totals:?}");
    }
    // 結果の `dims` と `labels` が出る（CLI の表が使う）
    assert!(result.dims.contains_key("/cases/0/table/0/total"));
    assert!(result.labels.contains_key("/cases/0/solar"));
}
