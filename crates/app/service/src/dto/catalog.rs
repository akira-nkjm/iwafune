//! カタログ品の表示用の型（`GET /design` の `catalog`、`fune catalog ls|show`）。

use super::ValueView;
use fune_model::design::{Maturity, NetKind, SharedCatalogStatus};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// カタログの出所 1 つ（fune.toml の `[[catalog]]`。`ProjectInfo.catalogs`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CatalogSourceView {
    /// カタログ名（品の ID の `<カタログ名>/` の部分。`CatalogItemView.scope` と同じ値）
    pub name: String,
    /// このプロジェクトの中（`path = …`）なら true。共有カタログ（`git = …`）は読み取り専用で false。
    /// 判定は `fune_model::design::CatalogSource::local_dir`（品の `editable` と同じ）
    pub editable: bool,
    /// 共有カタログ（`git = …`）なら、lock の commit と読み込みの状態（判定は `Design::shared_catalog_statuses`）
    pub shared: Option<SharedCatalogStatus>,
}

/// カタログ品 1 つ。値は部品の値と同じ形（`ValueView`）で、表示文字列は service が作る。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CatalogItemView {
    /// 出所付きの ID（`"local/rw_10mNms"`）
    pub id: String,
    /// カタログ名（fune.toml の `[[catalog]]` の `name`。例: `"local"`）
    pub scope: String,
    /// カタログの中の名前（例: `"rw_10mNms"`）
    pub name: String,
    /// このプロジェクトのカタログ（`[[catalog]] path = …`）の品なら true。共有カタログ（`git = …`）の品は読み取り専用で false
    pub editable: bool,
    pub kind: String,
    pub label: Option<String>,
    /// 英語の表示名
    pub label_en: Option<String>,
    pub manufacturer: Option<String>,
    pub part_number: Option<String>,
    pub maturity: Option<Maturity>,
    /// タグ（書いた順）
    pub tags: Vec<String>,
    /// タグを `fune catalog set <品> tags <値>` に打つ形（カンマ区切り）にした文字列。GUI の欄に出す（読み方は service が持つ）
    pub tags_text: String,
    /// データシートの出典の ID（`sources/<ID>`。URL・版・sha256 はその出典にある）
    pub datasheet: Option<String>,
    /// データシートの出典の参照先が Web の URL なら、その URL（GUI のリンク）
    pub datasheet_link: Option<String>,
    /// 出典を書いていない値に当てる既定の出典（`sources/<ID>`）
    pub source: Option<String>,
    /// ポートの雛形（ポート名 → ネットの種類）
    pub ports: BTreeMap<String, NetKind>,
    /// 機器の状態の語彙（kind から）
    pub states: Vec<String>,
    /// 属性 ID → 品に書かれた値（部品の `PartRow.values` と同じ形）
    pub values: BTreeMap<String, ValueView>,
    /// 属性 → 状態（空文字なら属性全体）→ パラメーターパス（`local/rw_10mNms.power.on`）
    pub paths: BTreeMap<String, BTreeMap<String, String>>,
    /// この品を選択中か候補に入れている部品の ID
    pub users: Vec<String>,
}
