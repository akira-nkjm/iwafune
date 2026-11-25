//! 外部データ（DE440・宇宙天気）の API の型（`GET /data/status`、`POST /data/{id}/fetch`、`GET /data/space_weather`）。

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
    /// 例: `"de440"`・`"space_weather"`
    pub id: String,
    pub path: String,
    /// ファイルがあるか
    pub present: bool,
    /// 大きさ（ファイルが無ければ null。`display` は `"20.7 MB"`）
    pub size: Option<QuantityView>,
    /// 期待する SHA-256。DE440 は決めてある値、宇宙天気は取得したときに記録した値（記録が無ければ null）
    pub sha256_expected: Option<String>,
    /// SHA-256 が一致したか（ファイルが無ければ null）
    pub verified: Option<bool>,
    /// このプロセスで読み込んで解析に使える状態か（宇宙天気は表として読めたか）
    pub installed: bool,
    /// 読み込みに失敗した理由
    pub install_error: Option<String>,
    /// 取得元（`gh release download …`・URL）
    pub source: String,
    /// 取得した日時（ISO 8601、UTC）。記録が無ければ null（DE440 は版が決まっているので記録しない）
    pub fetched_at: Option<String>,
    /// 何に使うか（利用者に見せる文。英語）
    pub used_by: String,
    /// 出所と利用条件（利用者に見せる文。英語。無ければ null）
    pub license: Option<String>,
}

/// 宇宙天気の表の版（解析の結果に残す）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct SpaceWeatherVersion {
    /// 取得元の URL
    pub source_url: String,
    /// 取得した日時（ISO 8601、UTC）。手で置いた表で記録が無ければ null
    pub fetched_at: Option<String>,
    /// 読み込んだ表の SHA-256
    pub sha256: String,
    /// 取得したときに記録した SHA-256（記録が無ければ null）
    pub recorded_sha256: Option<String>,
    /// 表の最初の日（`YYYY-MM-DD`）
    pub first_date: String,
    /// 実績値（観測・補間）の最後の日
    pub last_observed_date: Option<String>,
    /// 予測を含む最後の日
    pub last_date: String,
}

/// 宇宙天気の問い合わせ（`GET /data/space_weather?date=2003-10-29&activity=moderate`。CLI は `fune data space-weather <日付> --activity`）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SpaceWeatherQuery {
    /// ISO 8601 の元期（日付だけでも可）
    pub date: String,
    /// 表の範囲の外で使う太陽活動（`low`・`moderate`・`high`）
    pub activity: Option<String>,
}

/// 日付で引いた宇宙天気（`GET /data/space_weather`、`fune data space-weather`）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SpaceWeatherLookup {
    /// 引いた元期
    pub epoch: sat_astro::time::Epoch,
    /// 表の範囲の外で使った太陽活動のプリセット
    pub fallback: sat_astro::atmosphere::space_weather::SolarActivity,
    /// NRLMSISE-00 の入力（F10.7・81 日平均・Ap・3 時間 Ap の履歴）と出所
    pub weather: sat_astro::atmosphere::space_weather::DatedSpaceWeather,
    /// 使った表の版（表が無い・読めなければ null。そのときはプリセット）
    pub version: Option<SpaceWeatherVersion>,
    /// 表を読めなかった理由
    pub load_error: Option<String>,
}
