//! スラスタの射影: RCS の入力（アンロード時間・1 日の推薬）と、モードで噴射するスラスタの推力誤差。
//! デモ（fixtures/demo-6u）の写しにスラスタを足して確かめる。

use fune_model::design::StateAssign;
use fune_projection::Context;
use std::path::Path;

/// デモの写しにスラスタ THR_1 を足して読み込む。`tmp` は写しを置く場所。
fn demo_with_thruster(tmp: &Path) -> fune_model::Design {
    use fune_ops::{ApplyOptions, apply};
    let root = tmp.join("demo-6u");
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"),
        &root,
    );
    apply(&root, &thruster_ops(), &ApplyOptions::default()).unwrap();
    fune_store::load(&root).unwrap().design
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
            group_under: vec![],
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
    use sat_astro::math::Vec3;
    let tmp = tempfile::tempdir().unwrap();
    let design = demo_with_thruster(tmp.path());
    let resolved = fune_model::resolve(&design);
    let ctx = Context::new(&design, &resolved, None).unwrap();
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

/// MISSION モードで時間の 10 % 噴射すると、そのモードの RW の入力に推力誤差（噴射の割合と重心）が入り、
/// 解析の擾乱の種類に推力誤差が出る。スラスタがあれば RCS で捨てる 1 日の推薬と ΔV 換算も出る。
#[test]
fn rw_unloading_sees_thrust_misalignment_in_firing_modes() {
    let tmp = tempfile::tempdir().unwrap();
    let mut design = demo_with_thruster(tmp.path());
    let mut assign = StateAssign::new("off");
    assign.duty.insert("on".to_owned(), 0.1);
    design
        .modes
        .get_mut("MISSION")
        .unwrap()
        .states
        .insert("THR_1".into(), assign);
    let resolved = fune_model::resolve(&design);
    let ctx = Context::new(&design, &resolved, Some("hot_imaging")).unwrap();
    let mp = fune_projection::structure::summary(&ctx, &mut fune_projection::MissingInputs::new())
        .unwrap();
    let input = fune_projection::aocs::rw_unloading_for_mode(&ctx, Some("MISSION")).unwrap();
    let thrust = input.spacecraft.thrust.as_ref().unwrap();
    assert_eq!(thrust.firing.len(), 1);
    assert!((thrust.firing[0].firing_fraction.value() - 0.1).abs() < 1e-15);
    assert!((thrust.center_of_mass - mp.center_of_mass).norm() < 1e-15);
    let rcs = input.rcs.as_ref().unwrap();
    assert!((rcs.mass.value() - mp.total_mass.value()).abs() < 1e-12);
    let out =
        sat_analysis::catalog::run_json("aocs.rw_unloading", serde_json::to_value(&input).unwrap())
            .unwrap();
    let kinds: Vec<&str> = out["disturbances"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["key"].as_str().unwrap())
        .collect();
    assert_eq!(kinds.last(), Some(&"thrust"));
    assert!(
        out["rcs_unloading"]["momentum_per_day"].as_f64().unwrap() > 0.0,
        "{out}"
    );
    // 噴射しないモードでは推力誤差を入れない（RCS の推薬は求める）
    let safe = fune_projection::aocs::rw_unloading_for_mode(&ctx, Some("SAFE")).unwrap();
    assert!(safe.spacecraft.thrust.is_none());
    assert!(safe.rcs.is_some());
}
