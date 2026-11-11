//! 外部データ（DE440 など）の API の型（`GET /data/status`、`POST /data/{id}/fetch`）。

use super::QuantityView;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 外部データの状態。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct DataStatus {
    /// キャッシュのディレクトリ
    pub cache_dir: String,
    /// 解析が使う天体の暦（DE440 を読み込んでいれば `de440`、無ければ解析式 `analytic`）
    pub ephemeris: sat_astro::ephem::EphemerisSource,
    pub items: Vec<DataItem>,
}

/// 外部データ 1 つ。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct DataItem {
    /// 例: `"de440"`
    pub id: String,
    pub path: String,
    /// ファイルがあるか
    pub present: bool,
    /// 大きさ（ファイルが無ければ null。`display` は `"20.7 MB"`）
    pub size: Option<QuantityView>,
    pub sha256_expected: String,
    /// SHA-256 が一致したか（ファイルが無ければ null）
    pub verified: Option<bool>,
    /// このプロセスで読み込んで解析に使える状態か
    pub installed: bool,
    /// 読み込みに失敗した理由
    pub install_error: Option<String>,
    /// 取得元（`gh release download …`）
    pub source: String,
}
