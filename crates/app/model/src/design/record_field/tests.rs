use super::*;
use crate::design::{Case, Harness, Maneuver, Phase, Requirement, Source};

fn case() -> Case {
    serde_json::from_value(serde_json::json!({ "id": "hot", "label": "Hot", "orbits": 3 })).unwrap()
}

fn requirement() -> Requirement {
    serde_json::from_value(serde_json::json!({
        "id": "R1", "statement": "The mass shall be at most 10 kg", "quantity": "Mass",
        "constraint": { "kind": "at_most", "value": { "value": 10.0, "unit": "kg" } },
        "verify": { "check": "mass" }
    }))
    .unwrap()
}

#[test]
fn paths_name_a_collection_an_id_and_a_field() {
    assert_eq!(
        parse_record_field("cases.hot.beta").unwrap().unwrap(),
        (RecordKind::Case, "hot".to_owned(), "beta".to_owned())
    );
    assert_eq!(
        parse_record_field("requirements.R1.constraint.value")
            .unwrap()
            .unwrap(),
        (
            RecordKind::Requirement,
            "R1".to_owned(),
            "constraint.value".to_owned()
        )
    );
    assert!(parse_record_field("RW_X.mass").is_none());
    assert!(parse_record_field("analysis.power.dod_max").is_none());
    // 2 区切り以下は部品の属性などのパス（`harness.mass`）
    for part_path in ["cases", "cases.hot", "harness.mass", "phases.P"] {
        assert!(parse_record_field(part_path).is_none(), "{part_path}");
    }
    for bad in ["cases..beta", "cases.hot.", "cases.hot..x"] {
        assert!(parse_record_field(bad).unwrap().is_err(), "{bad}");
    }
}

#[test]
fn unknown_fields_list_the_valid_ones() {
    let e = RecordKind::Harness.field("colour").unwrap_err().to_string();
    assert!(
        e.contains("`colour` is not a field of a harness record"),
        "{e}"
    );
    assert!(
        e.contains("label, from, to, nets, wire, cores, length"),
        "{e}"
    );
}

#[test]
fn each_type_reads_and_writes_its_text() {
    let read = |ty, t: &str| RecordValue::from_text("f", ty, t).unwrap();
    assert_eq!(read(FieldType::Integer, " -3 "), RecordValue::Integer(-3));
    assert_eq!(read(FieldType::Unsigned, "15"), RecordValue::Integer(15));
    assert_eq!(read(FieldType::Number, "0.5"), RecordValue::Number(0.5));
    assert_eq!(read(FieldType::Bool, "true"), RecordValue::Bool(true));
    assert_eq!(
        read(FieldType::List, "a, b"),
        RecordValue::List(vec!["a".into(), "b".into()])
    );
    assert_eq!(
        read(FieldType::Quantity, "-45 deg"),
        RecordValue::Quantity(Quantity::new(-45.0, "deg"))
    );
    for (ty, t) in [
        (FieldType::Unsigned, "-1"),
        (FieldType::Unsigned, "1.5"),
        (FieldType::Integer, "x"),
        (FieldType::Number, "nan"),
        (FieldType::Bool, "yes"),
        (FieldType::Choice(&["a", "b"]), "c"),
        (FieldType::Quantity, "sixty"),
    ] {
        assert!(RecordValue::from_text("f", ty, t).is_err(), "{t}");
    }
    for v in [
        RecordValue::Text("x y".into()),
        RecordValue::Integer(4),
        RecordValue::Number(0.25),
        RecordValue::Bool(false),
        RecordValue::Quantity(Quantity::new(1.5, "km")),
        RecordValue::List(vec!["p".into(), "q".into()]),
    ] {
        let ty = match v {
            RecordValue::Text(_) => FieldType::Text,
            RecordValue::Integer(_) => FieldType::Integer,
            RecordValue::Number(_) => FieldType::Number,
            RecordValue::Bool(_) => FieldType::Bool,
            RecordValue::Quantity(_) => FieldType::Quantity,
            RecordValue::List(_) => FieldType::List,
        };
        assert_eq!(RecordValue::from_text("f", ty, &v.to_text()).unwrap(), v);
    }
}

#[test]
fn patching_changes_one_field_and_nested_tables_follow() {
    let c = patched(&case(), "orbits", None).unwrap();
    assert_eq!((c.orbits, c.label.as_deref()), (None, Some("Hot")));
    let r = patched(
        &requirement(),
        "constraint.value",
        Some(&RecordValue::Quantity(Quantity::new(12.0, "kg"))),
    )
    .unwrap();
    assert_eq!(r.constraint.value, Some(Quantity::new(12.0, "kg")));
    assert_eq!(r.verify.check.as_deref(), Some("mass"));
    // 入れ子の表の最後の欄を外すと表ごと無くなる
    let r = patched(&r, "verify.check", None).unwrap();
    assert!(r.verify.is_empty());
    // 無かった表に欄を足すと表ができる
    let r = patched(
        &r,
        "verify.analysis",
        Some(&RecordValue::Text("power.sizing".into())),
    )
    .unwrap();
    assert_eq!(r.verify.analysis.as_deref(), Some("power.sizing"));
    // 必須の欄は外せない。型に合わない値は拒む
    let e = patched(&r, "statement", None).unwrap_err().to_string();
    assert!(e.contains("missing field `statement`"), "{e}");
    assert!(
        patched(
            &r,
            "constraint.kind",
            Some(&RecordValue::Text("sideways".into()))
        )
        .is_err()
    );
}

/// 宣言した欄の名前がすべて、レコードの serde が読めるキーになっている（欄を足して宣言を忘れる・綴りを間違えるのを防ぐ）。
#[test]
fn every_declared_field_is_a_key_of_its_record() {
    fn sample(ty: FieldType) -> RecordValue {
        match ty {
            FieldType::Text => RecordValue::Text("x".into()),
            FieldType::Choice(names) => RecordValue::Text(names[0].into()),
            FieldType::Integer | FieldType::Unsigned => RecordValue::Integer(1),
            FieldType::Number => RecordValue::Number(0.5),
            FieldType::Bool => RecordValue::Bool(true),
            FieldType::Quantity => RecordValue::Quantity(Quantity::new(1.0, "m")),
            FieldType::List => RecordValue::List(vec!["x".into()]),
        }
    }
    fn check<T: Serialize + DeserializeOwned>(kind: RecordKind, base: serde_json::Value) {
        let base: T = serde_json::from_value(base).unwrap();
        for spec in kind.fields() {
            let v = sample(spec.ty);
            // 一覧・ポート・ID の綴りは serde が決める（ここでは「未知のキー」で落ちないことだけを見る）
            if let Err(e) = patched(&base, spec.name, Some(&v)) {
                let m = e.to_string();
                assert!(!m.contains("unknown field"), "{kind:?}.{}: {m}", spec.name);
            }
        }
    }
    check::<Case>(RecordKind::Case, serde_json::json!({}));
    check::<Maneuver>(
        RecordKind::Maneuver,
        serde_json::json!({ "category": "other", "method": "impulsive" }),
    );
    check::<Requirement>(
        RecordKind::Requirement,
        serde_json::to_value(requirement()).unwrap(),
    );
    check::<Harness>(
        RecordKind::Harness,
        serde_json::json!({ "from": "A.X", "to": "B.Y" }),
    );
    check::<Source>(
        RecordKind::Source,
        serde_json::json!({ "kind": "judgement" }),
    );
    check::<Phase>(RecordKind::Phase, serde_json::json!({}));
}
