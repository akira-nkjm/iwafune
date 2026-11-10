//! KPI（系の代表値・判定・塗りの比・判定の数え方）は service が決め、CLI と Web は並べるだけにする。
//! デモの値は、KPI の選び方を Web に持っていたときの表示と同じ（最小の安全余裕だけは、次元を比（%）で引くように直した）。
use fune_service::dto::{
    BudgetKind, BudgetRequest, FillBasis, KpiKey, KpiTally, KpiView, RunAnalysisRequest, System,
    Verdict,
};
use fune_service::{Workspace, analysis, budget};
use std::path::Path;

fn demo() -> Workspace {
    Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
        .expect("demo")
}

fn kpis(ws: &Workspace, id: &str, case: Option<&str>) -> Vec<KpiView> {
    let req = RunAnalysisRequest {
        case: case.map(str::to_owned),
        ..Default::default()
    };
    analysis::run_analysis(ws, id, &req).unwrap().kpis
}

fn display(k: &KpiView) -> &str {
    k.value.as_ref().map_or("—", |v| v.display.as_str())
}

/// 系のページの数値帯に並ぶ値（解析の一覧の順）。
#[test]
fn demo_kpis_show_the_same_values_as_before() {
    let ws = demo();
    let expected: &[(&str, &[(KpiKey, &str)])] = &[
        ("power.mode_load", &[(KpiKey::ModeLoad, "17.4 W")]),
        (
            "power.orbit_generation",
            &[(KpiKey::OrbitGeneration, "12.3 W")],
        ),
        (
            "power.energy_balance",
            &[(KpiKey::OrbitBalance, "-5.09 W"), (KpiKey::MaxDod, "100 %")],
        ),
        (
            "attitude.rw_unloading",
            &[
                (KpiKey::UnloadInterval, "16.4 h"),
                (KpiKey::PeakMomentum, "2.32 mN·m·s"),
            ],
        ),
        (
            "attitude.unload_time",
            &[(KpiKey::MtqUnloadTime, "25.4 min")],
        ),
        ("attitude.sensor_fov", &[(KpiKey::SensorIntrusion, "100 %")]),
        (
            "cdh.data_budget",
            &[
                (KpiKey::DataPerDay, "2.77 GB"),
                (KpiKey::StorageRequired, "8.30 GB"),
                (KpiKey::StorageMargin, "-3.61 %"),
            ],
        ),
        ("comm.link_budget", &[(KpiKey::LinkMargin, "3.05 dB")]),
        ("comm.visibility", &[(KpiKey::ContactPerDay, "20.0 min")]),
        (
            "comm.data_throughput",
            &[
                (KpiKey::DownlinkPerDay, "301 MB"),
                (KpiKey::MaxBacklog, "2.47 GB"),
            ],
        ),
        (
            "thermal.steady",
            &[
                (KpiKey::MaxTemperature, "50.5 °C"),
                (KpiKey::MinTemperature, "-32.2 °C"),
                (KpiKey::NodesInRange, "—"),
            ],
        ),
        (
            "thermal.transient",
            &[(KpiKey::TransientMaxTemperature, "74.2 °C")],
        ),
        (
            "structure.mass_properties",
            &[(KpiKey::TotalMass, "5.00 kg")],
        ),
        // 取付ごとの最小の余裕（比）。以前は次元を引けずに無次元の 11.2 と出ていた
        (
            "structure.static_load",
            &[(KpiKey::MinSafetyMargin, "1124 %")],
        ),
        (
            "structure.modal_estimate",
            &[(KpiKey::MinFrequency, "74.8 Hz")],
        ),
    ];
    for (id, want) in expected {
        let got: Vec<(KpiKey, String)> = kpis(&ws, id, None)
            .iter()
            .map(|k| (k.key, display(k).to_owned()))
            .collect();
        let want: Vec<(KpiKey, String)> = want.iter().map(|(k, d)| (*k, (*d).to_owned())).collect();
        assert_eq!(got, want, "{id}");
    }
}

/// 概要のカードに使う判定・塗りの比・判定の数え方。判定は解析の判定（9 件の節点のうち 4 件が範囲の外か端）。
#[test]
fn demo_verdicts_fills_and_tallies() {
    let ws = demo();
    let eb = kpis(&ws, "power.energy_balance", None);
    assert_eq!(eb[1].verdict, Some(Verdict::Violation));
    let dod = eb[1].fill.unwrap();
    assert_eq!((dod.ratio, dod.basis), (1.0, FillBasis::Value));
    let storage = &kpis(&ws, "cdh.data_budget", None)[2];
    assert_eq!(storage.fill.unwrap().ratio, 0.0, "負の余裕は 0 に寄せる");
    assert_eq!(storage.verdict, Some(Verdict::Violation));
    let link = &kpis(&ws, "comm.link_budget", None)[0];
    assert_eq!((link.verdict, link.fill), (Some(Verdict::Ok), None));
    let range = &kpis(&ws, "thermal.steady", None)[2];
    assert_eq!(
        range.tally,
        Some(KpiTally {
            checked: 9,
            flagged: 4
        })
    );
    assert!((range.fill.unwrap().ratio - 5.0 / 9.0).abs() < 1e-12);
    // 判定の対象でない KPI（発生量・角運動量）は判定を持たない
    assert!(kpis(&ws, "cdh.data_budget", None)[0].verdict.is_none());
    assert!(
        kpis(&ws, "attitude.rw_unloading", None)[1]
            .verdict
            .is_none()
    );
    // バジェットの塗り（成長余裕込みの合計 / 上限。上限を超える行は 1 に寄せる）
    let mass = budget::budget(&ws, BudgetKind::Mass, &BudgetRequest::default()).unwrap();
    assert_eq!(
        mass.system,
        System::Structure,
        "質量のバジェットは構造の系で見せる"
    );
    let fill = mass.fill.unwrap();
    assert_eq!(fill.basis, FillBasis::UsedOverLimit);
    assert!((fill.ratio - 5.6374 / 12.0).abs() < 1e-9);
    let power = budget::budget(&ws, BudgetKind::Power, &BudgetRequest::default()).unwrap();
    assert!(
        power
            .rows
            .iter()
            .any(|r| r.fill.is_some_and(|f| f.ratio == 1.0))
    );
}

/// 電力のモードの消費電力は、バジェットと同じモード（ケースのモード、無ければ合計が最大のモード）。
#[test]
fn mode_load_uses_the_budget_mode() {
    let ws = demo();
    for case in [None, Some("cold_safe")] {
        let req = BudgetRequest {
            case: case.map(str::to_owned),
            ..Default::default()
        };
        let b = budget::budget(&ws, BudgetKind::Power, &req).unwrap();
        let mode = b.mode.clone().unwrap();
        let row = b.modes.iter().find(|r| r.node.as_deref() == Some(&mode));
        let load = &kpis(&ws, "power.mode_load", case)[0];
        assert_eq!(display(load), row.unwrap().with_growth.display, "{case:?}");
    }
    let safe = &kpis(&ws, "power.mode_load", Some("cold_safe"))[0];
    assert_eq!(display(safe), "4.04 W");
}

/// 解析の一覧は KPI の定義（名前）を、系のページに並べる順で持つ（結果が届く前に名前を並べるため）。
#[test]
fn catalog_lists_kpi_definitions() {
    let catalog = analysis::analysis_catalog().unwrap();
    let structure: Vec<KpiKey> = catalog
        .iter()
        .filter(|e| e.info.id.starts_with("structure."))
        .flat_map(|e| e.kpis.iter().map(|k| k.key))
        .collect();
    assert_eq!(
        structure,
        [
            KpiKey::TotalMass,
            KpiKey::MinSafetyMargin,
            KpiKey::MinFrequency
        ]
    );
    let sunlit = catalog
        .iter()
        .find(|e| e.info.id == "power.solar_sunlit")
        .unwrap();
    assert!(sunlit.kpis.is_empty());
    let dod = catalog
        .iter()
        .flat_map(|e| &e.kpis)
        .find(|k| k.key == KpiKey::MaxDod)
        .unwrap();
    assert_eq!(
        (dod.label.as_str(), dod.label_en.as_str()),
        ("最大 DoD", "Max DoD")
    );
}
