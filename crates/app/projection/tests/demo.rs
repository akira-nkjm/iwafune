//! デモの設計データ（fixtures/demo-6u）から、すべての解析の入力を組み立てて実行できるか。

use fune_model::design::CommonSettings;
use fune_projection::{Context, ProjectionError, entries, project_json};
use std::path::Path;

fn demo() -> (fune_model::Design, fune_model::Resolved) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    let snap = fune_store::load(&root).expect("デモを読めること");
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

/// デモに -Z パネルのスラスタ THR_1（+Y 向き、質量 50 g）を足す Op。
fn thruster_ops() -> Vec<fune_ops::Op> {
    use fune_model::value::{Value, Vector3};
    use fune_ops::Op;
    let v = |t: &str, d: serde_json::Value| -> Value {
        serde_json::from_value(serde_json::json!({ "type": t, "data": d })).unwrap()
    };
    let scalar = |value: f64, unit: &str| {
        v(
            "scalar",
            serde_json::json!({ "value": value, "unit": unit }),
        )
    };
    let set = |attr: &str, value: Value| Op::SetValue {
        part: "THR_1".into(),
        attr: attr.into(),
        state: None,
        value,
        from: None,
    };
    vec![
        Op::CreatePart {
            id: "THR_1".into(),
            group: Some("aocs".into()),
            catalog: None,
            kind: Some("thruster".into()),
            count: None,
            label: None,
            mode_state: None,
        },
        set("mass", scalar(50.0, "g")),
        set("thrust", scalar(10.0, "mN")),
        set("isp", scalar(60.0, "s")),
        set("direction", Value::Vec3(Vector3::unitless(0.0, 1.0, 0.0))),
        Op::Place {
            part: "THR_1".into(),
            on: Some("PANEL_MZ".into()),
            point: None,
            position: Some(Vector3 {
                x: 40.0,
                y: 0.0,
                z: 0.0,
                unit: Some("mm".into()),
                frame: None,
            }),
            orientation: None,
        },
    ]
}

/// スラスタを足すと、取付をたどった機体座標の位置・向きと、重心が入力に入る。
#[test]
fn unload_time_projects_thrusters_in_body_frame() {
    use fune_ops::{ApplyOptions, apply};
    use sat_astro::math::Vec3;
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("demo-6u");
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"),
        &root,
    );
    apply(&root, &thruster_ops(), &ApplyOptions::default()).unwrap();
    let snap = fune_store::load(&root).unwrap();
    let resolved = fune_model::resolve(&snap.design);
    let ctx = Context::new(&snap.design, &resolved, None).unwrap();
    let input = fune_projection::aocs::unload_time(&ctx).unwrap();
    let t = &input.thrusters[0];
    let (origin, q) = fune_projection::pose::body_pose(&resolved, "PANEL_MZ").unwrap();
    let expected = origin + q.rotate(Vec3::new(0.04, 0.0, 0.0));
    assert!(
        (t.position - expected).norm() < 1e-12,
        "{:?} vs {expected:?}",
        t.position
    );
    assert!((t.direction - q.rotate(Vec3::new(0.0, 1.0, 0.0))).norm() < 1e-12);
    assert!((t.thrust.value() - 0.010).abs() < 1e-15);
    assert!(input.center_of_mass.is_some());
    let out =
        sat_analysis::catalog::run_json("aocs.unload_time", serde_json::to_value(&input).unwrap())
            .unwrap();
    assert!(out["rcs"].is_object(), "{out}");
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
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
    assert_eq!(fune_projection::parts::count_of(part), 1);
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
