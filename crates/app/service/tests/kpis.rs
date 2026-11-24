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

/// 系のページの数値帯に並ぶ値（解析の一覧の順）。ケースなしで回す。
/// 発電・RW のアンロード・センサの視野・熱の非定常は既定のケース（hot_imaging、MISSION モード）を使う。
/// #24 より前は姿勢則の表の最初（DOWNLINK）を黙って使っていたので、この 4 つの値はそこから変わった。
#[test]
fn demo_kpis_show_the_same_values_as_before() {
    let ws = demo();
    let expected: &[(&str, &[(KpiKey, &str)])] = &[
        ("power.mode_load", &[(KpiKey::ModeLoad, "17.4 W")]),
        (
            "power.orbit_generation",
            // #147 で日照率を足した（軌道 1 周の日照の時間の割合）
            &[
                (KpiKey::OrbitGeneration, "28.3 W"),
                (KpiKey::SunlitFraction, "78.4 %"),
            ],
        ),
        // #106 から、最長継続時間を持つモード（撮像・ダウンリンク）は定常の上でその時間だけ続けたときで、
        // モードの並びも入れて選ぶ（以前はダウンリンクを丸 1 日続けた -5.09 W・100 %）。
        // #138 から撮像は日照中だけ（`when = "sunlit"`）で、1 周の軌道平均の収支は 15.8 W → 15.0 W（最悪の撮像が日陰でなくなる）。
        // 最大 DoD は継続時間の長いダウンリンク（指定なし）のまま
        (
            "power.energy_balance",
            &[(KpiKey::OrbitBalance, "15.0 W"), (KpiKey::MaxDod, "15.1 %")],
        ),
        // #388 段階 4: 空力・太陽輻射圧の面積と腕を部品の箱から出す（デモの `cp_offset_*` は消した）。16.2 h → 17.6 h、
        // 2.39 → 2.25 mN·m·s。合計は残留磁気トルク（約 8e-7 N·m）が支配し、空力は 1.9e-8 → 5.9e-9 N·m、
        // 太陽輻射圧は 4.5e-9 → 8.8e-8 N·m（+X 側の面積 約 0.145〜0.166 m² × Cr 1.3 × P 4.5e-6 N/m² × 腕 約 0.09 m
        // ≈ 8e-8 N·m と桁が合う）。値そのものの手計算は無理なので、面積と腕の単体の手計算は
        // `disturbance/surface/tests.rs` と `rw_unloading/geometry_tests.rs` が持つ。
        (
            "aocs.rw_unloading",
            &[
                (KpiKey::UnloadInterval, "17.6 h"),
                (KpiKey::PeakMomentum, "2.25 mN·m·s"),
            ],
        ),
        // #168 から RCS のアンロード時間も KPI にしたが、デモにスラスタは無いので出ない
        ("aocs.unload_time", &[(KpiKey::MtqUnloadTime, "25.4 min")]),
        ("aocs.sensor_fov", &[(KpiKey::SensorIntrusion, "100 %")]),
        // −X 側の太陽センサーの置き直し（#374）で I_xx が 0.11236 → 0.11226 kg·m² になり、30.06 → 30.04 s（表示 30.1 → 30.0 s）
        ("aocs.slew_time", &[(KpiKey::SlewTime, "30.0 s")]),
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
                (KpiKey::MaxTemperature, "39.3 °C"),
                (KpiKey::MinTemperature, "-21.9 °C"),
                (KpiKey::NodesInRange, "—"),
                // 冷たいケースではヒータが容量いっぱい（飽和）
                (KpiKey::MaxHeaterDuty, "100 %"),
            ],
        ),
        (
            "thermal.transient",
            &[
                (KpiKey::TransientMaxTemperature, "58.0 °C"),
                (KpiKey::TransientMaxHeaterDuty, "41.5 %"),
            ],
        ),
        (
            "structure.mass_properties",
            &[(KpiKey::TotalMass, "5.55 kg")],
        ),
        // 取付ごとの最小の余裕（比）。以前は次元を引けずに無次元の 11.2 と出ていた
        (
            "structure.static_load",
            &[(KpiKey::MinSafetyMargin, "1003 %")],
        ),
        (
            "structure.modal_estimate",
            &[(KpiKey::MinFrequency, "74.8 Hz")],
        ),
        // 構造の新しい解析（#384 干渉・#397 円錐の遮り・#388 外形・打ち上げ適合）の代表値。
        // 干渉と円錐の遮りは判定を数える KPI なので値は「—」で、数は `demo_new_analyses_kpis` が見る
        // 打ち上げ適合の最小の余裕: 外形が収納形態の全部品の箱になり（#374）、y が最小。手計算: 245 / 239 − 1 = 2.51 %
        // （x は 125 / 116.7 − 1 = 7.1 %、z は 380 / 369 − 1 = 3.0 %、質量は 12 / 6.30 − 1 = 90 %）
        (
            "structure.launch_fit",
            &[(KpiKey::LaunchFitMargin, "2.51 %")],
        ),
        (
            "structure.interference",
            &[(KpiKey::InterferenceOverlaps, "—")],
        ),
        (
            "structure.cone_clearance",
            &[(KpiKey::ConeObstructions, "—")],
        ),
        (
            "structure.external_geometry",
            &[(KpiKey::MeanProjectedArea, "1082 cm²")],
        ),
        // 軌道の幾何（区間のうち地球までの最大距離。デモは地球周回なので軌道半径）
        (
            "orbit.geometry",
            &[(KpiKey::GreatestEarthDistance, "6928 km")],
        ),
        // 日付つきの遷移も窓も無いデモでは「当てはまるものが無い」で、値は「—」（KPI が無いのとは違う）
        ("propulsion.transfer_check", &[(KpiKey::TransferMiss, "—")]),
        (
            "propulsion.launch_window",
            &[(KpiKey::LaunchWindowDeltaV, "—")],
        ),
        // 推進系（デモの電気推進スラスタで 550 → 600 km の低推力の上昇と、近地点 250 km への離脱）
        (
            "propulsion.maneuvers",
            &[(KpiKey::LongestTransfer, "35.3 h")],
        ),
        ("propulsion.dv_budget", &[(KpiKey::DvWithMargin, "138 m/s")]),
        (
            "propulsion.propellant",
            &[
                (KpiKey::LoadedPropellant, "67.2 g"),
                (KpiKey::TankVolumeMargin, "66.6 %"),
            ],
        ),
        (
            "propulsion.thruster_usage",
            &[(KpiKey::ImpulseMargin, "226 %")],
        ),
    ];
    for (id, want) in expected {
        let values = kpis(&ws, id, None);
        assert!(!values.is_empty(), "{id}");
        let got: Vec<(KpiKey, String)> = values
            .iter()
            .map(|k| (k.key, display(k).to_owned()))
            .collect();
        let want: Vec<(KpiKey, String)> = want.iter().map(|(k, d)| (*k, (*d).to_owned())).collect();
        assert_eq!(got, want, "{id}");
    }
}

/// 判定を数える新しい KPI（重なりの組・遮られた円錐）の数。デモの値を出力（`structure.interference` の重なり 18 組 /
/// 判定した 573 組、`structure.cone_clearance` の遮られた円錐 2 / 判定した 7）と手で突き合わせた。
#[test]
fn demo_new_analyses_kpis_count_checks() {
    let ws = demo();
    let overlaps = &kpis(&ws, "structure.interference", None)[0];
    assert_eq!(
        overlaps.tally,
        Some(KpiTally {
            checked: 573,
            flagged: 18
        })
    );
    assert_eq!(overlaps.verdict, Some(Verdict::Warning));
    let cones = &kpis(&ws, "structure.cone_clearance", None)[0];
    assert_eq!(
        cones.tally,
        Some(KpiTally {
            checked: 7,
            flagged: 2
        })
    );
    assert_eq!(cones.verdict, Some(Verdict::Warning));
}

/// 概要のカードに使う判定・塗りの比・判定の数え方。判定は解析の判定（節点と温度範囲の種類の組 13 件のうち 4 件が、
/// 不確かさ 5 K を含めて範囲の外か端（ヒータのサーモスタットで冷たいケースの下限側が持ち上がる）。判定は上限側・下限側に分かれるが、数えるのは組ごとに 1 つ（両側の悪いほう）。温度範囲はケースのモードでの部品の電源の状態で選ぶので、電源を切るモードの
/// CAM の動作・性能の範囲は違反にならない）。
#[test]
fn demo_verdicts_fills_and_tallies() {
    let ws = demo();
    let eb = kpis(&ws, "power.energy_balance", None);
    assert_eq!(eb[1].verdict, Some(Verdict::Ok));
    let dod = eb[1].fill.unwrap();
    assert_eq!(dod.basis, FillBasis::Value);
    assert!((dod.ratio - eb[1].value.as_ref().unwrap().si).abs() < 1e-12);
    let storage = &kpis(&ws, "cdh.data_budget", None)[2];
    assert_eq!(
        storage.fill.unwrap().ratio,
        0.0,
        "a negative margin is clamped to 0"
    );
    assert_eq!(storage.verdict, Some(Verdict::Violation));
    let link = &kpis(&ws, "comm.link_budget", None)[0];
    assert_eq!((link.verdict, link.fill), (Some(Verdict::Ok), None));
    let range = &kpis(&ws, "thermal.steady", None)[2];
    assert_eq!(
        range.tally,
        Some(KpiTally {
            checked: 13,
            flagged: 4
        })
    );
    assert!((range.fill.unwrap().ratio - 9.0 / 13.0).abs() < 1e-12);
    // 判定の対象でない KPI（発生量・角運動量）は判定を持たない
    assert!(kpis(&ws, "cdh.data_budget", None)[0].verdict.is_none());
    assert!(kpis(&ws, "aocs.rw_unloading", None)[1].verdict.is_none());
    // バジェットの塗り（成長余裕込みの合計 / 上限。上限を超える行は 1 に寄せる）
    let mass = budget::budget(&ws, BudgetKind::Mass, &BudgetRequest::default()).unwrap();
    assert_eq!(
        mass.system,
        System::Structure,
        "the mass budget is shown under the structure subsystem"
    );
    let fill = mass.fill.unwrap();
    assert_eq!(fill.basis, FillBasis::UsedOverLimit);
    // 推進系の部品（EPT 0.40 kg × 1.10・XE_TANK 0.15 kg × 1.05）の分だけ以前の 5.6374 kg から増えた
    assert!((fill.ratio - 6.2349 / 12.0).abs() < 1e-9);
    let power = budget::budget(&ws, BudgetKind::Power, &BudgetRequest::default()).unwrap();
    assert!(
        power
            .rows
            .iter()
            .any(|r| r.fill.is_some_and(|f| f.ratio == 1.0))
    );
}

/// ケース未指定なら運用全体、明示なら同じケースを選んだバジェットのモードと一致する。
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
            KpiKey::MinFrequency,
            KpiKey::LaunchFitMargin,
            KpiKey::InterferenceOverlaps,
            KpiKey::ConeObstructions,
            KpiKey::MeanProjectedArea,
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

/// 実行の結果は概要の解析の表の行を持つ: 主な結果は KPI の先頭（定義の順に 3 つまで）、判定の文は持ち主ごとに全部。
/// 棒グラフと隠す表は系ごとの組み立て（電源の 4 つの解析）だけが足すので、ほかの解析では null と空。
/// 概要の行に棒グラフを持つ解析（電源の 4 つ）。
const POWER_CHARTS: [&str; 4] = [
    "power.mode_load",
    "power.orbit_generation",
    "power.energy_balance",
    "power.sizing",
];

#[test]
fn every_run_result_carries_its_overview_row() {
    let ws = demo();
    for entry in analysis::analysis_catalog().unwrap() {
        let r =
            analysis::run_analysis(&ws, &entry.info.id, &RunAnalysisRequest::default()).unwrap();
        let n = r.kpis.len().min(3);
        assert_eq!(r.overview.main, r.kpis[..n], "{}", r.id);
        let grouped: usize = r.overview.checks.iter().map(|g| g.checks.len()).sum();
        assert_eq!(grouped, r.checks.len(), "{}", r.id);
        let power = POWER_CHARTS.contains(&r.id.as_str());
        assert_eq!(r.overview.chart.is_some(), power, "{}", r.id);
        if r.id != "power.energy_balance" {
            assert!(r.overview.hidden_tables.is_empty(), "{}", r.id);
        }
    }
}

/// 熱の 3D の色: 部品ごとの代表の温度は service が選ぶ（節点と部品の対応・全ケースの最高・ケース・判定）。
/// ほかの解析は欄を出さない。
#[test]
fn thermal_steady_returns_the_representative_temperature_per_part() {
    let ws = demo();
    let run = |id| analysis::run_analysis(&ws, id, &RunAnalysisRequest::default()).unwrap();
    let result = run("thermal.steady");
    let heat = result.part_heat.expect("part_heat");
    assert!(!heat.is_empty());
    let output: sat_analysis::thermal::ThermalSteadyOutput =
        serde_json::from_value(result.output.unwrap()).unwrap();
    for h in &heat {
        let env = output.envelope.iter().find(|e| e.node == h.node).unwrap();
        assert_eq!(h.temperature.si, env.max.value());
        assert_eq!(h.case, env.max_case);
        assert_eq!(h.verdict, env.verdict);
    }
    assert!(run("power.mode_load").part_heat.is_none());
}
