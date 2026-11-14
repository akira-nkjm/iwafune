//! 組み込みの語彙の表示名（日本語・英語）。機器の状態・属性の系・API の列挙の値。中身は `labels.toml`。

use super::Role;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;
use ts_rs::TS;

/// 組み込みの表示名（TOML の原文）。
pub const LABELS_TOML: &str = include_str!("labels.toml");

/// 表示名 1 つ。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(rename = "VocabLabel")]
pub struct Label {
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_en: Option<String>,
}

/// 語彙の表示名の一覧。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(rename = "VocabLabels")]
pub struct Labels {
    /// role → 解析の系（None は系に属さないと明示）。
    #[serde(default)]
    pub role_system: BTreeMap<Role, Option<String>>,
    /// 画面で系（section）を並べる順
    #[serde(default)]
    pub section_order: Vec<String>,
    /// 機器の状態（`off`・`on`・`rx` …）
    #[serde(default)]
    pub state: BTreeMap<String, Label>,
    /// 属性の系（AttributeDef の `section`）
    #[serde(default)]
    pub section: BTreeMap<String, Label>,
    /// 列挙の型（`maturity`・`net_kind`・`source_kind`・`distribution`・`evidence`・`origin`・`severity`・`attitude_law`・
    /// `propagator`・`link_direction`・`uncertainty_side`・`value_type`・`budget_kind`・`maneuver_category`・`maneuver_method`・`central_body`・`maneuver_steering`）→ 値 → 表示名
    #[serde(default, rename = "enum")]
    pub enums: BTreeMap<String, BTreeMap<String, Label>>,
    /// 列挙の型 → 値を選択肢に並べる順（`enum` の同じ型の値をすべて並べる。無い型は値の綴りの順）
    #[serde(default)]
    pub enum_order: BTreeMap<String, Vec<String>>,
}

/// 組み込みの表示名（埋め込みの TOML はテストで解析できることを保証している）。
pub fn labels() -> &'static Labels {
    static CELL: OnceLock<Labels> = OnceLock::new();
    CELL.get_or_init(|| toml::from_str(LABELS_TOML).unwrap_or_default())
}

/// section の無い属性の系。
pub const DEFAULT_SECTION: &str = "basic";

/// 系（section）の ID と表示名を、画面で並べる順に。
pub fn sections() -> Vec<(&'static str, &'static Label)> {
    let l = labels();
    l.section_order
        .iter()
        .filter_map(|key| l.section.get(key).map(|label| (key.as_str(), label)))
        .collect()
}

/// 列挙の値の表示名（`enum_label("budget_kind", "mass")`）。型の名前は [`Labels::enums`] のキー。
pub fn enum_label(ty: &str, value: &str) -> Option<&'static Label> {
    labels().enums.get(ty)?.get(value)
}

/// 機器の状態の表示名（無ければ None。プロジェクト独自の状態は ID をそのまま出す）。
pub fn state_label(state: &str) -> Option<&'static Label> {
    labels().state.get(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_parse_and_cover_builtin_states() {
        let parsed: Result<Labels, _> = toml::from_str(LABELS_TOML);
        assert!(parsed.is_ok(), "{parsed:?}");
        for kind in super::super::kinds().values() {
            for state in &kind.states {
                assert!(state_label(state).is_some(), "{state}");
            }
        }
        for attr in super::super::attributes().values() {
            assert!(attr.label_en.is_some(), "{}", attr.id);
            if let Some(section) = &attr.section {
                assert!(labels().section.contains_key(section), "{section}");
            }
        }
        assert!(super::super::kinds().values().all(|k| k.label_en.is_some()));
        assert_eq!(
            labels().enums["maturity"]["new"].label_en.as_deref(),
            Some("New")
        );
    }

    #[test]
    fn enum_order_lists_every_value_of_its_type_once() {
        for (ty, order) in &labels().enum_order {
            let mut sorted = order.clone();
            sorted.sort();
            let keys: Vec<String> = labels().enums[ty].keys().cloned().collect();
            assert_eq!(sorted, keys, "enum_order.{ty}");
        }
        // 成熟度は選択肢の順（既製品 → 改修品 → 新規開発）で、どれもモデルの Maturity として読める
        let maturity = &labels().enum_order["maturity"];
        assert_eq!(maturity, &["off_the_shelf", "modified", "new"]);
        for m in maturity {
            assert!(m.parse::<crate::design::Maturity>().is_ok(), "{m}");
        }
    }
}
