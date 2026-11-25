//! 回線・ケースを足すダイアログの選択肢（`GET /design` の `add_choices`）。デモの設計の送受信機・帯域・熱の環境条件と一致する。
use fune_service::Workspace;
use std::path::Path;

#[test]
fn the_demo_offers_its_transceivers_bands_and_thermal_cases() {
    let ws =
        Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
            .unwrap();
    let view = fune_service::query::design_view(&ws).unwrap();
    let c = &view.add_choices;
    // 送受信機の役割の部品だけ（役割の無い部品や、アンテナは入らない）。ID の順
    assert_eq!(c.transceivers, ["STX", "UHF_TRX"]);
    // 地上局が性能を持つ帯域（重複なし。名前の順）
    assert_eq!(c.bands, ["s_band", "uhf"]);
    // 設計の熱の環境条件の名前の順
    let mut sorted = c.thermal_cases.clone();
    sorted.sort();
    assert_eq!(c.thermal_cases, sorted);
    assert!(
        c.thermal_cases.iter().any(|n| n == "hot"),
        "{:?}",
        c.thermal_cases
    );
}
