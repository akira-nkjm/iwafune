//! 電源の射影: `power.mode_load`・`power.solar_sunlit`・`power.orbit_generation`・`power.energy_balance`。

use crate::context::Context;
use crate::missing::MissingInputs;
use crate::parts::{self, by_role, direction_body, label, ratio, state_values};
use crate::timeline;
use fune_model::Target;
use fune_model::builtin::Role;
use fune_model::design::AnalysisSystem;
use sat_analysis::power::energy_balance::{
    EnergyBalance, EnergyBalanceBattery, EnergyBalanceCase, EnergyBalanceLoad, LoadGrowth,
};
use sat_analysis::power::mode_load::ModeLoadPart;
use sat_analysis::power::solar::{SolarArray, SolarLife, SolarPanel};
use sat_analysis::power::solar_sunlit::{SolarSunlitCase, SolarSunlitSun};
use sat_analysis::power::{
    EnergyBalanceInput, ModeLoadInput, OrbitGenerationInput, SolarSunlitInput,
};
use sat_analysis::spec::Analysis;
use sat_astro::timeline::TimelineSpec;
use sat_units::typed::{Area, Energy, Power, Voltage};

mod sizing;
pub use sizing::sizing;

/// `power.mode_load`
pub fn mode_load(ctx: &Context<'_>) -> Result<ModeLoadInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let input = mode_load_input(ctx, &mut m);
    m.finish(|| Some(input))
}

/// 部品の電力とモードから `ModeLoadInput` を作る（収支でも使う）。
pub fn mode_load_input(ctx: &Context<'_>, m: &mut MissingInputs) -> ModeLoadInput {
    let parts: Vec<ModeLoadPart> = parts::with_attr(ctx, "power")
        .filter_map(|p| {
            Some(ModeLoadPart {
                id: p.id.clone(),
                label: label(p),
                group: p.system.map(|s| s.to_string()).unwrap_or_default(),
                count: parts::count_of(p),
                redundancy: crate::parts::redundancy(p),
                states: p.states.clone(),
                power: state_values::<Power>(p, "power")?,
                growth_margin: parts::growth(ctx.design, p),
            })
        })
        .collect();
    if parts.is_empty() {
        m.push(Target::any_part("power"), "消費電力を持つ部品がありません");
    }
    if ctx.design.modes.is_empty() {
        m.push(Target::Modes, "運用モード（modes/<ID>.toml）");
    }
    ModeLoadInput {
        parts,
        modes: ctx.modes(),
        default_growth_margin: 0.0,
    }
}

/// 太陽電池パネル（role = solar_panel）の並び。
pub fn solar_array(ctx: &Context<'_>, m: &mut MissingInputs) -> SolarArray {
    let panels: Vec<SolarPanel> = by_role(ctx, Role::SolarPanel)
        .filter_map(|p| {
            let why = "太陽電池パネルの値";
            let area = parts::extensive_qty::<Area>(p, "area", why, m);
            let eff = ratio(p, "cell_efficiency", why, m);
            let normal = direction_body(ctx, p, "normal", "発電面の法線（部品座標）", m);
            Some(SolarPanel {
                id: p.id.clone(),
                area: area?,
                cell_efficiency: eff?,
                packing_factor: p.scalar("packing_factor"),
                eol_factor: p.scalar("eol_factor"),
                normal: normal?,
            })
        })
        .collect();
    if panels.is_empty() {
        m.push(
            Target::any_of_roles(&[Role::SolarPanel], None),
            "太陽電池パネル（kind の role = solar_panel）の部品がありません",
        );
    }
    SolarArray {
        panels,
        solar_flux: None,
        conversion_efficiency: ctx
            .design
            .analysis
            .power
            .as_ref()
            .and_then(|p| p.conversion_efficiency),
    }
}

/// `power.solar_sunlit`: 姿勢則を持つモードごとに 1 ケース（軌道の時系列で日照中を評価）。
pub fn solar_sunlit(ctx: &Context<'_>) -> Result<SolarSunlitInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let array = solar_array(ctx, &mut m);
    let mut cases = Vec::new();
    let modes = timeline::modes_with_law(ctx);
    if modes.is_empty() {
        m.push(
            AnalysisSystem::Aocs.setting_path().child(&["law"]),
            "姿勢則（モードごと）",
        );
    }
    for mode in modes {
        if let Some(t) = timeline::timeline(ctx, Some(&mode), 1, &mut m) {
            cases.push(SolarSunlitCase {
                id: mode.clone(),
                label: mode.clone(),
                sun: SolarSunlitSun::Timeline {
                    timeline: Box::new(t),
                },
            });
        }
    }
    m.finish(|| Some(SolarSunlitInput { array, cases }))
}

/// `power.orbit_generation`
pub fn orbit_generation(ctx: &Context<'_>) -> Result<OrbitGenerationInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let array = solar_array(ctx, &mut m);
    let mode = timeline::case_mode_id(ctx);
    let t = timeline::timeline(ctx, mode.as_deref(), power_orbits(ctx), &mut m);
    m.finish(|| {
        Some(OrbitGenerationInput {
            timeline: t?,
            array,
            life: SolarLife::Eol,
        })
    })
}

/// `power.energy_balance`: モードごとに 1 ケース（ケースを選べばそのモードだけ）。
pub fn energy_balance(ctx: &Context<'_>) -> Result<EnergyBalanceInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let array = solar_array(ctx, &mut m);
    let loads = mode_load_input(ctx, &mut m);
    let mode = timeline::case_mode_id(ctx);
    let t = attitude_timeline(ctx, EnergyBalance::ID, &mut m)?;
    let batteries = batteries(ctx, &mut m);
    let cases: Vec<EnergyBalanceCase> = ctx
        .design
        .ordered_modes()
        .into_iter()
        .filter(|md| mode.as_ref().is_none_or(|id| md.id.as_str() == id))
        .map(|md| EnergyBalanceCase {
            id: md.id.to_string(),
            label: md.label.clone().unwrap_or_else(|| md.id.to_string()),
            load: EnergyBalanceLoad::Mode {
                mode: md.id.to_string(),
            },
            attitude: mode_law(ctx, md.id.as_str()),
        })
        .collect();
    m.finish(|| {
        Some(EnergyBalanceInput {
            timeline: t?,
            array,
            life: SolarLife::Eol,
            batteries,
            loads,
            load_growth: LoadGrowth::Included,
            cases,
        })
    })
}

/// 電力収支と逆算の上段の時系列。既定ケースから借りるのは姿勢則のモードだけで、β・周回数などは元の ctx を使う。
fn attitude_timeline(
    ctx: &Context<'_>,
    analysis: &str,
    m: &mut MissingInputs,
) -> Result<Option<TimelineSpec>, MissingInputs> {
    let (attitude_case, _) = crate::case_policy::effective_case(
        Some(ctx.design),
        analysis,
        ctx.case.map(|c| c.id.as_str()),
    );
    let attitude_ctx = Context::new(ctx.design, ctx.resolved, attitude_case)?;
    let attitude_mode = timeline::case_mode_id(&attitude_ctx);
    Ok(timeline::timeline(
        ctx,
        attitude_mode.as_deref(),
        power_orbits(ctx),
        m,
    ))
}

/// モードごとの姿勢則（書かれているモードだけ）。
fn mode_law_map(
    ctx: &Context<'_>,
) -> std::collections::BTreeMap<String, sat_astro::attitude::AttitudeLaw> {
    ctx.design
        .modes
        .values()
        .filter_map(|md| Some((md.id.to_string(), mode_law(ctx, md.id.as_str())?)))
        .collect()
}

fn power_orbits(ctx: &Context<'_>) -> u32 {
    ctx.case
        .and_then(|c| c.orbits)
        .or_else(|| ctx.design.analysis.power.as_ref().and_then(|p| p.orbits))
        .unwrap_or(timeline::DEFAULT_ORBITS)
}

/// 電池を個別に渡す。容量の集約規則は解析が持つ。
fn batteries(ctx: &Context<'_>, m: &mut MissingInputs) -> Vec<EnergyBalanceBattery> {
    let cells: Vec<_> = by_role(ctx, Role::Battery).collect();
    if cells.is_empty() {
        m.push(
            Target::any_of_roles(&[Role::Battery], None),
            "電池（kind の role = battery）の部品がありません",
        );
        return Vec::new();
    }
    let why = "電池の値";
    let settings = ctx.design.analysis.power.as_ref();
    let dod = m.need(
        settings.and_then(|s| s.dod_max),
        AnalysisSystem::Power.setting_path().child(&["dod_max"]),
        "許容 DoD",
    );

    // 効率と初期 SOC は書かれていなければ None（既定値は解析が持つ）。
    cells
        .into_iter()
        .filter_map(|cell| {
            Some(EnergyBalanceBattery {
                capacity: parts::extensive_qty::<Energy>(cell, "capacity", why, m)?,
                voltage: parts::part_qty::<Voltage>(cell, "voltage", why, m)?,
                charge_efficiency: cell.scalar("charge_efficiency"),
                discharge_efficiency: cell.scalar("discharge_efficiency"),
                initial_soc: settings.and_then(|s| s.initial_soc),
                dod_limit: dod?,
                max_charge_power: None,
            })
        })
        .collect()
}

/// モードの姿勢則（`analysis.aocs.law.<モード>`）。書かれていなければ None（時系列の姿勢則を使う）。
fn mode_law(ctx: &Context<'_>, mode: &str) -> Option<sat_astro::attitude::AttitudeLaw> {
    let laws = &ctx.design.analysis.aocs.as_ref()?.law;
    laws.get(mode).map(crate::context::law_of)
}
