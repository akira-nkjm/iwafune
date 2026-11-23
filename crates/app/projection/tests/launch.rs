//! 打ち上げ条件（`conditions/launch.toml` が選ぶ打ち上げ機・ディスペンサー）: 値の優先（launch.toml → ディスペンサー → 打ち上げ機）、
//! 要求が限度を厳しくする側にだけ働くこと、足りない入力が置き場所を名指すこと。

use fune_model::Design;
use fune_projection::{Context, ProjectionError, project_json};
use serde_json::Value;
use std::path::Path;

fn demo() -> Design {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    fune_store::load(&root)
        .expect("the demo should load")
        .design
}

/// 解析 `id` の入力（JSON）。足りない入力があれば、そのパスと理由を返す。
fn input(design: &Design, id: &str) -> Result<Value, Vec<(String, String)>> {
    let resolved = fune_model::resolve(design);
    let ctx = Context::new(design, &resolved, None).unwrap();
    project_json(id, &ctx).map_err(|e| match e {
        ProjectionError::Missing(m) => m
            .into_vec()
            .into_iter()
            .map(|m| (m.path, m.reason))
            .collect(),
        other => panic!("{other}"),
    })
}

fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or_else(|| panic!("{v} is not a number"))
}

fn case(d: &Design) -> (f64, f64) {
    let load = input(d, "structure.static_load").unwrap();
    (
        num(&load["cases"][0]["axial"]),
        num(&load["cases"][0]["lateral"]),
    )
}

fn launch(d: &mut Design) -> &mut fune_model::design::LaunchSettings {
    d.conditions.launch.as_mut().unwrap()
}

/// launch.toml に書いた荷重が先、無ければディスペンサー、それも無ければ打ち上げ機の値。`g0` で読む。
#[test]
fn loads_follow_launch_toml_then_dispenser_then_launcher() {
    let mut d = demo();
    assert_eq!(case(&d), (6.0, 2.0));
    launch(&mut d).load_axial = None;
    launch(&mut d).load_lateral = None;
    let (axial, lateral) = case(&d);
    assert!(
        (axial - 7.0).abs() < 1e-9 && (lateral - 2.5).abs() < 1e-9,
        "{axial} {lateral}"
    );
    launch(&mut d).dispenser = None;
    let (axial, lateral) = case(&d);
    assert!(
        (axial - 8.0).abs() < 1e-9 && (lateral - 3.0).abs() < 1e-9,
        "{axial} {lateral}"
    );
}

/// どこにも荷重が無ければ、launch.toml のキーを名指しし、カタログ品の属性でも与えられると伝える。
#[test]
fn a_missing_load_names_where_it_can_be_given() {
    let mut d = demo();
    launch(&mut d).load_axial = None;
    launch(&mut d).launcher = None;
    launch(&mut d).dispenser = None;
    let missing = input(&d, "structure.static_load").unwrap_err();
    let (path, reason) = missing
        .iter()
        .find(|(p, _)| p == "conditions.launch.load_axial")
        .unwrap();
    assert_eq!(path, "conditions.launch.load_axial");
    assert!(
        reason.contains("conditions/launch.toml") && reason.contains("launch_load_axial"),
        "{reason}"
    );
}

/// 質量の上限は打ち上げ機・ディスペンサーの値（ディスペンサーが先）と質量の要求の候補で、出所付き。
#[test]
fn launch_fit_input_carries_the_limits_with_their_origin() {
    let mut d = demo();
    let fit = input(&d, "structure.launch_fit").unwrap();
    let limits = fit["max_mass"].as_array().unwrap();
    assert_eq!(limits[0]["origin"], "dispenser local/disp_6u_rs");
    assert!((num(&limits[0]["value"]) - 14.0).abs() < 1e-9);
    assert_eq!(limits[1]["origin"], "requirement R_MASS");
    assert!((num(&limits[1]["value"]) - 12.0).abs() < 1e-9);
    // 外形はディスペンサーの値（打ち上げ機の箱より先）
    assert_eq!(fit["envelope"]["origin"], "dispenser local/disp_6u_rs");
    assert!((num(&fit["envelope"]["size"]["y"]) - 0.24).abs() < 1e-9);
    // 構体の外形（0.1 × 0.226 × 0.366 m）
    assert!((num(&fit["size"]["z"]) - 0.366).abs() < 1e-9);
    // ディスペンサーを外すと打ち上げ機の値になる
    launch(&mut d).dispenser = None;
    let fit = input(&d, "structure.launch_fit").unwrap();
    assert_eq!(fit["max_mass"][0]["origin"], "launcher local/lv_alpha");
    assert_eq!(fit["envelope"]["origin"], "launcher local/lv_alpha");
    assert!((num(&fit["min_axial"][0]["value"]) - 50.0).abs() < 1e-9);
}

/// 推進系があれば積む推薬が入る（部品の行と別に持つ）。
#[test]
fn launch_fit_input_splits_parts_and_propellant() {
    let d = demo();
    let fit = input(&d, "structure.launch_fit").unwrap();
    // 部品の行は質量バジェットと同じ（成長余裕つき）。素の合計は 5.548 kg、成長込みは行から数える
    let parts = fit["parts"].as_array().unwrap();
    let bare: f64 = parts
        .iter()
        .map(|p| num(&p["value"]) * num(&p["count"]))
        .sum();
    assert!((bare - 5.548).abs() < 1e-9);
    let grown: f64 = parts
        .iter()
        .map(|p| num(&p["value"]) * num(&p["count"]) * (1.0 + num(&p["growth"])))
        .sum();
    assert!(grown > bare, "{grown} {bare}");
    let propellant = num(&fit["propellant_mass"]);
    assert!(propellant > 0.0 && propellant < 1.0, "{propellant}");
}

/// 最低固有振動数の要求は、ディスペンサー・打ち上げ機の値を厳しくする側にだけ働く。
#[test]
fn a_requirement_only_tightens_the_catalog_frequency() {
    let mut d = demo();
    let axial =
        |d: &Design| num(&input(d, "structure.modal_estimate").unwrap()["requirements"]["axial"]);
    // デモの要求 100 Hz はディスペンサーの 90 Hz より厳しい
    assert!((axial(&d) - 100.0).abs() < 1e-9);
    // 要求を外すと、ディスペンサーの値（打ち上げ機の 50 Hz より先）
    let kept = d.requirements.remove("R_FREQ_AXIAL").unwrap();
    assert!((axial(&d) - 90.0).abs() < 1e-9);
    // 緩い要求（60 Hz）は効かない
    let mut loose = kept.clone();
    loose.constraint.value = Some(fune_model::value::Quantity::new(60.0, "Hz".to_owned()));
    d.requirements.insert(loose.id.clone(), loose);
    assert!((axial(&d) - 90.0).abs() < 1e-9);
}

/// 打ち上げ機もディスペンサーも選んでいなければ、選ぶ場所を名指す。
#[test]
fn launch_fit_asks_for_a_selection() {
    let mut d = demo();
    launch(&mut d).launcher = None;
    launch(&mut d).dispenser = None;
    let missing = input(&d, "structure.launch_fit").unwrap_err();
    assert!(
        missing
            .iter()
            .any(|(p, _)| p == "conditions.launch.launcher"),
        "{missing:?}"
    );
}
