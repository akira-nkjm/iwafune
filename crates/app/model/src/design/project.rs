//! プロジェクトの設定（`fune.toml`）。serde の形がそのままファイルの形。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// `fune.toml` 全体。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Project {
    /// `[project]`
    #[serde(rename = "project")]
    pub meta: ProjectMeta,
    /// `[modes]`: モードの表示順（省略すると ID 順）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modes: Option<ModeOrder>,
    /// `[[catalog]]`: カタログの出所
    #[serde(default, rename = "catalog")]
    pub catalogs: Vec<CatalogSource>,
}

/// `[project]`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ProjectMeta {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// schema の版（現在は 2）
    #[serde(default = "default_schema")]
    pub schema: u32,
    /// 書き出す TOML の版（`"1.0"`）
    #[serde(default = "default_toml")]
    pub toml: String,
}

impl Default for ProjectMeta {
    fn default() -> Self {
        Self {
            name: String::new(),
            label: None,
            schema: SCHEMA_VERSION,
            toml: default_toml(),
        }
    }
}

/// 現在の schema の版。
pub const SCHEMA_VERSION: u32 = 2;

fn default_schema() -> u32 {
    SCHEMA_VERSION
}

fn default_toml() -> String {
    "1.0".to_string()
}

/// カタログの出所（`[[catalog]]`）。`path` ならリポジトリ内、`git` なら共有リポジトリ（まだ読まない）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct CatalogSource {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
}

impl CatalogSource {
    /// 書き換えられる（このプロジェクトの中の `[[catalog]] path = …`）なら、そのディレクトリ。共有カタログ（`git = …`）は None。
    /// **カタログの出所が書き換えられるかの判定はここだけ**（store の `catalog_location`・fune-ops・service の一覧・
    /// 公開できない出典の検査が使う）。
    pub fn local_dir(&self) -> Option<&str> {
        self.path.as_deref()
    }
}

/// モードの表示順。指定していないモードは後ろに ID 順で並べる。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ModeOrder {
    /// 表示する順のモード ID。存在しない ID は検査で警告する。
    #[serde(default)]
    pub order: Vec<String>,
}

impl super::Design {
    /// 指定した一覧、未指定の ID 順でモードを返す。欠けた ID と重複は表示から除く。
    pub fn ordered_modes(&self) -> Vec<&super::Mode> {
        let mut seen = std::collections::BTreeSet::new();
        let listed = self.project.modes.iter().flat_map(|m| &m.order);
        listed
            .map(String::as_str)
            .chain(self.modes.keys().map(crate::id::Id::as_str))
            .filter(|id| seen.insert(*id))
            .filter_map(|id| self.modes.get(id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design::Mode;
    use crate::{Design, Id, check};

    #[test]
    fn display_order_handles_listed_unlisted_missing_and_duplicates() {
        let mut design = Design::default();
        for id in ["SAFE", "NOMINAL", "MISSION", "DOWNLINK"] {
            design.modes.insert(
                Id::new(id),
                Mode {
                    id: Id::new(id),
                    ..Default::default()
                },
            );
        }
        let ids = |d: &Design| {
            d.ordered_modes()
                .iter()
                .map(|m| m.id.to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(&design), ["DOWNLINK", "MISSION", "NOMINAL", "SAFE"]);
        design.project.modes = Some(ModeOrder {
            order: vec![
                "NOMINAL".into(),
                "MISSING".into(),
                "MISSION".into(),
                "NOMINAL".into(),
            ],
        });
        assert_eq!(ids(&design), ["NOMINAL", "MISSION", "DOWNLINK", "SAFE"]);
        let diagnostics = check(&design);
        let missing: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.code == "mode.order_missing")
            .collect();
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].severity, crate::Severity::Warning);
        assert!(missing[0].message.contains("MISSING"));
        let json = serde_json::to_value(&design.project).unwrap();
        assert_eq!(json["modes"]["order"][0], "NOMINAL");
        assert!(
            serde_json::from_value::<Project>(
                serde_json::json!({"project":{"name":"sample"}, "modes":{"unknown":[]}})
            )
            .is_err()
        );
    }
}
