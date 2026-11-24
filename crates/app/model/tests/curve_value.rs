//! 曲線の値（`payload_mass_by_c3`）の検査: 形（点の数・昇順・有限）、引数と値の単位の次元、組み込みの属性定義。
use fune_model::builtin;
use fune_model::check::check_value_against;
use fune_model::diagnostic::Severity;
use fune_model::value::{Curve, Value, ValueType};

fn curve(at: &[f64], at_unit: &str, values: &[f64], unit: &str) -> Value {
    Value::Curve(Curve {
        at: at.to_vec(),
        at_unit: at_unit.into(),
        values: values.to_vec(),
        unit: unit.into(),
    })
}

fn codes(v: &Value) -> Vec<&'static str> {
    let def = &builtin::attributes()["payload_mass_by_c3"];
    check_value_against(def, v, &[])
        .into_iter()
        .map(|f| f.1)
        .collect()
}

#[test]
fn the_builtin_definition_is_a_mass_curve_over_specific_energy() {
    let def = &builtin::attributes()["payload_mass_by_c3"];
    assert_eq!(def.value_type, ValueType::Curve);
    assert_eq!(def.dimension.as_deref(), Some("Mass"));
    assert_eq!(def.at_dimension.as_deref(), Some("SpecificEnergy"));
}

#[test]
fn a_well_formed_curve_has_no_findings() {
    assert!(codes(&curve(&[0.0, 10.0], "km^2/s^2", &[480.0, 360.0], "kg")).is_empty());
    // 同じ次元の別の単位（m²/s²・g）は許される（許可の単位の外は warning だけ）
    assert!(
        codes(&curve(&[0.0, 1e7], "m^2/s^2", &[480.0, 360.0], "kg")).is_empty(),
        "m^2/s^2 is the same dimension"
    );
}

#[test]
fn shape_errors() {
    assert_eq!(codes(&curve(&[], "km^2/s^2", &[], "kg")), ["curve.shape"]);
    assert_eq!(
        codes(&curve(&[0.0, 10.0], "km^2/s^2", &[1.0], "kg")),
        ["curve.shape"]
    );
    assert_eq!(
        codes(&curve(&[10.0, 0.0], "km^2/s^2", &[1.0, 2.0], "kg")),
        ["curve.shape"]
    );
    assert_eq!(
        codes(&curve(&[0.0, 0.0], "km^2/s^2", &[1.0, 2.0], "kg")),
        ["curve.shape"]
    );
    assert_eq!(
        codes(&curve(&[0.0, f64::NAN], "km^2/s^2", &[1.0, 2.0], "kg")),
        ["curve.shape"]
    );
}

#[test]
fn units_are_checked_for_both_axes() {
    // 値が質量でない
    assert_eq!(
        codes(&curve(&[0.0], "km^2/s^2", &[1.0], "m")),
        ["unit.dimension"]
    );
    // 引数が C3（SpecificEnergy）でない
    assert_eq!(
        codes(&curve(&[0.0], "km", &[1.0], "kg")),
        ["unit.dimension"]
    );
    // 読めない単位
    assert_eq!(codes(&curve(&[0.0], "bogus", &[1.0], "kg")), ["unit.parse"]);
    // 値の域（質量は 0 以上）は warning
    let def = &builtin::attributes()["payload_mass_by_c3"];
    let f = check_value_against(def, &curve(&[0.0], "km^2/s^2", &[-1.0], "kg"), &[]);
    assert!(
        f.iter()
            .any(|f| f.1 == "value.out_of_domain" && f.0 == Severity::Warning)
    );
}
