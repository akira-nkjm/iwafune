//! 軌道の高度低下（`orbit.decay`）・ミッション幾何（`orbit.geometry`）と宇宙天気の列挙の表示名。

use super::label;
use crate::dto::AnalysisLabel;
use sat_analysis::orbit::GeometrySpanBasis;
use sat_analysis::orbit::decay::{DecayCondition, DragAreaSource};
use sat_astro::atmosphere::space_weather::{SolarActivity, WeatherSource};

pub(super) fn solar_activity(a: SolarActivity) -> AnalysisLabel {
    match a {
        SolarActivity::Low => label("低い太陽活動", "Low solar activity"),
        SolarActivity::Moderate => label("中程度の太陽活動", "Moderate solar activity"),
        SolarActivity::High => label("高い太陽活動", "High solar activity"),
    }
}

pub(super) fn geometry_span_basis(b: GeometrySpanBasis) -> AnalysisLabel {
    match b {
        GeometrySpanBasis::OneOff => label("軌道の 1 回限りの期間", "The orbit's one-off span"),
        GeometrySpanBasis::PhaseDuration => label("フェーズの期間", "The phase duration"),
        GeometrySpanBasis::OneCycle => label("軌道の 1 周", "One orbit"),
    }
}

pub(super) fn weather_source(w: WeatherSource) -> AnalysisLabel {
    match w {
        WeatherSource::Observed => label("観測値", "Observed"),
        WeatherSource::Interpolated => label("補間値", "Interpolated"),
        WeatherSource::DailyPredicted => label("日ごとの予測", "Daily prediction"),
        WeatherSource::MonthlyPredicted => label("月ごとの予測", "Monthly prediction"),
        WeatherSource::Preset => label("太陽活動のプリセット", "Solar-activity preset"),
        WeatherSource::Manual => label("手で与えた値", "Manual value"),
    }
}

pub(super) fn drag_area_source(s: DragAreaSource) -> AnalysisLabel {
    match s {
        DragAreaSource::Given => label("与えた面積", "Given area"),
        DragAreaSource::BodyBoxes => label(
            "部品の箱の和集合の、全方向の平均の投影面積",
            "Mean projected area over all directions of the union of the part boxes",
        ),
    }
}

pub(super) fn decay_condition(kind: &str) -> Option<AnalysisLabel> {
    // 内部タグの列挙はタグの値だけを見る（中身の太陽活動は別の欄の値として引く）
    let probe = match kind {
        "preset" => DecayCondition::Preset {
            activity: SolarActivity::Moderate,
        },
        "dated" => DecayCondition::Dated {
            fallback: SolarActivity::Moderate,
        },
        _ => return None,
    };
    Some(match probe {
        DecayCondition::Preset { .. } => label(
            "太陽活動のプリセットで一定",
            "Constant solar-activity preset",
        ),
        DecayCondition::Dated { .. } => label(
            "宇宙天気の表から日付で引いた値",
            "Dated values from the space-weather table",
        ),
    })
}
