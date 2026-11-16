//! 電源サイジングへの写し替え。最長蝕の計算は解析に渡す。SAP と姿勢則は電力収支と同じものを渡す。
use super::{
    attitude_timeline, base_mode, batteries, max_duration, mode_law_map, mode_load_input,
    power_key, sequence, solar_array,
};
use crate::parts::by_role;
use crate::quantity::setting_opt;
use crate::{Context, MissingInputs};
use fune_model::Target;
use fune_model::builtin::Role;
use fune_model::design::setting_dims as dims;
use sat_analysis::power::sizing::{Sizing, SizingCell, SizingInput, SizingOptions};
use sat_analysis::spec::Analysis;
use sat_units::typed::Qty;

/// 軌道・期間・時系列と姿勢則・モード・パネルと電池・電源設定。時系列の逆算に使う最長継続時間・
/// モードの並び・基本のモードは `power.energy_balance` と同じものを渡す。
pub fn sizing(ctx: &Context<'_>) -> Result<SizingInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let season = match crate::orbit::eclipse_season(ctx) {
        Ok(input) => Some(input),
        Err(missing) => {
            m.extend(missing);
            None
        }
    };
    let timeline = attitude_timeline(ctx, Sizing::ID, &mut m)?;
    let loads = mode_load_input(ctx, &mut m);
    let array = solar_array(ctx, &mut m);
    let batteries = batteries(ctx, &mut m);
    let options = options(ctx, &mut m);
    // 太陽距離の慣例値（地球・月は 1 AU）が無い中心天体では太陽距離を書く（発電の強さが決まらない）
    if let Some(central) = season.as_ref().map(|s| s.orbit.central)
        && options.sun_distance.is_none()
        && central.nominal_sun_distance_m().is_none()
    {
        m.push(
            dims::POWER_SIZING_SUN_DISTANCE.at(&[]),
            format!("Sun distance of central body {central:?}"),
        );
    }
    let mode = crate::timeline::case_mode_id(ctx);
    let max_durations = ctx
        .ordered_modes()
        .into_iter()
        .filter(|md| mode.as_ref().is_none_or(|id| md.id.as_str() == id))
        .filter_map(|md| Some((md.id.to_string(), max_duration(md, &mut m)?)))
        .collect();
    let sequence = sequence(ctx, &mut m);
    if let Some(case) = ctx.case
        && case.mode.is_some()
        && mode.is_none()
    {
        m.push(
            Target::case(case.id.as_str(), Some("mode")),
            "The specified mode does not exist",
        );
    }
    // 設計寿命はミッション期間（`conditions.orbit.duration`）。季節の解析の期間も同じ設定から作るので同じ値
    let design_life = season.as_ref().map(|s| s.duration);
    m.finish(|| {
        Some(SizingInput {
            season: season?,
            design_life,
            timeline: timeline?,
            mode_laws: mode_law_map(ctx),
            loads,
            mode,
            cells: cells(ctx),
            array,
            batteries,
            options,
            max_durations,
            sequence,
            base_mode: base_mode(ctx),
        })
    })
}

/// セルの電圧・電流を両方持つパネル（直並列数を出すときだけ使う）。
fn cells(ctx: &Context<'_>) -> Vec<SizingCell> {
    by_role(ctx, Role::SolarPanel)
        .filter_map(|p| {
            Some(SizingCell {
                panel: p.id.clone(),
                voltage: Qty::from_raw(ctx.scalar(p, "cell_voltage")?),
                current: Qty::from_raw(ctx.scalar(p, "cell_current")?),
            })
        })
        .collect()
}

fn options(ctx: &Context<'_>, m: &mut MissingInputs) -> SizingOptions {
    let power = ctx.design.analysis.power.as_ref();
    let s = power.and_then(|p| p.sizing.as_ref());
    // 書かれていなければ解析の既定値を使う欄も、読んだものとして記録する
    let key = |k: &str| ctx.note(power_key(&["sizing", k]));
    for k in [
        "sunlit_efficiency",
        "eclipse_efficiency",
        "battery_efficiency",
        "annual_degradation",
        "inherent_degradation",
        "battery_strings",
        "bus_voltage",
        "sun_distance",
    ] {
        key(k);
    }
    let sunlit = s.and_then(|s| s.sunlit_efficiency);
    if sunlit.is_none() {
        ctx.note(power_key(&["conversion_efficiency"]));
    }
    SizingOptions {
        sunlit_efficiency: sunlit.or_else(|| power.and_then(|p| p.conversion_efficiency)),
        eclipse_efficiency: s.and_then(|s| s.eclipse_efficiency),
        battery_efficiency: s.and_then(|s| s.battery_efficiency),
        annual_degradation: s.and_then(|s| s.annual_degradation),
        inherent_degradation: s.and_then(|s| s.inherent_degradation),
        battery_strings: s.and_then(|s| s.battery_strings),
        bus_voltage: setting_opt(
            &dims::POWER_SIZING_BUS_VOLTAGE,
            &[],
            s.and_then(|s| s.bus_voltage.as_ref()),
            m,
        ),
        sun_distance: setting_opt(
            &dims::POWER_SIZING_SUN_DISTANCE,
            &[],
            s.and_then(|s| s.sun_distance.as_ref()),
            m,
        ),
    }
}
