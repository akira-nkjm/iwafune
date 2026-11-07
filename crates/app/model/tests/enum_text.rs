//! 列挙の文字列の形（`Display`・`FromStr`）は、ファイルと API の綴り（serde）と同じでなければならない。
//! 片方だけ直してずれるのを防ぐため、すべての値で突き合わせる。

use fune_model::design::{BudgetKind, NetKind, SourceKind};
use fune_model::resolve::{Evidence, Origin};
use fune_model::value::ValueType;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt::Display;
use std::str::FromStr;

fn display_same_as_serde<T: Serialize + Display + std::fmt::Debug>(values: &[T]) {
    for v in values {
        let serde_text = serde_json::to_value(v).unwrap();
        assert_eq!(
            serde_text,
            serde_json::Value::String(v.to_string()),
            "{v:?}"
        );
    }
}

fn same_as_serde<T>(values: &[T])
where
    T: Serialize + DeserializeOwned + Display + FromStr + PartialEq + std::fmt::Debug,
    <T as FromStr>::Err: std::fmt::Debug,
{
    display_same_as_serde(values);
    for v in values {
        assert_eq!(&v.to_string().parse::<T>().unwrap(), v);
    }
}

#[test]
fn enum_text_matches_serde_names() {
    same_as_serde(&[BudgetKind::Mass, BudgetKind::Power, BudgetKind::Data]);
    same_as_serde(&[NetKind::Power, NetKind::Data, NetKind::Rf]);
    same_as_serde(&[
        SourceKind::Placeholder,
        SourceKind::Judgement,
        SourceKind::Analysis,
        SourceKind::Datasheet,
        SourceKind::Measured,
    ]);
    same_as_serde(&[
        Origin::Catalog,
        Origin::Override,
        Origin::KindDefault,
        Origin::Derived,
        Origin::Unset,
    ]);
    same_as_serde(&[
        Evidence::Placeholder,
        Evidence::Unsourced,
        Evidence::Judgement,
        Evidence::Analysis,
        Evidence::Datasheet,
        Evidence::Measured,
    ]);
    display_same_as_serde(&[
        ValueType::Scalar,
        ValueType::Vec3,
        ValueType::Quat,
        ValueType::Tensor3,
        ValueType::Enum,
        ValueType::Bool,
        ValueType::Text,
        ValueType::Range,
        ValueType::Table,
    ]);
}
