//! `orbit.trapped_flux` を service 経由で、本物の配布データ（キャッシュにあるとき。無ければ読み飛ばす）とデモの設計で。
//! 地図の読み込みは `data::install_cached_datasets`（CLI・HTTP の起動時と同じ）。

use fune_service::dto::{AnalysisRunStatus, RunAnalysisRequest};
use fune_service::{Workspace, analysis, data};
use std::path::Path;

fn demo() -> Workspace {
    Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
        .unwrap()
}

/// 4 つの地図と 2 つの古い場が読み込めたら `true`。無ければ読み飛ばしを表示して `false`。
fn data_installed() -> bool {
    data::install_cached_datasets();
    let missing = data::radiation_missing();
    if !missing.is_empty() {
        eprintln!(
            "SKIPPED: the AP-8/AE-8 data is not in the data cache (missing: {}); set IWAFUNE_DATA_DIR",
            missing.join(", ")
        );
    }
    missing.is_empty()
}

#[test]
fn demo_orbit_flux_with_the_real_data() {
    if !data_installed() {
        return;
    }
    let started = std::time::Instant::now();
    let result = analysis::run_analysis(
        &demo(),
        "orbit.trapped_flux",
        &RunAnalysisRequest::default(),
    )
    .unwrap();
    assert_eq!(result.status, AnalysisRunStatus::Ok, "{:?}", result.error);
    let out = result.output.unwrap();
    eprintln!("orbit.trapped_flux took {:?}", started.elapsed());
    let cases = out["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 4);
    for case in cases {
        let model = case["model"].as_str().unwrap();
        for p in case["spectrum"].as_array().unwrap() {
            // フラックスは SI（m⁻² s⁻¹）で持ち、cm⁻² s⁻¹ に直して表示する
            eprintln!(
                "{model} >{:>7} MeV  mean {:>10.3e}  peak {:>10.3e}  fluence {:>10.3e} /cm2  time {:.3}",
                p["energy_mev"],
                p["mean_flux"].as_f64().unwrap() / 1e4,
                p["peak_flux"].as_f64().unwrap() / 1e4,
                p["fluence"].as_f64().unwrap() / 1e4,
                p["time_fraction"].as_f64().unwrap(),
            );
        }
        eprintln!(
            "{model}: L {:?}..{:?}, samples without L {}",
            case["l_min"], case["l_max"], case["samples_without_l"]
        );
    }
}
