//! 電源サイジングへの写し替え。最長蝕の計算は解析に渡す。SAP と姿勢則は電力収支と同じものを渡す。
use super::{attitude_timeline, batteries, mode_law_map, mode_load_input, solar_array};
use crate::parts::by_role;
use crate::quantity::setting_opt;
use crate::{Context, MissingInputs};
use fune_model::Target;
use fune_model::builtin::Role;
use fune_model::design::setting_dims as dims;
use sat_analysis::power::sizing::{Sizing, SizingCell, SizingInput, SizingOptions};
use sat_analysis::spec::Analysis;
use sat_units::typed::Qty;

/// 軌道・期間・時系列と姿勢則・モード・パネルと電池・電源設定。
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
    let mode = crate::timeline::case_mode_id(ctx);
    if let Some(case) = ctx.case
        && case.mode.is_some()
        && mode.is_none()
    {
        m.push(
            Target::case(case.id.as_str(), Some("mode")),
            "指定モードがありません",
        );
    }
    m.finish(|| {
        Some(SizingInput {
            season: season?,
            timeline: timeline?,
            mode_laws: mode_law_map(ctx),
            loads,
            mode,
            cells: cells(ctx),
            array,
            batteries,
            options,
        })
    })
}

/// セルの電圧・電流を両方持つパネル（直並列数を出すときだけ使う）。
fn cells(ctx: &Context<'_>) -> Vec<SizingCell> {
    by_role(ctx, Role::SolarPanel)
        .filter_map(|p| {
            Some(SizingCell {
                panel: p.id.clone(),
                voltage: Qty::from_raw(p.scalar("cell_voltage")?),
                current: Qty::from_raw(p.scalar("cell_current")?),
            })
        })
        .collect()
}

fn options(ctx: &Context<'_>, m: &mut MissingInputs) -> SizingOptions {
    let power = ctx.design.analysis.power.as_ref();
    let s = power.and_then(|p| p.sizing.as_ref());
    SizingOptions {
        sunlit_efficiency: s
            .and_then(|s| s.sunlit_efficiency)
            .or_else(|| power.and_then(|p| p.conversion_efficiency)),
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
