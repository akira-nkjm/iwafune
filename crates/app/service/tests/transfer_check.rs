//! `propulsion.transfer_check` を service 経由で: 日付つきの遷移が無ければ空の結果（不足の入力でも誤りでもない）、
//! 有れば軌跡の系列に表示名・表示単位が付き、注記の語彙が引ける。暦が DE440 かどうかは環境で変わるので、
//! 判定の有無は見ない（判定の規則は sat-analysis、暦ごとの結果の形は sat-astro のテスト）。
use fune_model::design::{
    CentralBodyKind, Maneuver, ManeuverCategory, ManeuverConstraints, ManeuverMethod,
    ManeuverTarget, ManeuverTransfer,
};
use fune_model::value::Quantity;
use fune_service::dto::{AnalysisRunStatus, KpiKey, Op, RunAnalysisRequest};
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
        delta_v: None,
        target: ManeuverTarget {
            body: Some(CentralBodyKind::Mars),
            perigee_altitude: Some(Quantity::new(300.0, "km")),
            ..ManeuverTarget::default()
        },
        constraints: ManeuverConstraints::default(),
        transfer: Some(ManeuverTransfer {
            departure: Some("2028-12-10T00:00:00Z".into()),
            arrival: Some("2029-09-25T00:00:00Z".into()),
            departure_by: None,
        }),
    }
}

#[test]
fn the_demo_has_no_dated_transfer_so_nothing_applies() {
    let result =
        analysis::run_analysis(&demo(), "propulsion.transfer_check", &Default::default()).unwrap();
    assert_eq!(result.status, AnalysisRunStatus::Ok, "{:?}", result.error);
    assert!(result.missing.is_empty());
    assert_eq!(
        result.output.unwrap()["transfers"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert!(result.checks.is_empty());
    // 何も当てはまらない（日付つきの遷移が無い）ことは、KPI の値なし（行は「—」）で、KPI が無い（「結果なし」）のとは違う
    assert_eq!(result.kpis.len(), 1);
    assert_eq!(result.kpis[0].key, KpiKey::TransferMiss);
    assert!(result.kpis[0].value.is_none());
}

#[test]
fn a_dated_transfer_is_checked_with_labelled_series_and_notes() {
    let req = RunAnalysisRequest {
        ops: vec![Op::CreateManeuver {
            maneuver: mars_maneuver(),
        }],
        ..Default::default()
    };
    let result = analysis::run_analysis(&demo(), "propulsion.transfer_check", &req).unwrap();
    assert_eq!(result.status, AnalysisRunStatus::Ok, "{:?}", result.error);
    let out = result.output.unwrap();
    let t = &out["transfers"][0];
    assert_eq!(t["maneuver"], "MARS");
    let channels = t["series"]["channels"].as_array().unwrap();
    assert_eq!(channels.len(), 7);
    for c in channels {
        assert!(c["label_en"].as_str().is_some_and(|s| !s.is_empty()), "{c}");
        assert_eq!(c["display_unit"], "AU", "{c}");
    }
    // 注記の表示名（語彙は TransferCheckNote）
    let note = &result.labels["/transfers/0/notes/0"];
    assert!(note.label_en.contains("point-mass"), "{}", note.label_en);
    // 量の次元（ずれ・影響圏の半径）
    assert_eq!(result.dims["/transfers/0/miss_distance"], "Length");
    assert_eq!(result.dims["/transfers/0/arrival_soi_radius"], "Length");
    // 代表値は到着のずれ（遷移のうち最大）で、出力の値と一致する
    assert_eq!(result.kpis.len(), 1);
    let miss = result.kpis[0].value.as_ref().unwrap();
    assert_eq!(result.kpis[0].key, KpiKey::TransferMiss);
    assert_eq!(miss.si, t["miss_distance"].as_f64().unwrap());
}

/// 暦の外の日付は、`propulsion.maneuvers`（段階 2）と同じく解析の誤りになる（黙って計算しない）。
#[test]
fn dates_outside_the_ephemeris_fail_like_the_maneuvers_analysis() {
    let mut m = mars_maneuver();
    m.transfer = Some(ManeuverTransfer {
        departure: Some("2200-12-10T00:00:00Z".into()),
        arrival: Some("2201-09-25T00:00:00Z".into()),
        departure_by: None,
    });
    let req = RunAnalysisRequest {
        ops: vec![Op::CreateManeuver { maneuver: m }],
        ..Default::default()
    };
    let ws = demo();
    let a = analysis::run_analysis(&ws, "propulsion.maneuvers", &req).unwrap();
    let b = analysis::run_analysis(&ws, "propulsion.transfer_check", &req).unwrap();
    assert_eq!(a.status, AnalysisRunStatus::Failed);
    assert_eq!(b.status, AnalysisRunStatus::Failed);
    assert_eq!(a.error, b.error);
    assert!(b.error.unwrap().contains("outside the ephemeris"));
}
