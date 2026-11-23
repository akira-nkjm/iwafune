//! 地上局の表示（`GET /design` の `stations`）と、`fune ground add` に打った引数（`CommandText::GroundAdd`。読み方は `edit::ground`）。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 地上局 1 つ（`conditions/ground/<ID>.toml`。ID 順）。値は設定（`SettingView`。`group` が `conditions.ground.<ID>`）で見せる。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct GroundStationView {
    pub id: String,
    /// 表示名（無ければ null）
    pub label: Option<String>,
    /// 設定ファイル（ルートからの相対パス）
    pub file: String,
    /// この局の設定（`SettingView.group`。`conditions.ground.<ID>`）。画面は設定の表をこの値で引く
    pub group: String,
    /// 通信の解析の `stations` に挙げられているか（挙げられていれば消せない。`stations` が空なら全局を使うので偽）
    pub in_use: bool,
}

/// `fune ground add <ID> --label … --latitude "35.7 deg" …` の引数（文字列のまま）。
/// 書かなかった旗（省略・null・空の文字列）は書かない。量（`"35.7 deg"`）を読むのは service。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(optional_fields = nullable)]
pub struct GroundAddText {
    pub id: String,
    pub label: Option<String>,
    /// 緯度（`"35.7126 deg"`）
    pub latitude: Option<String>,
    /// 経度（`"139.762 deg"`）
    pub longitude: Option<String>,
    /// 標高（`"40 m"`）
    pub altitude: Option<String>,
    /// 運用できる最低仰角（`"10 deg"`）
    pub min_elevation: Option<String>,
}
