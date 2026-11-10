//! 列挙の文字列の形（`as_str`・`Display`・`FromStr`・`NAMES`）は、ファイルと API の綴り（serde）と同じ。
//! 綴りは `enum_text!` に 1 回だけ書くので構造上ずれないが、マクロを通っていることと、読み直せることをすべての値で確かめる。

use fune_model::Severity;
use fune_model::design::{
    AttitudeLawKind, BudgetKind, CatalogFieldName, LinkDirection, Maturity, NetFieldName, NetKind,
    PartFieldName, PropagatorKind, SourceKind, UncertaintySide, ViewPortSide,
};
use fune_model::enum_text::{EnumText, EnumTextError};
use fune_model::path::SettingScope;
use fune_model::resolve::{Evidence, Origin};
use fune_model::value::ValueType;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt::{Debug, Display};
use std::str::FromStr;

fn same_as_serde<T>()
where
    T: EnumText
        + Serialize
        + DeserializeOwned
        + Display
        + FromStr<Err = EnumTextError>
        + PartialEq
        + Debug,
{
    assert_eq!(T::ALL.len(), T::NAMES.len());
    for (v, name) in T::ALL.iter().zip(T::NAMES) {
        assert_eq!(v.as_str(), *name);
        assert_eq!(
            serde_json::to_value(v).unwrap(),
            serde_json::Value::String(v.to_string()),
            "{v:?}"
        );
        assert_eq!(&v.to_string().parse::<T>().unwrap(), v);
        assert_eq!(
            &serde_json::from_value::<T>(serde_json::Value::String((*name).to_owned())).unwrap(),
            v
        );
    }
    let e = "no_such_value".parse::<T>().unwrap_err();
    assert_eq!(e.what, T::WHAT);
    assert_eq!(e.choices, T::NAMES.join(" / "));
}

#[test]
fn enum_text_matches_serde_names() {
    same_as_serde::<Maturity>();
    same_as_serde::<BudgetKind>();
    same_as_serde::<NetKind>();
    same_as_serde::<SourceKind>();
    same_as_serde::<UncertaintySide>();
    same_as_serde::<ViewPortSide>();
    same_as_serde::<AttitudeLawKind>();
    same_as_serde::<LinkDirection>();
    same_as_serde::<PropagatorKind>();
    same_as_serde::<Severity>();
    same_as_serde::<Origin>();
    same_as_serde::<Evidence>();
    same_as_serde::<ValueType>();
    same_as_serde::<SettingScope>();
    same_as_serde::<PartFieldName>();
    same_as_serde::<CatalogFieldName>();
    same_as_serde::<NetFieldName>();
}

/// 誤りの文は値の一覧を添える（CLI の `--source-side` の誤りなど、そのまま利用者に見せる）。
#[test]
fn unknown_text_names_the_choices() {
    assert_eq!(
        "middle".parse::<ViewPortSide>().unwrap_err().to_string(),
        "`middle` は端子の側ではありません（left / right / top / bottom）"
    );
    assert_eq!(
        "Mass".parse::<BudgetKind>().unwrap_err().to_string(),
        "`Mass` はバジェットの種類ではありません（mass / power / data）"
    );
}
