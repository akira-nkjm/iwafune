//! デモの設計データ（fixtures/demo-6u）から、すべての解析の入力を組み立てて実行できるか。

use fune_model::design::CommonSettings;
use fune_projection::{Context, ProjectionError, entries, project_json};
use std::path::Path;

fn demo() -> (fune_model::Design, fune_model::Resolved) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    let snap = fune_store::load(&root).expect("the demo should load");
    let mut design = snap.design;
    design.analysis.common = Some(CommonSettings {
        default_case: Some("hot_imaging".to_owned()),
    });
    let resolved = fune_model::resolve(&design);
    (design, resolved)
}

/// 入力が組み立てられれば解析が通ること。足りなければ、パスで名指しされること。
#[test]
fn every_analysis_projects_or_names_missing_inputs() {
    let (design, resolved) = demo();
    let ctx = Context::new(&design, &resolved, None).unwrap();
    for e in entries() {
        match project_json(e.id, &ctx) {
            Ok(input) => {
                let out = sat_analysis::catalog::run_json(e.id, input);
                assert!(out.is_ok(), "{}: {:?}", e.id, out.err());
            }
            Err(ProjectionError::Missing(m)) => {
                let list = m.into_vec();
                assert!(!list.is_empty(), "{}", e.id);
                assert!(list.iter().all(|x| !x.path.is_empty()), "{}", e.id);
            }
            // 射影の行が `None` の解析（`structure.modal_fem` は第 3 段階まで JSON の入力だけ）。表の側で「作れない」と分かっている。
            Err(ProjectionError::NotImplemented(id)) => {
                assert!(!e.projectable && id == e.id, "{}", e.id);
            }
            Err(other) => panic!("{}: {other}", e.id),
        }
    }
}

/// 軌道を使わない解析と主要な時系列の解析は、デモでそのまま回ること。
#[test]
fn core_analyses_run_on_the_demo() {
    let (design, resolved) = demo();
    for id in [
        "power.mode_load",
        "power.energy_balance",
        "cdh.data_budget",
        "comm.link_budget",
        "structure.mass_properties",
        "structure.static_load",
    ] {
        let ctx = Context::new(&design, &resolved, None).unwrap();
        let input = project_json(id, &ctx).unwrap_or_else(|e| panic!("{id}: {e}"));
        let out = sat_analysis::catalog::run_json(id, input);
        assert!(out.is_ok(), "{id}: {:?}", out.err());
    }
}

/// 姿勢の合成: 取付の根（BUS_FRAME）は機体の原点で恒等。
#[test]
fn root_part_is_body_origin() {
    let (_, resolved) = demo();
    let (p, q) = fune_projection::pose::body_pose(&resolved, "BUS_FRAME").unwrap();
    assert_eq!((p.x, p.y, p.z), (0.0, 0.0, 0.0));
    assert!((q.w - 1.0).abs() < 1e-12);
}

/// 電力収支のケースは、モードごとの姿勢則を使う（SAFE・NOMINAL は太陽指向、MISSION・DOWNLINK は地心指向）。
#[test]
fn energy_balance_cases_use_each_modes_attitude_law() {
    let (design, resolved) = demo();
    let ctx = Context::new(&design, &resolved, None).unwrap();
    let input = fune_projection::power::energy_balance(&ctx).unwrap();
    assert_eq!(input.cases.len(), design.modes.len());
    for (mode, expected) in [
        ("SAFE", "sun_pointing"),
        ("NOMINAL", "sun_pointing"),
        ("MISSION", "nadir_pointing"),
        ("DOWNLINK", "nadir_pointing"),
    ] {
        let case = input.cases.iter().find(|c| c.id == mode).unwrap();
        let law = serde_json::to_value(case.attitude.as_ref().unwrap()).unwrap();
        assert_eq!(law["law"], expected, "{mode}");
    }
}

/// MTQ だけのデモでは RCS の入力は空で、重心も渡さない。
#[test]
fn unload_time_without_thrusters() {
    let (design, resolved) = demo();
    let ctx = Context::new(&design, &resolved, None).unwrap();
    let input = fune_projection::aocs::unload_time(&ctx).unwrap();
    assert!(input.thrusters.is_empty());
    assert!(input.center_of_mass.is_none());
    assert!(!input.magnetorquers.is_empty());
}

/// 割合の合計の許容差は、設計の検査（fune-model）と解析（sat-analysis）で同じ値にする。
/// fune-model は sat-analysis に依存できないので、両方が見えるここで一致を確かめる。
#[test]
fn share_tolerance_matches_between_check_and_analysis() {
    assert_eq!(
        fune_model::check::SHARE_TOLERANCE,
        sat_analysis::modes::DUTY_TOLERANCE
    );
}

/// 要求の「確かめる解析」の検査（fune-model の `ANALYSIS_IDS`）は、解析の登録（sat-analysis の `catalog`）と同じ集合にする。
/// fune-model は sat-analysis に依存できないので、両方が見えるここで一致を確かめる。
#[test]
fn requirement_analysis_ids_match_the_analysis_catalog() {
    let mut registered: Vec<String> = sat_analysis::catalog::catalog()
        .into_iter()
        .map(|i| i.id)
        .collect();
    let mut known: Vec<String> = fune_model::design::ANALYSIS_IDS
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    registered.sort();
    known.sort();
    assert_eq!(known, registered);
}

/// 員数を変更しても単体レートは変えず、count だけを解析へ渡す。
#[test]
fn cdh_projects_per_unit_rates_and_normalizes_counts() {
    let (design, mut resolved) = demo();
    let original =
        fune_projection::cdh::data_budget(&Context::new(&design, &resolved, None).unwrap())
            .unwrap();
    for part in resolved.parts.values_mut() {
        part.count = 3;
    }
    let counted =
        fune_projection::cdh::data_budget(&Context::new(&design, &resolved, None).unwrap())
            .unwrap();
    for (a, b) in original.parts.iter().zip(&counted.parts) {
        assert_eq!(b.count, 3);
        assert_eq!(
            serde_json::to_value(&a.hk_rate).unwrap(),
            serde_json::to_value(&b.hk_rate).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&a.mission_rate).unwrap(),
            serde_json::to_value(&b.mission_rate).unwrap()
        );
    }
    let part = resolved.parts.values_mut().next().unwrap();
    part.count = 0;
    assert_eq!(part.count_or_one(), 1);
    assert_eq!(fune_projection::parts::extensive(part, 2.0), 2.0);
    part.count = 3;
    assert_eq!(fune_projection::parts::extensive(part, 2.0), 6.0);
}

/// ケースの β（`cases/hot_imaging.toml` は 60°）は、時系列の解析だけでなく、地上局のパス（可視・送信量）と
/// アンロード時間の地磁気の平均にも届く。β の指定が無いケースでは β を渡さない。
#[test]
fn case_beta_reaches_every_orbit_sampling() {
    let (mut design, resolved) = demo();
    let mut no_beta = design.cases["hot_imaging"].clone();
    no_beta.id = "no_beta".into();
    no_beta.beta = None;
    design.cases.insert(no_beta.id.clone(), no_beta);
    let beta = |id: &str, case: Option<&str>| -> serde_json::Value {
        let ctx = Context::new(&design, &resolved, case).unwrap();
        let input = project_json(id, &ctx).unwrap_or_else(|e| panic!("{id}: {e}"));
        match id {
            "aocs.unload_time" => input["field"]["sampling"]["beta"].clone(),
            "aocs.sensor_fov" => input["timeline"]["beta"].clone(),
            _ => input["sampling"]["beta"].clone(),
        }
    };
    let sixty = 60f64.to_radians();
    for id in [
        "comm.visibility",
        "comm.data_throughput",
        "aocs.unload_time",
        "aocs.sensor_fov",
    ] {
        let without_beta = (id == "aocs.sensor_fov").then_some("no_beta");
        assert!(beta(id, without_beta).is_null(), "{id}");
        let b = beta(id, Some("hot_imaging")).as_f64().unwrap_or(f64::NAN);
        assert!((b - sixty).abs() < 1e-12, "{id}: {b}");
    }
}

/// 電力収支で既定の姿勢則を借りても、ケースの期間などを全体に流し込まない。
#[test]
fn energy_balance_default_only_selects_the_timeline_attitude() {
    let (mut design, resolved) = demo();
    let baseline =
        fune_projection::power::energy_balance(&Context::new(&design, &resolved, None).unwrap())
            .unwrap();
    let case = design.cases.get_mut("hot_imaging").unwrap();
    case.orbits = Some(17);
    case.mode = Some("SAFE".into());
    let input =
        fune_projection::power::energy_balance(&Context::new(&design, &resolved, None).unwrap())
            .unwrap();
    let baseline = serde_json::to_value(baseline).unwrap();
    let input = serde_json::to_value(input).unwrap();
    assert_eq!(input["timeline"]["attitude"]["law"], "sun_pointing");
    assert!(input["timeline"]["span"].is_number());
    assert_eq!(input["timeline"]["span"], baseline["timeline"]["span"]);
    assert!(input["timeline"]["beta"].is_null());
    assert_eq!(input["cases"], baseline["cases"]);
    design.analysis.common = None;
    let missing =
        fune_projection::power::energy_balance(&Context::new(&design, &resolved, None).unwrap())
            .unwrap_err();
    assert!(!missing.into_vec().is_empty());
}

/// 依存の向きで共有できない設計検査と解析の予備状態の規則を突き合わせる。
#[test]
fn mode_validation_and_analysis_agree_on_population() {
    use fune_model::design::{Mode, Part, Redundancy, StateAssign};
    use sat_analysis::modes::{Population, counted_states};
    let (mut design, _) = demo();
    let mut part = Part::new("TEST_RW");
    part.kind = Some("reaction_wheel".into());
    part.count = 4;
    for (redundancy, active, spare) in [
        (Redundancy::Cold, Some(3), None),
        (Redundancy::Warm, Some(3), None),
        (Redundancy::Warm, Some(3), Some("on")),
        (Redundancy::Cold, Some(5), None),
        (Redundancy::None, Some(3), None),
        (Redundancy::Warm, Some(3), Some("unknown")),
    ] {
        part.redundancy = redundancy;
        design.parts.insert(part.id.clone(), part.clone());
        let mut a = StateAssign::new("on");
        a.active = active;
        a.spare_state = spare.map(str::to_owned);
        let mode = Mode {
            id: "TEST_MODE".into(),
            states: [(part.id.clone(), a)].into(),
            ..Default::default()
        };
        design.modes.insert(mode.id.clone(), mode.clone());
        let states = design.part_states(&part);
        let projected = fune_projection::context::mode_def(&mode);
        let redundancy = match redundancy {
            Redundancy::None => sat_analysis::modes::Redundancy::None,
            Redundancy::Cold => sat_analysis::modes::Redundancy::Cold,
            Redundancy::Warm => sat_analysis::modes::Redundancy::Warm,
        };
        let accepted = counted_states(
            &projected.assignments[0],
            Population {
                count: 4,
                redundancy,
                states,
            },
        )
        .is_ok();
        let errors = fune_model::check(&design).iter().any(|d| {
            d.severity == fune_model::Severity::Error
                && d.target == Some(fune_model::Target::mode("TEST_MODE", Some("TEST_RW")))
        });
        assert_eq!(accepted, !errors, "{active:?} {spare:?}");
    }
    let a = sat_analysis::modes::StateAssignment {
        part: "RW".into(),
        state: "on".into(),
        duty: Default::default(),
        active: Some(3),
        spare_state: None,
    };
    let states = [fune_model::builtin::STATE_OFF.to_owned(), "on".to_owned()];
    let terms = counted_states(
        &a,
        Population {
            count: 4,
            redundancy: sat_analysis::modes::Redundancy::Cold,
            states: &states,
        },
    )
    .unwrap();
    assert_eq!(terms.last().unwrap().0, fune_model::builtin::STATE_OFF);
}

/// 表示順を変えても消費電力の数値はモードごとに同じで、入力・出力の順だけが変わる。
#[test]
fn mode_display_order_preserves_analysis_values() {
    let (mut design, resolved) = demo();
    design.project.modes = Some(fune_model::design::ModeOrder {
        order: ["NOMINAL", "MISSION", "DOWNLINK", "SAFE"]
            .map(str::to_owned)
            .to_vec(),
    });
    let run = |d: &fune_model::Design| {
        let ctx = Context::new(d, &resolved, None).unwrap();
        let input = project_json("power.mode_load", &ctx).unwrap();
        sat_analysis::catalog::run_json("power.mode_load", input).unwrap()
    };
    let ordered = run(&design);
    let modes = ordered["modes"].as_array().unwrap();
    assert_eq!(
        modes
            .iter()
            .map(|m| m["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["NOMINAL", "MISSION", "DOWNLINK", "SAFE"]
    );
    design.project.modes = None;
    let mut sorted = ordered.clone();
    sorted["modes"]
        .as_array_mut()
        .unwrap()
        .sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    assert_eq!(sorted, run(&design));
}

/// 姿勢変更の時間: 角度と要求は `analysis/aocs.toml`、慣性は質量特性、ホイールは RW の部品から。
/// デモ（I_xx = 0.112 kg·m²（推進系の部品込み）、RW 10 mN·m·s × 0.8・1 mN·m、90°）は x 軸が最悪で台形:
/// t = θ I / h + h / τ = (π/2)(0.11226)/0.008 + 8 = 22.04 + 8 = 30.04 s。
/// （ホイールとタンクの置き直し（#384）で I_xx が 0.1100 から 0.1124 に増え、29.6 s から 30.1 s になった。
/// その後、−X 側の太陽センサー（35 g）を他の 3 つと同じ出方にし、y = +80 mm へ移した（#374）ので、I_xx は 0.11236 から 0.11226 に減り、30.04 s になった。）
#[test]
fn slew_time_projects_settings_inertia_and_wheels() {
    use sat_analysis::spec::Analysis;
    let (design, resolved) = demo();
    let ctx = Context::new(&design, &resolved, None).unwrap();
    let input = fune_projection::aocs::slew_time(&ctx).unwrap();
    let mp = fune_projection::structure::summary(&ctx, &mut fune_projection::MissingInputs::new())
        .unwrap();
    assert_eq!(input.inertia, mp.inertia);
    assert!((input.angle.value() - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    assert_eq!(input.required_time.map(|t| t.value()), Some(120.0));
    assert_eq!(input.wheels.len(), 3);
    let out = sat_analysis::aocs::SlewTime::run(&input).unwrap();
    let ixx = mp.inertia.m[0][0];
    let expect = std::f64::consts::FRAC_PI_2 * ixx / 0.008 + 8.0;
    assert_eq!(out.worst.label, "x");
    assert!((out.worst.time.value() - expect).abs() < 1e-9);
    assert!((out.worst.time.value() - 30.04).abs() < 0.005);
}
