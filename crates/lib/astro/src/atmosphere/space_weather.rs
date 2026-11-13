//! 宇宙天気（F10.7・Ap）の日ごとの表と、日付からの引き方。**NRLMSISE-00 に渡す宇宙天気を日付で決めるのはここだけ。**
//!
//! - 表 [`SpaceWeatherTable`] — CelesTrak の SW-All（CSV。`csv` モジュールの説明）をバイト列から読む。ファイルは読まない。
//! - 引き方 [`space_weather_at`] — 元期から、前日の F10.7・81 日中心平均・日平均 Ap・3 時間 Ap の履歴を返す。
//! - 範囲の外 — 表の実績値（OBS・INT）と予測値（PRD・PRM）の範囲の外や欠けた値は、太陽活動のプリセット
//!   [`SolarActivity`] に切り替える。どの値をどこから取ったかは [`WeatherSource`] で返す。
//! - 手で与えた値 — [`DatedSpaceWeather::manual`]。呼び出し側は、手で与えた値があればそれを優先し、
//!   無いときだけ [`space_weather_at`] を使う（手の値が上書き）。

mod csv;

use super::AtmosphereModel;
use super::nrlmsise00::{GeomagneticActivity, SpaceWeather};
use crate::time::Epoch;
use serde::{Deserialize, Serialize};

pub use csv::SpaceWeatherParseError;

/// 太陽活動の条件（表の範囲の外で使うプリセット）。
///
/// 値は ECSS-E-ST-10-04C（2008-11-15）Table 6-3「Reference fixed index values」の長期（27 日以上）の値:
/// 低 F10.7 = F10.7(81 日) = 65・Ap = 0、中 140・140・15、高 250・250・45。
/// 同じ表の短期（日ごと）の高い値（F10.7 = 300・Ap = 240）は長期の平均と組み合わせないと規定されているので、
/// 寿命や高度低下の見積もりのプリセットには使わない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SolarActivity {
    /// 低い太陽活動
    Low,
    /// 中程度の太陽活動（既定）
    #[default]
    Moderate,
    /// 高い太陽活動（長期の値）
    High,
}

impl SolarActivity {
    /// (F10.7 [sfu], F10.7 の 81 日平均 [sfu], 日平均 Ap)。ECSS-E-ST-10-04C Table 6-3 の長期の値。
    pub const fn indices(self) -> (f64, f64, f64) {
        match self {
            Self::Low => (65.0, 65.0, 0.0),
            Self::Moderate => (140.0, 140.0, 15.0),
            Self::High => (250.0, 250.0, 45.0),
        }
    }
}

/// F10.7 の値の種類（SW-All の `F10.7_DATA_TYPE`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum FluxQualifier {
    /// `OBS`: 観測値
    Observed,
    /// `INT`: 欠測・外れ値の補間（SERENE）
    Interpolated,
    /// `PRD`: 日ごとの予測（45 日先まで）
    DailyPredicted,
    /// `PRM`: 月ごとの予測。その月のすべての日に当てる
    MonthlyPredicted,
}

impl FluxQualifier {
    /// 予測値か（実績値の範囲の外か）。
    pub const fn is_predicted(self) -> bool {
        matches!(self, Self::DailyPredicted | Self::MonthlyPredicted)
    }
}

/// 値の出所。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum WeatherSource {
    /// 表の観測値
    Observed,
    /// 表の補間値
    Interpolated,
    /// 表の日ごとの予測
    DailyPredicted,
    /// 表の月ごとの予測
    MonthlyPredicted,
    /// 太陽活動のプリセット（表の範囲の外か、表に値が無い）
    Preset,
    /// 手で与えた値
    Manual,
}

impl From<FluxQualifier> for WeatherSource {
    fn from(q: FluxQualifier) -> Self {
        match q {
            FluxQualifier::Observed => Self::Observed,
            FluxQualifier::Interpolated => Self::Interpolated,
            FluxQualifier::DailyPredicted => Self::DailyPredicted,
            FluxQualifier::MonthlyPredicted => Self::MonthlyPredicted,
        }
    }
}

/// 表の 1 行（ふつうは 1 日分。月ごとの予測は月の 1 日の行）。値が空欄なら `None`。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpaceWeatherDay {
    /// 日の番号（2000-01-01 = 0。[`Epoch::day_number_of`]）
    pub day_number: i64,
    /// F10.7 の値の種類
    pub qualifier: FluxQualifier,
    /// 3 時間ごとの ap（0〜3 UT, …, 21〜24 UT）
    pub ap_3h: Option<[f64; 8]>,
    /// 日平均 Ap（`AP_AVG`）
    pub ap_daily: Option<f64>,
    /// 観測された F10.7 [sfu]（`F10.7_OBS`。1 AU に補正していない値。NRLMSISE-00 はこちらを使う）
    pub f107_obs: Option<f64>,
    /// F10.7（観測）の 81 日中心平均 [sfu]
    pub f107_obs_center81: Option<f64>,
    /// F10.7（観測）の直前 81 日平均 [sfu]
    pub f107_obs_last81: Option<f64>,
}

/// 宇宙天気の日ごとの表（日付の順）。
#[derive(Debug, Clone, PartialEq)]
pub struct SpaceWeatherTable {
    days: Vec<SpaceWeatherDay>,
}

/// 表の範囲（日の番号。2000-01-01 = 0）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpaceWeatherCoverage {
    /// 最初の日
    pub first: i64,
    /// 実績値（観測・補間）の最後の日。実績値が無ければ `None`
    pub last_observed: Option<i64>,
    /// 最後の日（月ごとの予測はその月の末日まで）
    pub last: i64,
}

impl SpaceWeatherTable {
    /// CelesTrak の SW-All の CSV（バイト列）を読む。
    pub fn from_csv_bytes(bytes: &[u8]) -> Result<Self, SpaceWeatherParseError> {
        csv::parse(bytes).map(|days| Self { days })
    }

    /// 日の並び（日付の順）。
    pub fn days(&self) -> &[SpaceWeatherDay] {
        &self.days
    }

    /// 表の範囲。
    pub fn coverage(&self) -> SpaceWeatherCoverage {
        let first = self.days.first().map_or(0, |d| d.day_number);
        let last_observed = self
            .days
            .iter()
            .rev()
            .find(|d| !d.qualifier.is_predicted())
            .map(|d| d.day_number);
        let last = self.days.last().map_or(first, |d| match d.qualifier {
            FluxQualifier::MonthlyPredicted => last_day_of_month(d.day_number),
            _ => d.day_number,
        });
        SpaceWeatherCoverage {
            first,
            last_observed,
            last,
        }
    }

    /// 日の番号 `day_number` の行。
    ///
    /// その日の行が無ければ、表の中の日（後ろに行がある日）は直前の行を保つ（月ごとの予測の月の中や、
    /// 日ごとの予測と月ごとの予測の間の数日）。最後の行より後は、最後の行が月ごとの予測でその月の中の日だけ。
    /// 表の外なら `None`。
    pub fn day(&self, day_number: i64) -> Option<&SpaceWeatherDay> {
        let i = match self
            .days
            .binary_search_by_key(&day_number, |d| d.day_number)
        {
            Ok(i) => return self.days.get(i),
            Err(0) => return None,
            Err(i) => i,
        };
        let row = self.days.get(i - 1)?;
        let inside = i < self.days.len();
        let in_last_month = row.qualifier == FluxQualifier::MonthlyPredicted
            && day_number <= last_day_of_month(row.day_number);
        (inside || in_last_month).then_some(row)
    }

    /// `epoch` を含む 3 時間区間から 19 区間（57 時間）前までの ap（新しい順）。どれかが欠ければ `None`。
    fn ap_3h_back(&self, epoch: Epoch) -> Option<[f64; 20]> {
        let (day, seconds) = epoch.day_number_and_seconds();
        let slot = ((seconds / 10_800.0).floor() as i64).clamp(0, 7);
        let mut out = [0.0; 20];
        for (back, value) in out.iter_mut().enumerate() {
            let k = day * 8 + slot - back as i64;
            let row = self.days_exact(k.div_euclid(8))?;
            *value = row.ap_3h?[k.rem_euclid(8) as usize];
        }
        Some(out)
    }

    fn days_exact(&self, day_number: i64) -> Option<&SpaceWeatherDay> {
        self.days
            .binary_search_by_key(&day_number, |d| d.day_number)
            .ok()
            .and_then(|i| self.days.get(i))
    }
}

/// ある元期の宇宙天気（NRLMSISE-00 の入力）と、その出所。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DatedSpaceWeather {
    /// 前日の F10.7 [sfu]
    pub f107: f64,
    /// F10.7 の 81 日中心平均 [sfu]（表に無ければ直前 81 日平均）
    pub f107a: f64,
    /// 日平均 Ap
    pub ap: f64,
    /// 3 時間 Ap の履歴（NRLMSISE-00 の `ap_array` と同じ並び: `[日平均, 現時点, -3h, -6h, -9h,
    /// -12〜-33h 平均, -36〜-57h 平均]`）。表に 3 時間値が揃っていなければ `None`
    pub ap_history: Option<[f64; 7]>,
    /// F10.7（前日の値と 81 日平均）の出所
    pub f107_source: WeatherSource,
    /// Ap の出所
    pub ap_source: WeatherSource,
}

impl DatedSpaceWeather {
    /// 太陽活動のプリセット。
    pub const fn preset(activity: SolarActivity) -> Self {
        let (f107, f107a, ap) = activity.indices();
        Self {
            f107,
            f107a,
            ap,
            ap_history: None,
            f107_source: WeatherSource::Preset,
            ap_source: WeatherSource::Preset,
        }
    }

    /// 手で与えた値（F10.7 [sfu]・81 日平均 [sfu]・日平均 Ap）。
    pub const fn manual(f107: f64, f107a: f64, ap: f64) -> Self {
        Self {
            f107,
            f107a,
            ap,
            ap_history: None,
            f107_source: WeatherSource::Manual,
            ap_source: WeatherSource::Manual,
        }
    }

    /// NRLMSISE-00 の入力。3 時間 Ap の履歴があればそれを使う（原典のスイッチ 9 = −1）。
    pub fn nrlmsise00(&self) -> SpaceWeather {
        SpaceWeather {
            f107_daily: self.f107,
            f107_81day_avg: self.f107a,
            geomagnetic: match self.ap_history {
                Some(h) => GeomagneticActivity::ThreeHourlyAp(h),
                None => GeomagneticActivity::DailyAp(self.ap),
            },
        }
    }

    /// 日平均 Ap で固定した大気モデル（抗力・空力トルクの入力。時刻で変わらない）。
    pub const fn atmosphere_model(&self) -> AtmosphereModel {
        AtmosphereModel::Nrlmsise00 {
            f107: self.f107,
            f107a: self.f107a,
            ap: self.ap,
        }
    }
}

/// 元期 `epoch` の宇宙天気を表 `table` から引く。表が無い・範囲の外・値が欠けているときは `fallback` のプリセット。
///
/// F10.7 は前日（UTC）の観測値、81 日平均はその日の中心平均（無ければ直前 81 日平均）。Ap はその日の日平均。
/// F10.7 と Ap は別々に引き、片方だけ欠ければそちらだけプリセットにする。
pub fn space_weather_at(
    table: Option<&SpaceWeatherTable>,
    epoch: Epoch,
    fallback: SolarActivity,
) -> DatedSpaceWeather {
    let mut out = DatedSpaceWeather::preset(fallback);
    let Some(table) = table else {
        return out;
    };
    let (day, _) = epoch.day_number_and_seconds();
    let today = table.day(day);
    let flux = table.day(day - 1).zip(today).and_then(|(prev, today)| {
        let f107 = prev.f107_obs?;
        let f107a = today.f107_obs_center81.or(today.f107_obs_last81)?;
        Some((f107, f107a, worse(prev.qualifier, today.qualifier)))
    });
    if let Some((f107, f107a, q)) = flux {
        (out.f107, out.f107a, out.f107_source) = (f107, f107a, q.into());
    }
    if let Some((ap, q)) = today.and_then(|t| Some((t.ap_daily?, t.qualifier))) {
        (out.ap, out.ap_source) = (ap, q.into());
        out.ap_history = table.ap_3h_back(epoch).map(|b| ap_history(ap, &b));
    }
    out
}

/// 2 つの日の種類のうち、予測に近いほう（出所は弱いほうを名乗る）。
fn worse(a: FluxQualifier, b: FluxQualifier) -> FluxQualifier {
    let rank = |q: FluxQualifier| match q {
        FluxQualifier::Observed => 0,
        FluxQualifier::Interpolated => 1,
        FluxQualifier::DailyPredicted => 2,
        FluxQualifier::MonthlyPredicted => 3,
    };
    if rank(a) >= rank(b) { a } else { b }
}

/// 新しい順の 3 時間 ap 20 個 → NRLMSISE-00 の `ap_array`。
fn ap_history(daily: f64, back: &[f64; 20]) -> [f64; 7] {
    let mean = |s: &[f64]| s.iter().sum::<f64>() / s.len() as f64;
    [
        daily,
        back[0],
        back[1],
        back[2],
        back[3],
        mean(&back[4..12]),
        mean(&back[12..20]),
    ]
}

/// 日の番号 `day_number` を含む月の末日の番号。
fn last_day_of_month(day_number: i64) -> i64 {
    let (y, m, _) = Epoch::calendar_of_day_number(day_number);
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    Epoch::day_number_of(ny, nm, 1) - 1
}

#[cfg(test)]
mod tests;
