//! API の型を `web/src/api/gen/` に TS として書き出す。CI はこのあと `git diff --exit-code web/src/api/gen` で差分が無いことを確かめる。

use std::path::Path;

#[test]
fn export_ts() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../web/src/api/gen");
    let files = fune_service::dto::export_ts(&dir).expect("TS types can be written");
    assert!(files.iter().any(|f| f.ends_with("index.ts")));
}

/// 重い任意の出力の型名（`Analysis::DETAIL_TYPE`）は、その型の TS の名前と一致し、書き出される。
#[test]
fn detail_type_names_match_the_exported_types() {
    use ts_rs::TS;
    let cfg = ts_rs::Config::new();
    let name = <sat_analysis::structure::ModalFemShapes as TS>::name(&cfg);
    assert_eq!(
        sat_analysis::catalog::detail_type("structure.modal_fem"),
        Some(name.as_str())
    );
    let name = <sat_analysis::thermal::ThermalPlateTemperatures as TS>::name(&cfg);
    for id in ["thermal.steady", "thermal.transient"] {
        assert_eq!(sat_analysis::catalog::detail_type(id), Some(name.as_str()));
    }
    let dir = tempfile::tempdir().unwrap();
    fune_service::dto::export_ts(dir.path()).unwrap();
    for entry in fune_service::analysis::analysis_catalog().unwrap() {
        if let Some(name) = entry.detail_type {
            assert!(dir.path().join(format!("{name}.ts")).exists(), "{name}");
        }
    }
}
