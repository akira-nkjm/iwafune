//! 属性の値の形の往復と、出典の付いた値の 1 行 1 要素の形（§10.2 の 3）。

use super::*;

fn roundtrip(src: &str, hint: Option<ValueType>) -> Authored<Value> {
    let doc: toml::Table = toml::from_str(&format!("v = {src}")).unwrap();
    let value = decode_value(&doc["v"], hint).unwrap();
    let encoded = encode_value(&value);
    let mut out = toml_edit::DocumentMut::new();
    out.insert("v", encoded);
    let again: toml::Table = toml::from_str(&out.to_string()).unwrap();
    assert_eq!(decode_value(&again["v"], hint).unwrap(), value, "{out}");
    value
}

#[test]
fn values_round_trip() {
    let s = roundtrip(
        r#"{ value = 0.62, unit = "kg", from = { source = "x", note = "n" } }"#,
        None,
    );
    assert!(matches!(s.value, Value::Scalar(Scalar { value, .. }) if value == 0.62));
    assert!(matches!(roundtrip("0.6", None).value, Value::Scalar(_)));
    assert!(matches!(
        roundtrip(r#"{ value = 1.0, min = 0.8, max = 1.3, unit = "W" }"#, None).value,
        Value::Scalar(Scalar { min: Some(_), .. })
    ));
    assert!(matches!(
        roundtrip(
            r#"{ on = { value = 1.0, unit = "W" }, off = { value = 0.0, unit = "W" } }"#,
            None
        )
        .value,
        Value::ByState(_)
    ));
    assert!(matches!(
        roundtrip("{ x = 0.0, y = 0.0, z = 1.0 }", None).value,
        Value::Vec3(_)
    ));
    assert!(matches!(
        roundtrip("{ w = 1.0, x = 0.0, y = 0.0, z = 0.0 }", None).value,
        Value::Quat(_)
    ));
    assert!(matches!(
        roundtrip(r#"{ min = -20.0, max = 60.0, unit = "degC" }"#, None).value,
        Value::Range(_)
    ));
    assert!(matches!(
        roundtrip(r#""al6061""#, Some(ValueType::Enum)).value,
        Value::Enum(_)
    ));
    assert!(matches!(roundtrip("true", None).value, Value::Bool(true)));
    // 曲線（1 変数の関数）。引数・値の配列と単位が往復する
    let curve = roundtrip(
        r#"{ at = [0.0, 10.0], at_unit = "km^2/s^2", values = [480.0, 360.5], unit = "kg" }"#,
        None,
    );
    assert!(matches!(
        &curve.value,
        Value::Curve(c) if c.at == [0.0, 10.0] && c.values == [480.0, 360.5] && c.unit == "kg"
    ));
    assert!(
        encoded_text(&curve).contains(r#"at = [0.0, 10.0]"#),
        "{}",
        encoded_text(&curve)
    );
}

#[test]
fn rejects_unknown_keys() {
    let doc: toml::Table = toml::from_str(r#"v = { value = 1.0, unti = "kg" }"#).unwrap();
    assert!(decode_value(&doc["v"], None).is_err());
}

fn encoded_text(value: &Authored<Value>) -> String {
    let mut doc = toml_edit::DocumentMut::new();
    let mut attr = Table::new();
    attr.insert("v", encode_value(value));
    doc.insert("attr", Item::Table(attr));
    crate::profile::finish(doc)
}

#[test]
fn sourced_values_are_written_one_element_per_line() {
    let s = roundtrip(
        r#"{ value = 115.0, unit = "g", from = { source = "ds_example", note = "Mass [g]: 115." } }"#,
        None,
    );
    assert_eq!(
        encoded_text(&s),
        "[attr.v]\nvalue       = 115.0\nunit        = \"g\"\nfrom.source = \"ds_example\"\nfrom.note   = \"Mass [g]: 115.\"\n"
    );
    let range = roundtrip(
        r#"{ min = -20.0, max = 60.0, unit = "degC", from = { source = "x" } }"#,
        None,
    );
    assert_eq!(
        encoded_text(&range),
        "[attr.v]\nmin         = -20.0\nmax         = 60.0\nunit        = \"degC\"\nfrom.source = \"x\"\n"
    );
    let cells = roundtrip(
        r#"{ off = { value = 0.0, unit = "W" }, on = { value = 1.0, unit = "W", from = { source = "x" } } }"#,
        None,
    );
    assert_eq!(
        encoded_text(&cells),
        "[attr.v]\noff            = { value = 0.0, unit = \"W\" }\non.value       = 1.0\non.unit        = \"W\"\non.from.source = \"x\"\n"
    );
    // 出典の無い値は 1 行のまま
    let bare = roundtrip(r#"{ value = 1.0, unit = "kg" }"#, None);
    assert_eq!(
        encoded_text(&bare),
        "[attr]\nv = { value = 1.0, unit = \"kg\" }\n"
    );
}

#[test]
fn both_forms_read_to_the_same_value() {
    let old: toml::Table = toml::from_str(
        "[attr]\nmass = { value = 1.0, unit = \"kg\", from = { source = \"x\", note = \"n\" } }\n[attr.power]\non = { value = 2.0, unit = \"W\", from = { source = \"y\" } }\n",
    )
    .unwrap();
    let new: toml::Table = toml::from_str(
        "[attr.mass]\nvalue = 1.0\nunit = \"kg\"\nfrom.source = \"x\"\nfrom.note = \"n\"\n\n[attr.power]\non.value = 2.0\non.unit = \"W\"\non.from.source = \"y\"\n",
    )
    .unwrap();
    for key in ["mass", "power"] {
        let a = decode_value(&old["attr"][key], None).unwrap();
        let b = decode_value(&new["attr"][key], None).unwrap();
        assert_eq!(a, b, "{key}");
    }
}
