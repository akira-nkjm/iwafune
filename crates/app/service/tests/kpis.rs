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
        ("power.mode_load", &[(KpiKey::ModeLoad, "18.7 W")]),
        (
            "power.orbit_generation",
            // #147 で日照率を足した（軌道 1 周の日照の時間の割合）。
            // #424 部品どうしの影: 展開したパドルが +Y 面のセル（SP_PY）に落とす影と、太陽センサーが +X・−X・−Y 面のセルに落とす影で、
            // 28.3 W → 27.1 W（日照率は変わらない）
            &[
                (KpiKey::OrbitGeneration, "27.1 W"),
                (KpiKey::SunlitFraction, "78.4 %"),
            ],
        ),
        // #106 から、最長継続時間を持つモード（撮像・ダウンリンク）は定常の上でその時間だけ続けたときで、
        // モードの並びも入れて選ぶ（以前はダウンリンクを丸 1 日続けた -5.09 W・100 %）。
        // #138 から撮像は日照中だけ（`when = "sunlit"`）で、1 周の軌道平均の収支は 15.8 W → 15.0 W（最悪の撮像が日陰でなくなる）。
        // 最大 DoD は継続時間の長いダウンリンク（指定なし）のまま。
        // #424 部品どうしの影（発電が 28.3 → 27.1 W に下がる）で、軌道平均の収支は 15.0 W → 14.2 W、最大 DoD は 15.1 % のまま
        (
            "power.energy_balance",
            &[(KpiKey::OrbitBalance, "13.6 W"), (KpiKey::MaxDod, "17.0 %")],
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
                // #424 部品どうしの影（外面の直達日射に影を数える）: 高温のケースで +Y 面のセルが展開パドルの影になり、
                // 最高温度 39.3 → 38.0 °C（最低温度は影の無い低温のケースなので -21.9 °C のまま）。
                // #446: 展開パドルが熱の外面（裏面と、セルの面）を持ち、遮った日射を吸収して取付から機体へ熱を伝えるので、
                // 低温・高温のどのケースも暖かくなる（最高 38.0 → 40.3 °C、最低 -21.9 → -17.2 °C。展開形態のパドルの外面
                // （裏面とセルの面）の吸収は、軌道平均で高温の約 62 W、低温の約 56 W）。覆われた面積（センサー・スラスタの下）は環境とやり取りしない
                // #472: パネルの上のセルが自分の外面を持つので、パネルの面積を二度（吸収も放射も）数えない。パネルの放射の面積が約半分
                // （ε·A が +X のパネルとセルで 0.140 → 0.070 m²）になり、機体が暖かくなる。そこでデモの熱設計を見直した: セルの吸収を電気出力を引いた
                // 正味（α 0.91 → 0.6）で数え、電池を断熱してヒータの設定温度を 20〜25 °C にし、カメラと +Z 面の結合と白塗装で冷やした。
                // 最高 40.3 → 29.5 °C（高温の +X のパネル）、最低 -17.2 → -13.7 °C（低温の -X のパネル）。低温のケースでは電池・カメラのヒータが容量いっぱい（飽和）
                (KpiKey::MaxTemperature, "29.5 °C"),
                (KpiKey::MinTemperature, "-10.9 °C"),
                (KpiKey::NodesInRange, "—"),
                (KpiKey::MaxHeaterDuty, "70.9 %"),
            ],
        ),
        (
            "thermal.transient",
            &[
                // #424 部品どうしの影: 最高温度 58.0 → 57.9 °C、ヒータの最大のデューティ 41.5 → 60.0 %（影で外面の入熱が減って全体が冷え、
                // BAT のヒータ HTR_BAT が多く働く。どちらも判定は ok）。
                // #446: 展開パドルが遮った日射を吸収して機体へ熱を伝えるので、全体が暖かくなり、最高温度 57.9 → 58.3 °C、
                // ヒータの最大のデューティ 60.0 → 30.5 %（ヒータが働く時間が減る）
                // #472: パネルの上のセルの面積を二度数えなくなり、デモの熱設計を見直した（セルの吸収は正味 α 0.6、電池のヒータの設定温度は 20〜25 °C）:
                // 最高 58.3 → 45.9 °C（+X のパネル）、ヒータの最大のデューティ 30.5 → 54.3 %（電池のヒータが設定温度を守るので多く働く）
                (KpiKey::TransientMaxTemperature, "45.9 °C"),
                (KpiKey::TransientMaxHeaterDuty, "36.1 %"),
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
            // #472: ±Y のセルを ±X と同じく 0.5 mm 持ち上げたので、投影面積の平均が 1082 → 1083 cm²（箱が 0.5 mm 外へ出る分）
            &[(KpiKey::MeanProjectedArea, "1083 cm²")],
        ),
        // 有限要素の固有値解析（#370）の最低の固有振動数（剛体を除く最初のモード）。解析の出力の最初のモードの値
        // 値を 70.0 Hz から 145 Hz に更新（#370）: デモのフレーム（BUS_FRAME）に部材の断面（member_width 8 mm・member_wall 1.5 mm）を足し、
        // 箱の 12 辺が梁の部材になって、フレームに付いた部品は MX・PX のパネルに半分ずつ載る代わりに部材につながるようになった
        // （MX・PX の重い部品による低い局所のモードが無くなる）。質量は変わらない（5.548 kg）
        ("structure.modal_fem", &[(KpiKey::FirstFrequency, "145 Hz")]),
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

/// 判定を数える新しい KPI（重なりの組・遮られた円錐）の数。デモの値を出力（`structure.interference` の重なり 6 組 /
/// 判定した 573 組のうち、承知済みの 1 組（ヒンジの縁）を除いた警告 5 組。パネルの角 12 組は縁の接触で、重なりに数えない。
/// `structure.cone_clearance` の遮られた円錐 2 / 判定した 7）と手で突き合わせた。
#[test]
fn demo_new_analyses_kpis_count_checks() {
    let ws = demo();
    let overlaps = &kpis(&ws, "structure.interference", None)[0];
    assert_eq!(
        overlaps.tally,
        Some(KpiTally {
            checked: 573,
            flagged: 5,
            acknowledged: Some(1)
        })
    );
    assert_eq!(overlaps.verdict, Some(Verdict::Warning));
    let cones = &kpis(&ws, "structure.cone_clearance", None)[0];
    assert_eq!(
        cones.tally,
        Some(KpiTally {
            checked: 7,
            flagged: 2,
            acknowledged: Some(0)
        })
    );
    assert_eq!(cones.verdict, Some(Verdict::Warning));
}

/// 概要のカードに使う判定・塗りの比・判定の数え方。判定は解析の判定（節点と温度範囲の種類の組 13 件のうち 2 件が、
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
            // #424 部品どうしの影: 高温のケースでカメラ（CAM）の性能の範囲の上限の警告が無くなる（最高 21.3 → 18.6 °C）ので 4 → 3。
            // #446: 展開パドルが遮った日射を吸収するようになり、CAM が再び上限を超える（最高 24.0 °C 前後）ので 3 → 4
            // #472: パネルの二重の放射を無くし、デモの熱設計を見直した（電池のヒータを 20〜25 °C に）ので、違反は無くなり、範囲の端（警告）は
            // 低温のケースの電池の性能（ヒータが容量いっぱい）と、カメラの性能（冷たい側）の 2 組だけ（4 → 3 → 2）
            // #491: 電池のヒータが設定温度を保てるようになり、電池の警告が消えてカメラの 1 組だけ（2 → 1）
            flagged: 1,
            acknowledged: None
        })
    );
    // 当てはまる割合は（判定した数 − 印の付いた数）/ 判定した数: 上の 1 に合わせて 12/13（#424 で 10/13 に、#446 で 9/13 に戻り、#472 で 11/13、#491 で 12/13）
    assert!((range.fill.unwrap().ratio - 12.0 / 13.0).abs() < 1e-12);
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
    // #491: ヒーターの増分 (3−2)×0.4 + (7−1)×0.3 = 2.2 W に成長余裕 5 % で 4.04 → 6.35 W
    assert_eq!(display(safe), "6.35 W");
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
            KpiKey::FirstFrequency,
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

/// 幾何の所見の 3D の色: 所見に関わる部品の状態は service が決める。デモの干渉は、承知済みの 1 組（ヒンジの縁）の部品が
/// `acknowledged`、承知でない 5 組（太陽センサと太陽電池パネル・`EPT` とパネル）の部品が `open`。形態 STOWED の円錐は、
/// 担い手 `SS_PX`・`STT` と遮る部品が `open`。ほかの解析は欄を出さない。
#[test]
fn geometry_findings_return_the_state_per_part() {
    use fune_service::dto::FindingState::{Acknowledged, Open};
    let ws = demo();
    let run = |id: &str, configuration: Option<&str>| {
        let req = RunAnalysisRequest {
            configuration: configuration.map(str::to_owned),
            ..Default::default()
        };
        analysis::run_analysis(&ws, id, &req).unwrap()
    };
    let state = |r: &fune_service::dto::AnalysisRunResult, part: &str| {
        r.part_findings
            .as_ref()
            .expect("part_findings")
            .iter()
            .find(|p| p.part == part)
            .map(|p| p.state)
    };
    let overlaps = run("structure.interference", None);
    // パネルの角は縁の接触（所見ではない）なので、パネルは色が付かない
    assert_eq!(state(&overlaps, "PANEL_MX"), None);
    assert_eq!(state(&overlaps, "SAP_PY"), Some(Acknowledged));
    // `SP_PY` はヒンジの縁（承知済み）と太陽センサ `SS_PY`（承知でない）の両方に関わるので `open`
    assert_eq!(state(&overlaps, "SP_PY"), Some(Open));
    assert_eq!(state(&overlaps, "EPT"), Some(Open));
    assert_eq!(state(&overlaps, "CAM"), None);
    let stowed = run("structure.cone_clearance", Some("STOWED"));
    assert_eq!(state(&stowed, "SS_PX"), Some(Open));
    assert_eq!(state(&stowed, "SAP_PY"), Some(Open));
    assert_eq!(state(&stowed, "STT"), Some(Open));
    assert!(run("power.mode_load", None).part_findings.is_none());
}
