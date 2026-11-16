//! 列挙の文字列の形（`as_str`・`Display`・`FromStr`・`NAMES`）は、ファイルと API の綴り（serde）と同じ。
//! 綴りは `enum_text!` に 1 回だけ書くので構造上ずれないが、マクロを通っていることと、読み直せることをすべての値で確かめる。

use fune_model::Severity;
use fune_model::design::{
    AnalysisSystem, AttitudeLawKind, BudgetKind, CatalogFieldName, HarnessEnd, LinkDirection,
    Maturity, NetEnd, NetFieldName, NetKind, PartFieldName, PropagatorKind, SourceKind,
    UncertaintySide, ViewPortSide,
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
    same_as_serde::<fune_model::builtin::Role>();
    same_as_serde::<Maturity>();
    same_as_serde::<fune_model::design::Redundancy>();
    same_as_serde::<BudgetKind>();
    same_as_serde::<NetKind>();
    same_as_serde::<SourceKind>();
    same_as_serde::<fune_model::design::Distribution>();
    same_as_serde::<fune_model::design::SharedCatalogState>();
    same_as_serde::<UncertaintySide>();
    same_as_serde::<ViewPortSide>();
    same_as_serde::<AttitudeLawKind>();
    same_as_serde::<fune_model::design::ThermalPresetKind>();
    same_as_serde::<LinkDirection>();
    same_as_serde::<PropagatorKind>();
    same_as_serde::<fune_model::design::CentralBodyKind>();
    same_as_serde::<Severity>();
    same_as_serde::<Origin>();
    same_as_serde::<Evidence>();
    same_as_serde::<ValueType>();
    same_as_serde::<SettingScope>();
    same_as_serde::<PartFieldName>();
    same_as_serde::<CatalogFieldName>();
    same_as_serde::<NetFieldName>();
    same_as_serde::<AnalysisSystem>();
    same_as_serde::<fune_model::design::ConditionTopic>();
    same_as_serde::<NetEnd>();
    same_as_serde::<HarnessEnd>();
    same_as_serde::<fune_model::design::ManeuverCategory>();
    same_as_serde::<fune_model::design::ManeuverMethod>();
    same_as_serde::<fune_model::design::ManeuverSteering>();
}

/// `analysis/` の型の欄（serde の名前 = ファイル名）は、共通設定と解析の系から成る。
#[test]
fn analysis_settings_fields_are_the_analysis_systems() {
    use fune_model::design::AnalysisSettings;
    let all = AnalysisSettings {
        common: Some(Default::default()),
        power: Some(Default::default()),
        aocs: Some(Default::default()),
        cdh: Some(Default::default()),
        comm: Some(Default::default()),
        thermal: Some(Default::default()),
        structure: Some(Default::default()),
        payload: Some(Default::default()),
        propulsion: Some(Default::default()),
    };
    let json = serde_json::to_value(&all).unwrap();
    let keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    let mut names = AnalysisSystem::NAMES.to_vec();
    names.push("common");
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    names.sort_unstable();
    assert_eq!(sorted, names);
    for system in AnalysisSystem::ALL {
        assert_eq!(
            system.setting_path().to_string(),
            format!("analysis.{system}")
        );
    }
}

/// 誤りの文は値の一覧を添える（CLI の `--source-side` の誤りなど、そのまま利用者に見せる）。
#[test]
fn unknown_text_names_the_choices() {
    assert_eq!(
        "middle".parse::<ViewPortSide>().unwrap_err().to_string(),
        "`middle` is not a valid terminal side (left / right / top / bottom)"
    );
    assert_eq!(
        "Mass".parse::<BudgetKind>().unwrap_err().to_string(),
        "`Mass` is not a valid budget kind (mass / power / data)"
    );
}

/// 旧系名を移行用の別名として受け付けない。
#[test]
fn old_system_name_is_rejected() {
    use fune_model::design::AnalysisSettings;
    assert!("attitude".parse::<AnalysisSystem>().is_err());
    assert!(serde_json::from_str::<AnalysisSystem>(r#""attitude""#).is_err());
    assert!(serde_json::from_str::<AnalysisSettings>(r#"{"attitude": {}}"#).is_err());
}

/// 解析も設定項目もない系に、編集できない設定の葉を出さない。
#[test]
fn payload_has_no_setting_entries() {
    use fune_model::Design;
    let mut design = Design::default();
    let has_payload = |design: &Design| {
        design.setting_entries().iter().any(|entry| {
            entry.path.scope == SettingScope::Analysis
                && entry.path.segments.first().map(String::as_str) == Some("payload")
        })
    };
    assert!(!has_payload(&design));
    design.analysis.payload = Some(Default::default());
    assert!(!has_payload(&design));
}
