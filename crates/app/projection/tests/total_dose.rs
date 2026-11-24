//! `orbit.total_dose` の射影: 環境の表（軌道の設定の曲線）・部品の遮蔽厚さと許容線量・設計全体の設定から入力ができ、
//! 足りないものは欄を指す不足入力になること。デモの設計をコードで書き換える（デモのデータは変えない）。

use fune_model::Design;
use fune_model::design::{CommonSettings, RadiationSettings};
use fune_model::value::{Curve, Quantity, Scalar, Value};
use fune_projection::{Context, MissingInput, orbit};
use sat_analysis::orbit::TotalDose;
use sat_analysis::spec::Analysis;
use std::path::Path;

const YEAR: f64 = 365.25 * 86_400.0;

fn demo() -> Design {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    fune_store::load(&root)
        .expect("the demo should load")
        .design
}

fn table(rates: [f64; 2]) -> Curve {
    Curve {
        at: vec![1.0, 10.0],
        at_unit: "mm".into(),
        values: rates.to_vec(),
        unit: "Gy/yr".into(),
    }
}

fn set(d: &mut Design, part: &str, attr: &str, value: f64, unit: &str) {
    let p = d.parts.get_mut(part).expect("part");
    p.attr.insert(
        fune_model::Id::new(attr),
        fune_model::value::Authored::bare(Value::Scalar(Scalar::new(value, unit))),
    );
}

fn input(d: &Design) -> Result<sat_analysis::orbit::TotalDoseInput, Vec<MissingInput>> {
    let resolved = fune_model::resolve(d);
    let ctx = Context::new(d, &resolved, None).expect("context");
    orbit::total_dose(&ctx).map_err(|m| m.into_vec())
}

/// デモの設計は、表も部品の値も無いので不足入力（環境の表は設計の軌道とフェーズの軌道の上書き、部品の許容線量）。
#[test]
fn the_unchanged_demo_reports_missing_inputs_on_their_own_keys() {
    let missing = input(&demo()).expect_err("missing");
    let paths: Vec<&str> = missing.iter().map(|m| m.path.as_str()).collect();
    assert!(
        paths.contains(&"conditions.orbit.dose_rate_by_shielding"),
        "{paths:?}"
    );
    assert!(
        paths.contains(&"phases.DISPOSAL.orbit.dose_rate_by_shielding"),
        "{paths:?}"
    );
    assert!(paths.contains(&"<part>.tid_tolerance"), "{paths:?}");
}

/// 書いた値が SI で入力になる（mm → m、Gy/yr → Gy/s、日 → 秒）。フェーズに上書きの軌道があって表が無いフェーズは、
/// 評価しない区間として入る（表を推測しない）。
#[test]
fn written_values_become_si_inputs_and_a_phase_without_a_table_is_left_empty() {
    let mut d = demo();
    d.conditions.orbit.as_mut().unwrap().dose_rate_by_shielding = Some(table([100.0, 10.0]));
    set(&mut d, "OBC", "shielding", 2.0, "mm");
    set(&mut d, "OBC", "tid_tolerance", 0.1, "kGy");
    d.analysis.common = Some(CommonSettings {
        radiation: Some(RadiationSettings {
            default_shielding: Some(Quantity::new(3.0, "mm")),
            design_margin: Some(1.5),
        }),
        ..Default::default()
    });
    let i = input(&d).expect("input");
    assert_eq!(i.design_margin, Some(1.5));
    assert!((i.default_shielding.unwrap().value() - 3e-3).abs() < 1e-15);
    let obc = i.parts.iter().find(|p| p.part == "OBC").unwrap();
    assert!((obc.shielding.unwrap().value() - 2e-3).abs() < 1e-15);
    assert!((obc.tolerance.unwrap().value() - 100.0).abs() < 1e-9);
    let nominal = i
        .legs
        .iter()
        .find(|l| l.phase.as_deref() == Some("NOMINAL_OPS"))
        .unwrap();
    assert!((nominal.duration.unwrap().value() - 730.0 * 86_400.0).abs() < 1e-6);
    let pts = nominal.environment.as_ref().unwrap();
    assert!((pts[0].rate.value() - 100.0 / YEAR).abs() < 1e-15);
    assert!((pts[1].shielding.value() - 10e-3).abs() < 1e-15);
    let disposal = i
        .legs
        .iter()
        .find(|l| l.phase.as_deref() == Some("DISPOSAL"))
        .unwrap();
    assert!(disposal.environment.is_none());
    // 解析が走り、除いた区間を警告する。OBC: 2 mm は 1 mm（100）と 10 mm（10）の間 → 対数線形
    let o = TotalDose::run(&i).expect("run");
    assert!(o.checks.iter().any(|c| c.key == "phases_left_out"));
    let rate = 100.0 * (10.0f64 / 100.0).powf(1.0 / 9.0);
    let days = 30.0 + 730.0;
    let want = rate * days / 365.25;
    let got = o
        .parts
        .iter()
        .find(|p| p.part == "OBC")
        .unwrap()
        .total
        .unwrap()
        .value();
    assert!((got - want).abs() < 1e-9 * want, "{got} vs {want}");
}

/// 次元の違う単位は、表の欄を指す不足入力になる。
#[test]
fn a_wrong_unit_in_the_table_points_at_the_table() {
    let mut d = demo();
    let mut bad = table([100.0, 10.0]);
    bad.unit = "kg".into();
    d.conditions.orbit.as_mut().unwrap().dose_rate_by_shielding = Some(bad);
    set(&mut d, "OBC", "tid_tolerance", 100.0, "Gy");
    let missing = input(&d).expect_err("missing");
    assert!(
        missing
            .iter()
            .any(|m| m.path == "conditions.orbit.dose_rate_by_shielding")
    );
}
