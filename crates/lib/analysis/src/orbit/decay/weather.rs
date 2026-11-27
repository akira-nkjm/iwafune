//! 高度低下の太陽活動の条件と、日付で引いた宇宙天気の列（表は持たない。呼び出し側が引いた値を受け取る）。
//!
//! 宇宙天気の表の解釈と日付からの引き方は `sat_astro::atmosphere::space_weather` だけが持つ。ここは
//! 引く日付の間隔（[`WEATHER_STEP_S`]）と、列のうちどの値を時刻に当てるか（直前のサンプルを保つ）だけを決める。

use crate::spec::AnalysisError;
use sat_astro::atmosphere::AtmosphereModel;
use sat_astro::atmosphere::space_weather::{DatedSpaceWeather, SolarActivity, WeatherSource};
use sat_astro::constants::SECONDS_PER_DAY;
use sat_astro::time::Epoch;
use sat_astro::timeline::sample_times;
use sat_units::typed::{Qty, Time};
use serde::{Deserialize, Serialize};

/// 宇宙天気の表から引く間隔 [s]（27 日 ≈ 太陽の自転周期）。
///
/// 寿命を決めるのは F10.7 の 81 日平均の変化（太陽周期）なので、日ごとの揺らぎは追わない。
/// サンプルの日の前日の F10.7 をその間隔のあいだ保つので、日ごとの値の揺らぎは平均されない（注意）。
pub const WEATHER_STEP_S: f64 = 27.0 * SECONDS_PER_DAY;

/// 高度低下を求める太陽活動の条件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DecayCondition {
    /// 太陽活動のプリセット（ECSS-E-ST-10-04C の長期の値）を全期間で一定にする
    Preset { activity: SolarActivity },
    /// 宇宙天気の表から日付で引いた値（表の外や欠けた値は `fallback` のプリセット）
    Dated { fallback: SolarActivity },
}

/// 日付で引いた宇宙天気の 1 サンプル。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DecayWeatherSample {
    /// 軌道の元期からの経過
    pub t: Qty<Time>,
    /// その日の宇宙天気と出所
    pub weather: DatedSpaceWeather,
}

/// 日付で引いた宇宙天気の列（[`WEATHER_STEP_S`] ごと。呼び出し側が表から引いて入力に入れる）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DecayWeatherSchedule {
    /// 表の外や欠けた値に使ったプリセット
    pub fallback: SolarActivity,
    /// 経過の順のサンプル（最初は元期）
    pub samples: Vec<DecayWeatherSample>,
}

/// 日付で引いた列の、値の出所ごとのサンプルの数と、予測値・プリセットを使い始めた日時。
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DecayWeatherOrigins {
    /// 実績値（観測・補間）
    pub measured: u32,
    /// 表の予測値（日ごと・月ごと）
    pub predicted: u32,
    /// プリセット（表の外か値が欠けている）
    pub preset: u32,
    /// 手で与えた値
    pub manual: u32,
    /// 最初に予測値を使った日時
    pub predicted_from: Option<Epoch>,
    /// 最初にプリセットを使った日時
    pub preset_from: Option<Epoch>,
}

/// 日付で引いた列のうち、どこから値を取ったか（F10.7 と Ap を別に数える。表の月ごとの予測は F10.7 だけを持つ）。
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DecayWeatherSummary {
    /// サンプルの数
    pub samples: u32,
    /// F10.7（前日の値と 81 日平均）の出所
    pub f107: DecayWeatherOrigins,
    /// Ap の出所
    pub ap: DecayWeatherOrigins,
}

/// 元期 `epoch` から `span_s` [s] までを [`WEATHER_STEP_S`] ごとに `lookup` で引いた列。
/// `lookup` は日付 → 宇宙天気（`sat_astro::atmosphere::space_weather::space_weather_at` に表とプリセットを渡したもの）。
pub fn dated_schedule(
    epoch: Epoch,
    span_s: f64,
    fallback: SolarActivity,
    lookup: impl Fn(Epoch) -> DatedSpaceWeather,
) -> Result<DecayWeatherSchedule, AnalysisError> {
    let samples = sample_times(span_s, WEATHER_STEP_S)?
        .into_iter()
        .map(|t| DecayWeatherSample {
            t: Qty::from_raw(t),
            weather: lookup(epoch.plus_seconds(t)),
        })
        .collect();
    Ok(DecayWeatherSchedule { fallback, samples })
}

/// 時刻ごとの大気モデルの選び方（一定か、列の直前のサンプルを保つか）。
#[derive(Debug, Clone, Copy)]
pub(super) enum Weather<'a> {
    Constant(AtmosphereModel),
    Dated(&'a [DecayWeatherSample]),
}

impl Weather<'_> {
    /// 経過 `t` [s] の大気モデル（日平均 Ap で固定した NRLMSISE-00）。列の前は最初、後ろは最後のサンプルを保つ。
    pub(super) fn atmosphere(&self, t: f64) -> AtmosphereModel {
        match self {
            Self::Constant(m) => *m,
            Self::Dated(samples) => {
                let i = samples.partition_point(|s| s.t.value() <= t);
                samples
                    .get(i.saturating_sub(1))
                    .map_or(AtmosphereModel::default(), |s| s.weather.atmosphere_model())
            }
        }
    }

    /// `t` [s] より後で値が変わる最初の経過（無ければ `None`）。積分の刻みをここで切る。
    pub(super) fn next_change(&self, t: f64) -> Option<f64> {
        match self {
            Self::Constant(_) => None,
            Self::Dated(samples) => {
                let i = samples.partition_point(|s| s.t.value() <= t);
                samples.get(i).map(|s| s.t.value())
            }
        }
    }
}

/// 列の出所の数と、予測値・プリセットを使い始めた日時。
pub(super) fn summarize(epoch: Epoch, samples: &[DecayWeatherSample]) -> DecayWeatherSummary {
    let mut s = DecayWeatherSummary::default();
    for sample in samples {
        let at = epoch.plus_seconds(sample.t.value());
        s.samples += 1;
        s.f107.count(sample.weather.f107_source, at);
        s.ap.count(sample.weather.ap_source, at);
    }
    s
}

impl DecayWeatherOrigins {
    fn count(&mut self, source: WeatherSource, at: Epoch) {
        match source {
            WeatherSource::Observed | WeatherSource::Interpolated => self.measured += 1,
            WeatherSource::DailyPredicted | WeatherSource::MonthlyPredicted => {
                self.predicted += 1;
                self.predicted_from.get_or_insert(at);
            }
            WeatherSource::Preset => {
                self.preset += 1;
                self.preset_from.get_or_insert(at);
            }
            WeatherSource::Manual => self.manual += 1,
        }
    }
}
