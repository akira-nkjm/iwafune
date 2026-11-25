//! 質量のバジェットの推薬（推進系の解析の積む推薬）と、乾燥・推薬込みの合計と判定。高度低下の軌道維持も同じ推進系の解析の推薬を使う。
use fune_service::dto::{
    BudgetKind, BudgetPropellant, BudgetRequest, BudgetView, RunAnalysisRequest, Verdict,
};
use fune_service::{Workspace, analysis, budget};
use serde_json::{Value, json};
use std::path::Path;

fn demo() -> Workspace {
    Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
        .expect("demo")
}

fn ops(v: Value) -> Vec<fune_service::dto::Op> {
    serde_json::from_value(v).unwrap()
}

fn mass(ws: &Workspace, ops: Vec<fune_service::dto::Op>) -> BudgetView {
    budget::budget(ws, BudgetKind::Mass, &BudgetRequest { case: None, ops }).unwrap()
}

/// 解析の出力の欄（SI の数）。
fn output(ws: &Workspace, id: &str, ops: Vec<fune_service::dto::Op>) -> Value {
    let req = RunAnalysisRequest {
        ops,
        ..RunAnalysisRequest::default()
    };
    let r = analysis::run_analysis(ws, id, &req).unwrap();
    assert!(r.missing.is_empty(), "{id}: {:?}", r.missing);
    r.output.unwrap()
}

/// マヌーバと推進系の部品をすべて消す編集。
fn without_propulsion() -> Value {
    json!([
        { "op": "remove_maneuver", "id": "DEORBIT" },
        { "op": "remove_maneuver", "id": "RAISE" },
        { "op": "remove_part", "id": "EPT" },
        { "op": "remove_part", "id": "XE_TANK" },
    ])
}

/// デモ: 推薬は `propulsion.propellant` の積む推薬。推薬込み = 乾燥（成長余裕込み）+ 推薬（成長余裕なし）、
/// 余裕 = 上限 12 kg × (1 − 0.1) − 合計、塗り = 合計 / 上限を手で確かめる。乾燥の合計は推薬で変わらない。
#[test]
fn demo_counts_loaded_propellant_and_compares_dry_and_wet() {
    let ws = demo();
    let v = mass(&ws, Vec::new());
    let loaded = output(&ws, "propulsion.propellant", Vec::new())["loaded_propellant"]
        .as_f64()
        .unwrap();
    assert!((loaded - 0.0672).abs() < 1e-3, "{loaded}");
    let dry = v.total.as_ref().unwrap().si;
    let rows: f64 = v.rows.iter().map(|r| r.with_growth.si).sum();
    assert!((dry - rows).abs() < 1e-12);
    let Some(BudgetPropellant::Counted {
        analysis,
        loaded: shown,
        wet,
        tanks,
    }) = &v.propellant
    else {
        panic!("{:?}", v.propellant);
    };
    assert_eq!(analysis, "propulsion.propellant");
    assert!((shown.si - loaded).abs() < 1e-15);
    // タンクが 1 つのデモでは、その 1 つが積む推薬の全部（組の積む量 `group.required` は `loaded_propellant` と丸めの差だけ違う）
    assert_eq!(tanks.len(), 1);
    assert!((tanks[0].loaded.si - loaded).abs() < 1e-9);
    assert!((wet.total.si - (dry + loaded)).abs() < 1e-12);
    let allowed = 12.0 * (1.0 - 0.1);
    assert!((wet.margin.as_ref().unwrap().si - (allowed - dry - loaded)).abs() < 1e-12);
    assert!((v.margin.as_ref().unwrap().si - (allowed - dry)).abs() < 1e-12);
    assert!((wet.fill.unwrap().ratio - (dry + loaded) / 12.0).abs() < 1e-12);
    assert_eq!(
        (v.verdict, wet.verdict),
        (Some(Verdict::Ok), Some(Verdict::Ok))
    );
}

/// 上限を乾燥と推薬込みのあいだにすると、乾燥は OK・推薬込みは違反（両方を上限と比べる）。
#[test]
fn wet_can_violate_while_dry_fits() {
    let ws = demo();
    let base = mass(&ws, Vec::new());
    let dry = base.total.as_ref().unwrap().si;
    // 使える量 = 上限 × 0.9 が乾燥 + 30 g になる上限
    let limit = (dry + 0.03) / 0.9;
    let v = mass(
        &ws,
        // 上限は要求 `R_MASS` が持つ
        ops(json!([{ "op": "set_requirement", "requirement": {
            "id": "R_MASS", "statement": "The mass shall not exceed the limit.", "quantity": "Mass",
            "constraint": { "kind": "at_most", "value": { "value": limit, "unit": "kg" } },
            "verify": { "budget": "mass" } } }])),
    );
    let Some(BudgetPropellant::Counted { wet, .. }) = &v.propellant else {
        panic!("{:?}", v.propellant);
    };
    assert_eq!(v.verdict, Some(Verdict::Ok));
    assert_eq!(wet.verdict, Some(Verdict::Violation));
}

/// 推進系が無ければ推薬の行を出さず、理由は「推進系が無い」。推進系の部品があってマヌーバが無ければ「入力が足りない」。
/// 電力とデータ量のバジェットは推薬を持たない。
#[test]
fn propellant_is_omitted_with_a_reason() {
    let ws = demo();
    let none = mass(&ws, ops(without_propulsion()));
    assert!(matches!(
        none.propellant,
        Some(BudgetPropellant::NoPropulsion)
    ));
    assert!(none.total.is_some() && none.missing.is_empty());
    let no_maneuver = mass(
        &ws,
        ops(json!([
            { "op": "remove_maneuver", "id": "DEORBIT" },
            { "op": "remove_maneuver", "id": "RAISE" },
        ])),
    );
    let Some(BudgetPropellant::Missing { missing }) = &no_maneuver.propellant else {
        panic!("{:?}", no_maneuver.propellant);
    };
    assert!(
        missing.iter().any(|m| m.reason.contains("Maneuver")),
        "{missing:?}"
    );
    for kind in [BudgetKind::Power, BudgetKind::Data] {
        let v = budget::budget(&ws, kind, &BudgetRequest::default()).unwrap();
        assert!(v.propellant.is_none(), "{kind:?}");
    }
}

/// 高度低下の軌道維持の推薬は `propulsion.propellant` の使える推薬（以前の `[decay] propellant` は使わず、警告で外すよう促す）。
/// ΔV は最も低い比推力のスラスタ（デモは EPT 1200 s）で、推薬を積んだ質量から使える推薬を使い切ったときの値。
#[test]
fn decay_uses_the_usable_propellant_of_the_propulsion_analysis() {
    let ws = demo();
    let p = output(&ws, "propulsion.propellant", Vec::new());
    let usable = p["usable_propellant"].as_f64().unwrap();
    let wet = p["wet_mass"].as_f64().unwrap();
    let legacy = ops(
        json!([{ "op": "set_setting", "path": "conditions.orbit.decay.propellant",
        "value": { "type": "quantity", "value": { "value": 0.2, "unit": "kg" } } }]),
    );
    for ops in [Vec::new(), legacy.clone()] {
        let used = output(&ws, "orbit.decay", ops)["used"].clone();
        assert!((used["propellant"].as_f64().unwrap() - usable).abs() < 1e-15);
        assert_eq!(used["thruster"], "EPT");
        let dv = 1200.0 * 9.80665 * (wet / (wet - usable)).ln();
        assert!((used["propulsion_delta_v"].as_f64().unwrap() - dv).abs() < 1e-9);
    }
    let s = fune_service::preview::snapshot_with(&ws, &legacy).unwrap();
    assert!(
        s.diagnostics
            .iter()
            .any(|d| d.diagnostic.code == "setting.moved"),
        "{:?}",
        s.diagnostics
    );
    let none = output(&ws, "orbit.decay", ops(without_propulsion()))["used"].clone();
    assert!(none["propellant"].is_null() && none["propulsion_delta_v"].is_null());
}
