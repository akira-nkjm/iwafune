//! 設定のうち、値が文字列の一覧になる欄の表（設定のパスの形 → 一覧）。**どの設定が一覧かはここだけに書く。**
//!
//! まだ書かれていない欄を人が打った文字列から読むとき（`fune set`・`POST /ops/from_text`）、値の型は文字列から推し量らず、
//! 欄の形で決める。一覧の欄はここに挙げ、読み方は [`crate::value::setting_from_text`] の一覧（カンマ区切り、または TOML の配列）。
//! 表の網羅は、設計データの一覧の値がどれもこの表に当たることを確かめるテスト（fune-service）が見る。

use super::setting_dims::{ANY, pattern_matches};
use crate::path::{SettingPath, SettingScope};
use crate::value::SettingValue;

/// 一覧の欄のパスの形（[`ANY`] は任意の 1 区切り）。
pub const LIST_SETTINGS: &[(SettingScope, &[&str])] = &[
    // 熱の節点にまとめる部品の ID（`[nodes]` の `<節点> = ["A", "B"]`）
    (SettingScope::Analysis, &["thermal", "nodes", ANY]),
    // 通信の解析で使う地上局の ID
    (SettingScope::Analysis, &["comm", "stations"]),
];

impl SettingPath {
    /// 値が文字列の一覧になる欄か。
    pub fn is_list(&self) -> bool {
        LIST_SETTINGS
            .iter()
            .any(|(scope, pattern)| pattern_matches(*scope, pattern, self))
    }
}

/// 一覧の欄なら、値の形の見本（空の一覧。まだ書かれていない欄を文字列から読むときの型だけに使う。既定値ではない）。
pub fn list_field_shape(path: &SettingPath) -> Option<SettingValue> {
    path.is_list().then(|| SettingValue::List(Vec::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_fields_are_matched_by_path_shape() {
        assert!(SettingPath::analysis(&["thermal", "nodes", "BUS"]).is_list());
        assert!(SettingPath::analysis(&["comm", "stations"]).is_list());
        assert!(!SettingPath::analysis(&["thermal", "nodes"]).is_list());
        assert!(!SettingPath::analysis(&["thermal", "heat_node", "BUS"]).is_list());
        assert!(!SettingPath::conditions(&["comm", "stations"]).is_list());
        assert_eq!(
            list_field_shape(&SettingPath::analysis(&["comm", "stations"])),
            Some(SettingValue::List(vec![]))
        );
    }
}
