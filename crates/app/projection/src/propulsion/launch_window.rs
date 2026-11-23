//! `propulsion.launch_window` の射影: 推進系の入力（`propulsion::maneuvers` と同じ。マヌーバの日付は `maneuver_dates` が読んだもの）に、
//! `analysis/propulsion.toml` の `[launch_window]` を写すだけ。既定値（窓・飛行時間・点の数）は解析が天体の組から決める。

use super::maneuvers;
use crate::context::Context;
use crate::missing::MissingInputs;
use crate::quantity::setting_opt;
use fune_model::design::PropulsionLaunchWindow as Written;
use fune_model::design::setting_dims as dims;
use fune_model::path::SettingPath;
use sat_analysis::propulsion::{LaunchWindowInput, LaunchWindowSettings};
use sat_astro::time::Epoch;

/// `propulsion.launch_window`。
pub fn launch_window(ctx: &Context<'_>) -> Result<LaunchWindowInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let propulsion = maneuvers(ctx).map_err(|e| m.extend(e)).ok();
    let written = ctx
        .design
        .analysis
        .propulsion
        .as_ref()
        .and_then(|s| s.launch_window.as_ref());
    let settings = settings(written, &mut m);
    m.finish(|| {
        Some(LaunchWindowInput {
            propulsion: propulsion?,
            settings,
        })
    })
}

/// 書かれた設定 → 解析の設定（読めない日付は不足入力に記録して `None` にする）。
fn settings(w: Option<&Written>, m: &mut MissingInputs) -> LaunchWindowSettings {
    let Some(w) = w else {
        return LaunchWindowSettings::default();
    };
    let departure_start = w.departure_start.as_deref().and_then(|text| {
        Epoch::parse(text.trim())
            .map_err(|e| {
                m.push(
                    SettingPath::analysis(&["propulsion", "launch_window", "departure_start"]),
                    format!("Cannot parse the launch window start date ({e})"),
                );
            })
            .ok()
    });
    LaunchWindowSettings {
        departure_start,
        departure_span: setting_opt(
            &dims::PROPULSION_LAUNCH_WINDOW_DEPARTURE_SPAN,
            &[],
            w.departure_span.as_ref(),
            m,
        ),
        time_of_flight_min: setting_opt(
            &dims::PROPULSION_LAUNCH_WINDOW_TOF_MIN,
            &[],
            w.time_of_flight_min.as_ref(),
            m,
        ),
        time_of_flight_max: setting_opt(
            &dims::PROPULSION_LAUNCH_WINDOW_TOF_MAX,
            &[],
            w.time_of_flight_max.as_ref(),
            m,
        ),
        departure_steps: w.departure_steps,
        arrival_steps: w.arrival_steps,
    }
}
