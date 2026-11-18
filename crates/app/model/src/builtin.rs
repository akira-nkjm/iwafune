//! 組み込みの属性定義・種別定義（registry §3 の語彙）。中身は同じディレクトリの TOML を埋め込む。
//!
//! 表示名（日本語・英語）は `label` / `label_en`、状態・系・列挙の表示名は [`labels`]。
//! 形はプロジェクト独自の `defs/attribute/<id>.toml`・`defs/kind/<id>.toml` と同じ（serde の derive が唯一の解釈）。

mod labels;

pub use crate::role::{Role, STATE_MAIN, STATE_OFF, STATE_THRUSTER_FIRING};

pub use labels::{
    DEFAULT_SECTION, LABELS_TOML, Label, Labels, enum_label, labels, role_label, role_other_label,
    sections, state_label,
};

use crate::design::{AttributeDef, KindDef};
use crate::id::Id;
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// 組み込みの属性定義（TOML の原文）。
pub const ATTRIBUTES_TOML: &str = include_str!("builtin/attributes.toml");
/// 組み込みの種別定義（TOML の原文）。
pub const KINDS_TOML: &str = include_str!("builtin/kinds.toml");

/// 全機器の継承元の種別 ID。
pub const BASE_KIND: &str = "component";

/// 組み込みの属性定義。
pub fn attributes() -> &'static BTreeMap<Id<AttributeDef>, AttributeDef> {
    static CELL: OnceLock<BTreeMap<Id<AttributeDef>, AttributeDef>> = OnceLock::new();
    // 埋め込みの TOML はテスト（`builtin_definitions_parse`）で解析できることを保証している。
    CELL.get_or_init(|| parse_attributes(ATTRIBUTES_TOML).unwrap_or_default())
}

/// 組み込みの種別定義。
pub fn kinds() -> &'static BTreeMap<Id<KindDef>, KindDef> {
    static CELL: OnceLock<BTreeMap<Id<KindDef>, KindDef>> = OnceLock::new();
    CELL.get_or_init(|| parse_kinds(KINDS_TOML).unwrap_or_default())
}

/// 組み込みか。
pub fn is_builtin_attribute(id: &str) -> bool {
    attributes().contains_key(id)
}

/// 組み込みか。
pub fn is_builtin_kind(id: &str) -> bool {
    kinds().contains_key(id)
}

/// `[<id>] …` の表を属性定義の一覧として解析する。
pub fn parse_attributes(
    text: &str,
) -> Result<BTreeMap<Id<AttributeDef>, AttributeDef>, toml::de::Error> {
    let raw: BTreeMap<String, AttributeDef> = toml::from_str(text)?;
    Ok(raw
        .into_iter()
        .map(|(id, mut def)| {
            def.id = Id::new(&id);
            (Id::new(id), def)
        })
        .collect())
}

/// `[<id>] …` の表を種別定義の一覧として解析する。
pub fn parse_kinds(text: &str) -> Result<BTreeMap<Id<KindDef>, KindDef>, toml::de::Error> {
    let raw: BTreeMap<String, KindDef> = toml::from_str(text)?;
    Ok(raw
        .into_iter()
        .map(|(id, mut def)| {
            def.id = Id::new(&id);
            (Id::new(id), def)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::units::{expect_dimension, parse_dimension_name};

    #[test]
    fn builtin_definitions_parse() {
        let attrs = parse_attributes(ATTRIBUTES_TOML);
        assert!(attrs.is_ok(), "{attrs:?}");
        let parsed = parse_kinds(KINDS_TOML);
        assert!(parsed.is_ok(), "{parsed:?}");
        assert!(attributes().len() >= 40);
        assert!(kinds().contains_key("reaction_wheel"));
    }

    #[test]
    fn roles_have_systems_and_bilingual_names() {
        let labels: Labels = toml::from_str(LABELS_TOML).unwrap();
        for role in Role::ALL {
            // None も明示的な対応。キーの欠落を許さない。
            assert!(labels.role_system.contains_key(role), "{role}");
            let kind = kinds().values().find(|k| k.role == Some(*role)).unwrap();
            assert!(!kind.label.trim().is_empty(), "{role}");
            assert!(
                kind.label_en
                    .as_deref()
                    .is_some_and(|s| !s.trim().is_empty()),
                "{role}"
            );
        }
        assert_eq!(labels.role_system.len(), Role::ALL.len());
        assert!(labels.state.contains_key(STATE_OFF));
    }

    #[test]
    fn thruster_vocabulary_has_the_firing_state() {
        for role in [Role::Thruster, Role::PropulsionThruster] {
            let thruster = kinds().values().find(|k| k.role == Some(role)).unwrap();
            assert!(
                thruster.states.iter().any(|s| s == STATE_THRUSTER_FIRING),
                "{role}"
            );
        }
    }

    #[test]
    fn unknown_roles_are_rejected_during_deserialization() {
        assert!(parse_kinds("[custom]\nlabel = 'Custom'\nrole = 'unknown_role'").is_err());
        assert!(
            serde_json::from_str::<KindDef>(r#"{"label":"Custom","role":"unknown_role"}"#).is_err()
        );
    }

    #[test]
    fn builtin_units_match_their_dimension() {
        for (id, def) in attributes() {
            if let Some(dim) = &def.dimension {
                assert!(parse_dimension_name(dim).is_ok(), "{id}: {dim}");
                for unit in &def.units {
                    assert!(expect_dimension(unit, dim).is_ok(), "{id}: {unit} vs {dim}");
                }
            }
        }
    }

    #[test]
    fn builtin_kinds_reference_builtin_attributes() {
        for (id, kind) in kinds() {
            for attr in kind.required.iter().chain(&kind.optional) {
                assert!(attributes().contains_key(attr.as_str()), "{id}: {attr}");
            }
            if let Some(parent) = &kind.extends {
                assert!(kinds().contains_key(parent.as_str()), "{id}: {parent}");
            }
        }
    }
}
