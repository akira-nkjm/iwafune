//! 通信の回線・ローカルのカタログ・プロジェクトの編集に打った引数（文字列のまま。`CommandText::{LinkAdd, CatalogLocalAdd, ProjectSet}`）。
//! 読み方は `edit::{link, catalog, project}`（CLI の引数も GUI の欄もそこを通る）。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// `fune link add <ID> --direction down --part STX --band s_band` の引数（`analysis/comm.toml` の `[link.<ID>]`。読み込める最小の 3 つ）。
/// 前後の空白は除く。向きは `down` / `up`。部品・帯域の実在は適用のときの検査が見る。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct LinkAddText {
    pub id: String,
    /// `down`（下り）か `up`（上り）
    pub direction: String,
    /// 衛星側の送受信機の部品 ID
    pub part: String,
    /// 地上局側の帯域（`conditions/ground/<ID>.toml` の `[link.<帯域>]`）
    pub band: String,
}

/// `fune catalog source add-local <名前> --path <ディレクトリ>` の引数（fune.toml にこのプロジェクトの中のカタログを足す）。
/// 前後の空白は除く。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct CatalogLocalAddText {
    /// カタログの名前（品は `<名前>/<ID>` で入る）
    pub name: String,
    /// プロジェクトの中のディレクトリ（`catalog`・`vendor/parts`）
    pub path: String,
}

/// `fune project set <フィールド> <値>`・`fune project unset <フィールド>` の引数（`fune.toml` の `[project]`）。
/// `field` は `name` か `label`。`value` が無い（省略・null・空）と消す（`label` だけ。`name` は消せない）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(optional_fields = nullable)]
pub struct ProjectSetText {
    pub field: String,
    pub value: Option<String>,
}
