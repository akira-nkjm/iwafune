//! 共有カタログの取得・更新の入出力（`fune catalog fetch|update`・`POST /catalog/fetch`・`POST /catalog/update`）。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 取得の指定（`fune catalog fetch [--locked]`・`POST /catalog/fetch`）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CatalogFetchRequest {
    /// true なら catalog.lock を書き換えない（lock が無い・fune.toml と違えば誤り。CI 向け）
    #[serde(default)]
    pub locked: bool,
}

/// 更新の指定（`fune catalog update <名前>`・`POST /catalog/update`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CatalogUpdateRequest {
    /// 更新する共有カタログの名前（fune.toml の `[[catalog]]` の `name`）
    pub name: String,
}

/// 取得・更新の結果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CatalogSyncResult {
    /// 共有カタログごとの結果（fune.toml の順）
    pub catalogs: Vec<CatalogSyncEntry>,
    /// catalog.lock を書き換えたか
    pub lock_changed: bool,
    /// 前後で変わった共有カタログの品（ID 順）
    pub items: Vec<CatalogItemChange>,
}

/// 共有カタログ 1 つの取得の結果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CatalogSyncEntry {
    pub name: String,
    pub git: String,
    pub rev: Option<String>,
    /// 今の commit（lock に書いたもの）
    pub commit: String,
    /// 前の lock の commit（lock に記録が無かったら None。変わっていなければ `commit` と同じ）
    pub previous: Option<String>,
}

/// 品の変わり方。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum CatalogItemChangeKind {
    Added,
    Removed,
    Changed,
}

/// 変わった品 1 つ。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CatalogItemChange {
    /// 出所付きの ID（`lab/rw_18mNms`）
    pub id: String,
    pub change: CatalogItemChangeKind,
    /// 変わった欄と値（欄の名前の順。追加・削除の品は空）
    pub values: Vec<CatalogValueChange>,
}

/// 変わった欄 1 つ（フィールドは `label` など、属性の値は `attr.<属性>`）。表示文字列は値の表示と同じ整形。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CatalogValueChange {
    pub key: String,
    pub before: Option<String>,
    pub after: Option<String>,
}
