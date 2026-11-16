//! 宇宙天気（CelesTrak SW-All）の取得・状態・読み込み・日付からの引き方。
//!
//! 表は毎日更新されるので、決めた SHA-256 は持たない。取得のたびに取得日時と SHA-256 を `SW-All.json` に記録し、
//! 読み込んだ表と一緒に版（[`SpaceWeatherVersion`]）として返す。解析はこの版を結果に残す。
//! 取得は `curl` で行い、表として読めることを確かめてから置き換える（取得元が混んでいて HTML が返っても壊れない）。

use super::verify::{Stamp, StampCache};
use super::{io_error, sha256_hex};
use crate::dto::{DataItem, SpaceWeatherLookup, SpaceWeatherVersion};
use crate::error::{Result, ServiceError};
use sat_astro::atmosphere::space_weather::{SolarActivity, SpaceWeatherTable, space_weather_at};
use sat_astro::time::Epoch;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

/// データの ID（`fune data fetch space_weather`、`POST /data/space_weather/fetch`）。
pub const ID: &str = "space_weather";
/// 取得元（CelesTrak の 1957 年からの全期間。実績・日ごとの予測・月ごとの予測）。
const SOURCE_URL: &str = "https://celestrak.org/SpaceData/SW-All.csv";
const FILE: &str = "SW-All.csv";
/// 取得の記録（取得元・取得日時・SHA-256）。
const RECORD: &str = "SW-All.json";

/// 取得の記録（`SW-All.json`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FetchRecord {
    source_url: String,
    fetched_at: String,
    sha256: String,
}

/// 読み込んだ表と、その版。
#[derive(Debug, Clone)]
pub struct LoadedSpaceWeather {
    /// 日ごとの表
    pub table: SpaceWeatherTable,
    /// 版（解析の結果に残す）
    pub version: SpaceWeatherVersion,
}

/// 読み込んだ表の覚え（ファイルの印が変わったら読み直す）。
static LOADED: StampCache<Arc<LoadedSpaceWeather>> = StampCache::new();

/// キャッシュの表を読む。ファイルが無ければ `None`、読めなければ誤り。
/// 解析は、これが `None` か誤りなら太陽活動のプリセットを使い、版を残さない。
pub fn loaded() -> Result<Option<Arc<LoadedSpaceWeather>>> {
    let path = super::cache_dir()?.join(FILE);
    let Ok(meta) = std::fs::metadata(&path) else {
        return Ok(None);
    };
    LOADED
        .get_or_check(Stamp::of(&path, &meta), || load(&path).map(Arc::new))
        .map(Some)
}

fn load(path: &Path) -> Result<LoadedSpaceWeather> {
    let bytes = std::fs::read(path).map_err(|e| io_error(path, e))?;
    let table = SpaceWeatherTable::from_csv_bytes(&bytes)
        .map_err(|e| ServiceError::Data(format!("{}: {e}", path.display())))?;
    let record = read_record(&path.with_file_name(RECORD));
    let coverage = table.coverage();
    let version = SpaceWeatherVersion {
        source_url: record
            .as_ref()
            .map_or_else(|| SOURCE_URL.to_owned(), |r| r.source_url.clone()),
        fetched_at: record.as_ref().map(|r| r.fetched_at.clone()),
        sha256: sha256_hex(&bytes),
        recorded_sha256: record.map(|r| r.sha256),
        first_date: date_text(coverage.first),
        last_observed_date: coverage.last_observed.map(date_text),
        last_date: date_text(coverage.last),
    };
    Ok(LoadedSpaceWeather { table, version })
}

/// 取得の記録を読む（無い・読めなければ `None`。手で置いた表も使えるようにする）。
fn read_record(path: &Path) -> Option<FetchRecord> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// 宇宙天気の表の状態。
pub(super) fn item(dir: &Path) -> Result<DataItem> {
    let path = dir.join(FILE);
    let size = std::fs::metadata(&path).ok().map(|m| m.len());
    let (loaded, install_error) = match loaded() {
        Ok(l) => (l, None),
        Err(e) => (None, Some(e.to_string())),
    };
    let version = loaded.as_ref().map(|l| &l.version);
    Ok(DataItem {
        id: ID.to_owned(),
        path: path.display().to_string(),
        present: size.is_some(),
        size: size.and_then(|b| crate::units::auto_from(b as f64, "B", "Information")),
        sha256_expected: version.and_then(|v| v.recorded_sha256.clone()),
        verified: version.and_then(|v| v.recorded_sha256.as_ref().map(|r| *r == v.sha256)),
        installed: loaded.is_some(),
        install_error,
        source: SOURCE_URL.to_owned(),
        fetched_at: version.and_then(|v| v.fetched_at.clone()),
    })
}

/// 表を取得してキャッシュ `dir` に置く。一時ディレクトリ `tmp` に落とし、表として読めることを確かめてから、
/// 表と取得の記録（取得日時・SHA-256）を置き換える。
pub(super) fn fetch(tmp: &Path, dir: &Path) -> Result<()> {
    let downloaded = tmp.join(FILE);
    download(&downloaded)?;
    let bytes = std::fs::read(&downloaded).map_err(|e| io_error(&downloaded, e))?;
    SpaceWeatherTable::from_csv_bytes(&bytes).map_err(|e| {
        ServiceError::Data(format!(
            "The downloaded data is not a space-weather table ({e})"
        ))
    })?;
    let record = FetchRecord {
        source_url: SOURCE_URL.to_owned(),
        fetched_at: now_iso(),
        sha256: sha256_hex(&bytes),
    };
    let record_tmp = tmp.join(RECORD);
    let json =
        serde_json::to_string_pretty(&record).map_err(|e| ServiceError::Internal(e.to_string()))?;
    std::fs::write(&record_tmp, json).map_err(|e| io_error(&record_tmp, e))?;
    let target = dir.join(FILE);
    std::fs::rename(&downloaded, &target).map_err(|e| io_error(&target, e))?;
    let record_path = dir.join(RECORD);
    std::fs::rename(&record_tmp, &record_path).map_err(|e| io_error(&record_path, e))
}

fn download(target: &Path) -> Result<()> {
    let output = Command::new("curl")
        .args(["--fail", "--silent", "--show-error", "--location"])
        .args(["--max-time", "300", "--output"])
        .arg(target)
        .arg(SOURCE_URL)
        .output()
        .map_err(|e| ServiceError::Data(format!("Cannot start curl: {e}")))?;
    if !output.status.success() {
        return Err(ServiceError::Data(format!(
            "Cannot fetch {SOURCE_URL}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

/// 日付から宇宙天気を引く（`GET /data/space_weather?date=…&activity=…`、`fune data space-weather <日付>`）。
/// `date` は ISO 8601（日付だけでも可）、`activity` は表の範囲の外で使う太陽活動（`low`・`moderate`・`high`。
/// 省略時は中程度）。表が無い・読めないときもプリセットで答え、`version` を空にする。
pub fn lookup(date: &str, activity: Option<&str>) -> Result<SpaceWeatherLookup> {
    let epoch = Epoch::parse(date).map_err(|e| ServiceError::InvalidInput(e.to_string()))?;
    let fallback = parse_activity(activity)?;
    let (loaded, load_error) = match loaded() {
        Ok(l) => (l, None),
        Err(e) => (None, Some(e.to_string())),
    };
    let weather = space_weather_at(loaded.as_deref().map(|l| &l.table), epoch, fallback);
    Ok(SpaceWeatherLookup {
        epoch,
        fallback,
        weather,
        version: loaded.map(|l| l.version.clone()),
        load_error,
    })
}

/// 打った太陽活動の綴り → [`SolarActivity`]。
fn parse_activity(text: Option<&str>) -> Result<SolarActivity> {
    let Some(text) = text.map(str::trim).filter(|t| !t.is_empty()) else {
        return Ok(SolarActivity::default());
    };
    serde_json::from_value(serde_json::Value::String(text.to_ascii_lowercase())).map_err(|_| {
        ServiceError::InvalidInput(format!(
            "Solar activity must be low, moderate or high: {text}"
        ))
    })
}

/// 日の番号 → `YYYY-MM-DD`。
fn date_text(day_number: i64) -> String {
    let (y, m, d) = Epoch::calendar_of_day_number(day_number);
    format!("{y:04}-{m:02}-{d:02}")
}

/// 今の時刻（UTC、ISO 8601）。
fn now_iso() -> String {
    let unix_s = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    Epoch::from_calendar(1970, 1, 1, 0, 0, 0.0)
        .plus_seconds(unix_s as f64)
        .to_iso()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_text() {
        assert_eq!(parse_activity(None).unwrap(), SolarActivity::Moderate);
        assert_eq!(parse_activity(Some(" High ")).unwrap(), SolarActivity::High);
        assert!(parse_activity(Some("extreme")).is_err());
    }

    #[test]
    fn dates_and_clock() {
        assert_eq!(date_text(Epoch::day_number_of(2003, 10, 29)), "2003-10-29");
        let now = now_iso();
        assert!(now.ends_with('Z') && Epoch::parse(&now).is_ok(), "{now}");
    }

    #[test]
    fn load_records_the_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE);
        let csv = "DATE,AP_AVG,F10.7_OBS,F10.7_DATA_TYPE,F10.7_OBS_CENTER81\n\
                   2030-01-01,5,100.0,OBS,101.0\n2030-01-02,6,102.0,PRD,103.0\n";
        std::fs::write(&path, csv).unwrap();
        let loaded = load(&path).unwrap();
        let v = &loaded.version;
        assert_eq!(v.sha256, sha256_hex(csv.as_bytes()));
        assert_eq!(
            (v.fetched_at.as_deref(), v.recorded_sha256.as_deref()),
            (None, None)
        );
        assert_eq!(v.first_date, "2030-01-01");
        assert_eq!(v.last_observed_date.as_deref(), Some("2030-01-01"));
        assert_eq!(v.last_date, "2030-01-02");
        let record = FetchRecord {
            source_url: SOURCE_URL.into(),
            fetched_at: "2030-01-01T00:00:00Z".into(),
            sha256: v.sha256.clone(),
        };
        std::fs::write(
            dir.path().join(RECORD),
            serde_json::to_string(&record).unwrap(),
        )
        .unwrap();
        let v = load(&path).unwrap().version;
        assert_eq!(v.fetched_at.as_deref(), Some("2030-01-01T00:00:00Z"));
        assert_eq!(v.recorded_sha256.as_ref(), Some(&v.sha256));
        std::fs::write(&path, "<html>busy</html>").unwrap();
        assert!(load(&path).is_err());
    }
}
