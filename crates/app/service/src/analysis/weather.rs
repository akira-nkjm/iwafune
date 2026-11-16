//! 高度低下の解析（`orbit.decay`）の入力に、宇宙天気の表から日付で引いた列を足し、使った表の版を返す。
//!
//! 表の読み込みは `data::space_weather::loaded`、日付からの引き方は `sat_astro::atmosphere::space_weather::space_weather_at`、
//! 引く日付の間隔は解析（`sat_analysis::orbit::decay::dated_schedule`）が決める。表が無い・読めないときは列を足さず
//! （解析はプリセットの条件だけを返す）、版も残さない。表の外の日付は既定のプリセット（中程度）になり、
//! どこからプリセットになったかは解析の出力（`weather_sources`）が返す。

use crate::data::space_weather::loaded;
use crate::dto::SpaceWeatherVersion;
use sat_analysis::orbit::OrbitDecay;
use sat_analysis::orbit::decay::{OrbitDecayInput, dated_schedule};
use sat_analysis::spec::Analysis;
use sat_astro::atmosphere::space_weather::{SolarActivity, space_weather_at};
use serde_json::Value;

/// 設計から組み立てた入力 `input` に、表から引いた列を足す（`orbit.decay` だけ。ほかの解析はそのまま）。
/// 列を足したら、使った表の版を返す。
pub(super) fn attach(id: &str, input: Value) -> (Value, Option<SpaceWeatherVersion>) {
    if id != OrbitDecay::ID {
        return (input, None);
    }
    let Ok(Some(table)) = loaded() else {
        return (input, None);
    };
    let Ok(mut typed) = serde_json::from_value::<OrbitDecayInput>(input.clone()) else {
        return (input, None);
    };
    let fallback = SolarActivity::default();
    let schedule = dated_schedule(typed.orbit.epoch, typed.span_s(), fallback, |epoch| {
        space_weather_at(Some(&table.table), epoch, fallback)
    });
    let Ok(schedule) = schedule else {
        return (input, None);
    };
    typed.dated_weather = Some(schedule);
    match serde_json::to_value(&typed) {
        Ok(v) => (v, Some(table.version.clone())),
        Err(_) => (input, None),
    }
}
