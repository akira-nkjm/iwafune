//! `propulsion.launch_window` を service 経由で: 天体間の遷移が無ければ空の結果（不足の入力でも誤りでもない）、
//! 有れば量の次元・列挙の表示名が付き、設定（`[launch_window]`）が射影を通って効き、
//! 暦の外にしか窓が無ければ `propulsion.maneuvers` と同じ種類の誤りになる。
use fune_model::design::{
    CentralBodyKind, Maneuver, ManeuverCategory, ManeuverConstraints, ManeuverMethod,
    ManeuverTarget,
};
use fune_model::value::Quantity;
use fune_service::dto::{AnalysisRunStatus, Op, RunAnalysisRequest};
use fune_service::{Workspace, analysis};
use std::path::Path;

fn demo() -> Workspace {
    Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
        .unwrap()
}

fn mars_maneuver() -> Maneuver {
    Maneuver {
        id: "MARS".into(),
        label: None,
        category: ManeuverCategory::Insertion,
        method: ManeuverMethod::Impulsive,
        thruster: Some("EPT".into()),
        after: Some("DEORBIT".into()),
        via_altitude: None,
        target: ManeuverTarget {
            body: Some(CentralBodyKind::Mars),
            perigee_altitude: Some(Quantity::new(300.0, "km")),
            ..ManeuverTarget::default()
        },
        constraints: ManeuverConstraints::default(),
        transfer: None,
    }
}

fn with_mars() -> RunAnalysisRequest {
    RunAnalysisRequest {
        ops: vec![Op::CreateManeuver {
            maneuver: mars_maneuver(),
        }],
        ..Default::default()
    }
}

#[test]
fn the_demo_has_no_interplanetary_maneuver_so_the_result_is_empty() {
    let result =
        analysis::run_analysis(&demo(), "propulsion.launch_window", &Default::default()).unwrap();
    assert_eq!(result.status, AnalysisRunStatus::Ok, "{:?}", result.error);
    assert!(result.missing.is_empty());
    let out = result.output.unwrap();
    assert_eq!(out["windows"].as_array().unwrap().len(), 0);
    assert_eq!(out["points"].as_array().unwrap().len(), 0);
    assert!(result.checks.is_empty());
}

#[test]
fn a_mars_maneuver_gets_a_grid_labelled_points_and_dimensions() {
    let result = analysis::run_analysis(&demo(), "propulsion.launch_window", &with_mars()).unwrap();
    assert_eq!(result.status, AnalysisRunStatus::Ok, "{:?}", result.error);
    let out = result.output.unwrap();
    let w = &out["windows"][0];
    assert_eq!(w["maneuver"], "MARS");
    assert_eq!(w["grid"]["departures"].as_array().unwrap().len(), 100);
    assert_eq!(w["grid"]["layers"].as_array().unwrap().len(), 5);
    assert!(w["solved_cells"].as_u64().unwrap() > 1000);
    // 量の次元（点の C3・ΔV・飛行時間）と、点の種類の表示名
    assert_eq!(result.dims["/points/0/c3"], "SpecificEnergy");
    assert_eq!(result.dims["/points/0/delta_v"], "Velocity");
    assert_eq!(result.dims["/points/0/time_of_flight"], "Time");
    // 格子の層は次元の名前を 1 つ持つ（値 1 つずつではなく）
    assert_eq!(w["grid"]["layers"][0]["dimension"], "Velocity");
    let kind = &result.labels["/points/0/kind"];
    assert!(kind.label_en.contains("delta-V"), "{}", kind.label_en);
    // 格子の大きな配列には 1 つずつ次元を付けない（出力が大きくならない）
    assert!(
        !result
            .dims
            .keys()
            .any(|k| k.contains("/grid/layers/0/values/")),
        "grid values must not each carry a dimension"
    );
}

#[test]
fn settings_reach_the_analysis_and_bad_ones_are_reported() {
    let ws = demo();
    let mut req = with_mars();
    for (path, value) in [
        ("analysis.propulsion.launch_window.departure_steps", "21"),
        ("analysis.propulsion.launch_window.departure_span", "100 d"),
        (
            "analysis.propulsion.launch_window.departure_start",
            "2028-09-01T00:00:00Z",
        ),
    ] {
        req.ops.push(
            fune_service::edit::set_op(
                &ws,
                &fune_service::edit::SetText {
                    path: path.into(),
                    value: value.into(),
                    ..Default::default()
                },
            )
            .unwrap(),
        );
    }
    let result = analysis::run_analysis(&ws, "propulsion.launch_window", &req).unwrap();
    assert_eq!(result.status, AnalysisRunStatus::Ok, "{:?}", result.error);
    let w = &result.output.unwrap()["windows"][0];
    assert_eq!(w["grid"]["departures"].as_array().unwrap().len(), 21);
    assert_eq!(w["grid"]["departures"][0], "2028-09-01T00:00:00Z");
    assert_eq!(w["grid"]["departures"][20], "2028-12-10T00:00:00Z");
    // 読めない日付は入力の不足として、設定のパスを名指しする
    let mut bad = with_mars();
    bad.ops.push(
        fune_service::edit::set_op(
            &ws,
            &fune_service::edit::SetText {
                path: "analysis.propulsion.launch_window.departure_start".into(),
                value: "next spring".into(),
                ..Default::default()
            },
        )
        .unwrap(),
    );
    let result = analysis::run_analysis(&ws, "propulsion.launch_window", &bad).unwrap();
    assert_eq!(
        result.status,
        AnalysisRunStatus::MissingInputs,
        "{:?}",
        result.error
    );
    assert!(
        result
            .missing
            .iter()
            .any(|m| m.path.contains("launch_window.departure_start"))
    );
}

/// 窓がすべて暦の外なら、`propulsion.maneuvers` の日付つきの遷移と同じ種類の誤り（黙って空にしない）。
#[test]
fn a_window_entirely_outside_the_ephemeris_fails() {
    let ws = demo();
    let mut req = with_mars();
    req.ops.push(
        fune_service::edit::set_op(
            &ws,
            &fune_service::edit::SetText {
                path: "analysis.propulsion.launch_window.departure_start".into(),
                value: "2200-01-01T00:00:00Z".into(),
                ..Default::default()
            },
        )
        .unwrap(),
    );
    let result = analysis::run_analysis(&ws, "propulsion.launch_window", &req).unwrap();
    assert_eq!(result.status, AnalysisRunStatus::Failed);
    assert!(result.error.unwrap().contains("outside the ephemeris"));
}
