//! プロジェクトの設定（`fune.toml`）。serde の形がそのままファイルの形。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// `fune.toml` 全体。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Project {
    /// `[project]`
    #[serde(rename = "project")]
    pub meta: ProjectMeta,
    /// `[groups]`: サブシステムの表示名（任意）
    #[serde(default)]
    pub groups: BTreeMap<String, String>,
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
