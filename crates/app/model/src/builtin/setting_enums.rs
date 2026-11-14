//! 設定の列挙の欄の表（設定のパスの形 → 列挙の型の名前）。**設定のどの欄が列挙で、表示名をどの型で引くかはここだけに書く。**
//!
//! 型の名前は labels.toml の `[enum.<型>]`（[`enum_label`]）。パスの形の当て方は
//! [`pattern_matches`](crate::design::setting_dims::pattern_matches)（[`ANY`] は任意の 1 区切り）。
//! 真偽の値は欄によらず `bool` の表示名（はい・いいえ）で引く。

use super::labels::{Label, enum_label};
use crate::design::setting_dims::{ANY, pattern_matches};
use crate::path::{SettingPath, SettingScope};
use crate::value::SettingValue;

/// 表の 1 行（パスの形と列挙の型の名前）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingEnum {
    pub scope: SettingScope,
    /// `scope` の後ろの区切りの形（[`ANY`] を含む）
    pub pattern: &'static [&'static str],
    /// 列挙の型の名前（labels.toml の `[enum.<型>]`）
    pub ty: &'static str,
}

const fn row(
    scope: SettingScope,
    pattern: &'static [&'static str],
    ty: &'static str,
) -> SettingEnum {
    SettingEnum { scope, pattern, ty }
}

/// 列挙の欄の表（`OrbitSettings.propagator`・`central`、`AttitudeLawSetting.law`、`CommLinkSettings.direction`、
/// `ThermalCaseSettings.preset`）。
pub const SETTING_ENUMS: &[SettingEnum] = &[
    row(
        SettingScope::Conditions,
        &["orbit", "propagator"],
        "propagator",
    ),
    row(
        SettingScope::Conditions,
        &["orbit", "central"],
        "central_body",
    ),
    row(
        SettingScope::Analysis,
        &["aocs", "law", ANY, "law"],
        "attitude_law",
    ),
    row(
        SettingScope::Analysis,
        &["comm", "link", ANY, "direction"],
        "link_direction",
    ),
    row(
        SettingScope::Analysis,
        &["thermal", "case", ANY, "preset"],
        "thermal_preset",
    ),
];

/// 設定のパスの列挙の型の名前（列挙の欄でなければ None）。
pub fn setting_enum(path: &SettingPath) -> Option<&'static str> {
    SETTING_ENUMS
        .iter()
        .find(|e| pattern_matches(e.scope, e.pattern, path))
        .map(|e| e.ty)
}

/// 設定の値の表示名（列挙の欄の文字の値と、真偽の値）。表示名が無ければ None（文字のまま見せる）。
pub fn setting_value_label(path: &SettingPath, value: &SettingValue) -> Option<&'static Label> {
    match value {
        SettingValue::Text(text) => enum_label(setting_enum(path)?, text),
        SettingValue::Bool(b) => enum_label("bool", if *b { "true" } else { "false" }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::labels;

    #[test]
    fn every_type_has_labels_and_paths_are_unique() {
        for (i, e) in SETTING_ENUMS.iter().enumerate() {
            assert!(labels().enums.contains_key(e.ty), "enum.{}", e.ty);
            for other in &SETTING_ENUMS[i + 1..] {
                assert!(
                    !(e.scope == other.scope && e.pattern == other.pattern),
                    "{:?}",
                    e.pattern
                );
            }
        }
    }

    #[test]
    fn labels_enum_text_and_bool_values() {
        let law = SettingPath::analysis(&["aocs", "law", "MISSION", "law"]);
        let text = |s: &str| SettingValue::Text(s.to_owned());
        let nadir = setting_value_label(&law, &text("nadir_pointing"));
        assert_eq!(nadir.map(|l| l.label.as_str()), Some("地心指向"));
        assert_eq!(
            nadir.and_then(|l| l.label_en.as_deref()),
            Some("Nadir pointing")
        );
        let propagator = SettingPath::conditions(&["orbit", "propagator"]);
        assert!(setting_value_label(&propagator, &text("j2")).is_some());
        // 表に無い値・列挙でない欄の文字は表示名を持たない
        assert_eq!(setting_value_label(&propagator, &text("rk4")), None);
        let label = SettingPath::conditions(&["orbit", "label"]);
        assert_eq!(setting_value_label(&label, &text("j2")), None);
        let sso = SettingPath::conditions(&["orbit", "sun_synchronous"]);
        let yes = setting_value_label(&sso, &SettingValue::Bool(true));
        assert_eq!(yes.and_then(|l| l.label_en.as_deref()), Some("Yes"));
    }
}
