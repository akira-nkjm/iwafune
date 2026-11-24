//! 重い任意の出力（`RunAnalysisRequest.detail`）: 既定では付かず、頼むと解析が持つものだけが付く。

use fune_service::dto::RunAnalysisRequest;
use fune_service::{Workspace, analysis};
use sat_analysis::structure::ModalFemShapes;
use std::path::Path;

fn demo() -> Workspace {
    Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
        .expect("demo")
}

fn run(ws: &Workspace, id: &str, detail: bool) -> fune_service::dto::AnalysisRunResult {
    let req = RunAnalysisRequest {
        detail,
        ..RunAnalysisRequest::default()
    };
    analysis::run_analysis(ws, id, &req).expect("runs")
}

#[test]
fn shapes_are_returned_only_when_requested() {
    let ws = demo();
    let id = "structure.modal_fem";
    let plain = run(&ws, id, false);
    assert!(plain.detail.is_none());
    let json = serde_json::to_value(&plain).unwrap();
    assert!(
        json.get("detail").is_none(),
        "the default JSON has no detail"
    );

    let with = run(&ws, id, true);
    let shapes: ModalFemShapes = serde_json::from_value(with.detail.clone().expect("detail"))
        .expect("the type named by detail_type");
    let nodes = plain.output.as_ref().unwrap()["model"]["nodes"]
        .as_u64()
        .unwrap();
    assert_eq!(shapes.mesh.nodes.len() as u64, nodes);
    assert!(!shapes.modes.is_empty());
    assert_eq!(with.output, plain.output, "the output does not change");
}

#[test]
fn the_catalog_names_the_detail_type() {
    let entries = analysis::analysis_catalog().expect("catalog");
    let with: Vec<_> = entries
        .iter()
        .filter(|e| e.detail_type.is_some())
        .map(|e| (e.info.id.as_str(), e.detail_type.as_deref().unwrap()))
        .collect();
    assert_eq!(
        with,
        [
            ("structure.modal_fem", "ModalFemShapes"),
            ("thermal.steady", "ThermalPlateTemperatures"),
            ("thermal.transient", "ThermalPlateTemperatures"),
        ]
    );
}

/// 板に分けていない設計（デモ）では、熱の解析は詳細を頼まれても詳細を付けず、出力も変わらない。
#[test]
fn a_design_without_subdivision_has_no_thermal_detail() {
    let ws = demo();
    for id in ["thermal.steady", "thermal.transient"] {
        let plain = run(&ws, id, false);
        let with = run(&ws, id, true);
        assert!(with.detail.is_none(), "{id}");
        assert_eq!(with.output, plain.output, "{id}");
    }
}
