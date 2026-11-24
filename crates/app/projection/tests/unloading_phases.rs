//! RCS でのアンロードの区間: ΔV バジェットの入力は、期間のあるフェーズごとに（そのフェーズの軌道・形態で）`aocs.rw_unloading` を評価し、
//! フェーズに期間が無ければ設計の軌道とミッション期間の 1 区間になる。スラスタが無ければ今までと同じ（足さない）。
//! デモの写しに 6 本のスラスタを足して確かめる（デモのデータは変えない）。

use fune_model::Design;
use fune_ops::{ApplyOptions, Op, apply};
use fune_projection::{Context, propulsion};
use sat_analysis::propulsion::{DvBudget, UnloadingNote};
use sat_analysis::spec::Analysis;
use std::path::Path;

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

/// -Z パネルに、機体の ±x・±y・±z まわりのトルクを出す 6 本（推力 10 mN、Isp 60 s、質量 50 g）を足す Op。
fn six_thruster_ops() -> Vec<Op> {
    use fune_model::value::{Value, Vector3};
    let v = |t: &str, d: serde_json::Value| -> Value {
        serde_json::from_value(serde_json::json!({ "type": t, "data": d })).unwrap()
    };
    let scalar = |value: f64, unit: &str| {
        v(
            "scalar",
            serde_json::json!({ "value": value, "unit": unit }),
        )
    };
    // (ID, パネル上の位置 [mm]、推力の向き)
    let table: [(&str, [f64; 2], [f64; 3]); 6] = [
        ("THR_ZP", [40.0, 0.0], [0.0, 1.0, 0.0]),
        ("THR_ZM", [-40.0, 0.0], [0.0, 1.0, 0.0]),
        ("THR_XP", [0.0, 40.0], [0.0, 0.0, 1.0]),
        ("THR_XM", [0.0, -40.0], [0.0, 0.0, 1.0]),
        ("THR_YP", [40.0, 0.0], [0.0, 0.0, 1.0]),
        ("THR_YM", [-40.0, 0.0], [0.0, 0.0, 1.0]),
    ];
    let mut ops = Vec::new();
    for (id, [x, y], d) in table {
        let set = |attr: &str, value: Value| Op::SetValue {
            part: id.into(),
            attr: attr.into(),
            state: None,
            value,
            from: None,
        };
        ops.push(Op::CreatePart {
            id: id.into(),
            group: Some("aocs".into()),
            group_under: vec![],
            catalog: None,
            kind: Some("thruster".into()),
            count: None,
            label: None,
            mode_state: None,
        });
        ops.push(set("mass", scalar(50.0, "g")));
        ops.push(set("thrust", scalar(10.0, "mN")));
        ops.push(set("isp", scalar(60.0, "s")));
        ops.push(set(
            "direction",
            Value::Vec3(Vector3::unitless(d[0], d[1], d[2])),
        ));
        ops.push(Op::Place {
            part: id.into(),
            on: Some("PANEL_MZ".into()),
            point: None,
            position: Some(Vector3 {
                x,
                y,
                z: 0.0,
                unit: Some("mm".into()),
                frame: None,
            }),
            orientation: None,
            configuration: None,
        });
    }
    ops
}

/// デモの写しに 6 本のスラスタを足し、姿勢則の表を 1 つにする（モードを選ばなくても一意に決まるように）。
fn demo_with_rcs(tmp: &Path) -> Design {
    let root = tmp.join("demo-6u");
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"),
        &root,
    );
    apply(&root, &six_thruster_ops(), &ApplyOptions::default()).unwrap();
    let mut d = fune_store::load(&root).unwrap().design;
    let aocs = d.analysis.aocs.as_mut().unwrap();
    aocs.law.retain(|k, _| k == "NOMINAL");
    d
}

fn dv_budget_input(d: &Design) -> sat_analysis::propulsion::PropulsionInput {
    let resolved = fune_model::resolve(d);
    let ctx = Context::bare(d, &resolved);
    propulsion::dv_budget(&ctx).unwrap()
}

#[test]
fn each_phase_with_a_duration_is_a_leg_in_its_own_orbit() {
    let tmp = tempfile::tempdir().unwrap();
    let d = demo_with_rcs(tmp.path());
    let input = dv_budget_input(&d);
    let legs = &input.unloading.as_ref().unwrap().legs;
    let phases: Vec<_> = legs.iter().map(|l| l.phase.as_deref().unwrap()).collect();
    assert_eq!(phases, ["LEOP", "NOMINAL_OPS", "DISPOSAL"]);
    let days: Vec<f64> = legs
        .iter()
        .map(|l| l.duration.unwrap().value() / 86_400.0)
        .collect();
    assert_eq!(days, [30.0, 730.0, 180.0]);
    // 評価する軌道はフェーズのもの（DISPOSAL だけが 250 km の軌道を持つ）
    let period = |i: usize| legs[i].rw_unloading.timeline.orbit().cycle().length_s();
    assert!(period(2) < period(1));
    assert!((period(0) - period(1)).abs() < 1e-9);
    // 区間ごとの推薬の和が合計
    let out = DvBudget::run(&input).unwrap();
    let u = out.unloading.unwrap();
    let sum: f64 = u
        .legs
        .iter()
        .filter_map(|l| l.propellant)
        .map(|p| p.value())
        .sum();
    assert!((u.propellant.value() - sum).abs() <= 1e-12 * sum);
    assert!(u.legs.iter().all(|l| l.counted));
    assert!(u.propellant.value() > 0.0);
    // 1 日あたりの推薬は軌道で違う（低い軌道は空力が効く）
    let per_day = |i: usize| u.legs[i].propellant.unwrap().value() / days[i];
    assert!(per_day(2) != per_day(1));
}

#[test]
fn without_phase_durations_the_mission_duration_is_one_leg() {
    let tmp = tempfile::tempdir().unwrap();
    let mut d = demo_with_rcs(tmp.path());
    for p in d.phases.values_mut() {
        p.duration = None;
    }
    let input = dv_budget_input(&d);
    let legs = &input.unloading.as_ref().unwrap().legs;
    assert_eq!(legs.len(), 1);
    assert!(legs[0].phase.is_none());
    let mission = fune_model::resolve(&d);
    let mission = Context::bare(&d, &mission).mission_duration().unwrap();
    assert_eq!(legs[0].duration, Some(mission));
}

#[test]
fn without_thrusters_nothing_is_added() {
    let d =
        fune_store::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
            .unwrap()
            .design;
    let input = dv_budget_input(&d);
    assert!(input.unloading.is_none());
    assert_eq!(input.unloading_note, Some(UnloadingNote::NoRcs));
    let out = DvBudget::run(&input).unwrap();
    assert!(out.unloading.is_none() && out.rows.iter().all(|r| !r.includes_rcs_unloading));
}
