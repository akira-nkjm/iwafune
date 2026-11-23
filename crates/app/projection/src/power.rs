//! 電源の射影: `power.mode_load`・`power.solar_sunlit`・`power.orbit_generation`・`power.energy_balance`。

use crate::context::Context;
use crate::missing::MissingInputs;
use crate::parts::{self, by_role, direction_body, label, ratio, state_values};
use crate::quantity::qty_opt;
use crate::timeline;
use fune_model::Target;
use fune_model::builtin::Role;
use fune_model::design::AnalysisSystem;
use sat_analysis::power::energy_balance::{
    EnergyBalance, EnergyBalanceBattery, EnergyBalanceCase, EnergyBalanceLoad,
};
use sat_analysis::power::mode_load::ModeLoadPart;
use sat_analysis::power::solar::{SolarArray, SolarPanel};
use sat_analysis::power::solar_sunlit::{SolarSunlitCase, SolarSunlitSun};
use sat_analysis::power::{
    EnergyBalanceInput, ModeLoadInput, OrbitGenerationInput, SolarSunlitInput,
};
use sat_analysis::spec::{Analysis, SamplingSpan};
use sat_astro::timeline::TimelineSpec;
use sat_units::typed::{Area, Energy, Power, Time, Voltage};

mod sequence;
mod sizing;
mod thrust;
pub use sizing::sizing;
pub use thrust::thrust_balance;

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
                count: p.count_or_one(),
                redundancy: crate::parts::redundancy(p),
                states: p.states.clone(),
                power: state_values::<Power>(ctx, p, "power")?,
                growth_margin: parts::growth(ctx.design, p),
            })
        })
        .collect();
    if parts.is_empty() {
        m.push(
            Target::any_part("power"),
            "No part with a power consumption",
        );
    }
    if ctx.design.modes.is_empty() {
        m.push(Target::Modes, "Operating modes (modes/<id>.toml)");
    }
    ModeLoadInput {
        parts,
        modes: ctx.modes(),
        default_growth_margin: None,
    }
}

/// 太陽電池パネル（role = solar_panel）の並び。
pub fn solar_array(ctx: &Context<'_>, m: &mut MissingInputs) -> SolarArray {
    let panels: Vec<SolarPanel> = by_role(ctx, Role::SolarPanel)
        .filter_map(|p| {
            let why = "Solar panel values";
            let area = parts::extensive_qty::<Area>(ctx, p, "area", why, m);
            let eff = ratio(ctx, p, "cell_efficiency", why, m);
            let normal = direction_body(
                ctx,
                p,
                "normal",
                "Generating-surface normal (part frame)",
                m,
            );
            Some(SolarPanel {
                id: p.id.clone(),
                area: area?,
                cell_efficiency: eff?,
                packing_factor: ctx.scalar(p, "packing_factor"),
                eol_factor: ctx.scalar(p, "eol_factor"),
                normal: normal?,
            })
        })
        .collect();
    if panels.is_empty() {
        m.push(
            Target::any_of_roles(&[Role::SolarPanel], None),
            "No solar panel part (kind with role = solar_panel)",
        );
    }
    SolarArray {
        panels,
        solar_flux: None,
        conversion_efficiency: ctx.setting(
            power_key(&["conversion_efficiency"]),
            ctx.design
                .analysis
                .power
                .as_ref()
                .and_then(|p| p.conversion_efficiency),
        ),
    }
}

/// `analysis.power.<…>`（`analysis/power.toml` のキー）。
fn power_key(keys: &[&str]) -> fune_model::path::SettingPath {
    AnalysisSystem::Power.setting_path().child(keys)
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
            "Attitude law (per mode)",
        );
    }
    for mode in modes {
        if let Some(t) =
            timeline::timeline_over(ctx, Some(&mode), SamplingSpan::OrbitAverage, &mut m)
        {
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
            life: None,
        })
    })
}

/// `power.energy_balance`: モードごとに 1 ケース（ケースを選べばそのモードだけ）と、モードの並び
/// （`analysis.power.active_sequence` の並び。[`sequence::active`]）・基本のモード（`analysis.power.base_mode`）・モードの最長継続時間。
pub fn energy_balance(ctx: &Context<'_>) -> Result<EnergyBalanceInput, MissingInputs> {
    let mut m = MissingInputs::new();
    let array = solar_array(ctx, &mut m);
    let loads = mode_load_input(ctx, &mut m);
    let mode = timeline::case_mode_id(ctx);
    let t = attitude_timeline(ctx, EnergyBalance::ID, &mut m)?;
    let batteries = batteries(ctx, &mut m);
    let cases: Vec<EnergyBalanceCase> = ctx
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
            max_duration: max_duration(md, &mut m),
            timing: sequence::mode_timing(md, &mut m),
        })
        .collect();
    let (sequence, sequence_timing) = sequence::active(ctx, &mut m);
    let base_mode = base_mode(ctx);
    m.finish(|| {
        Some(EnergyBalanceInput {
            timeline: t?,
            array,
            life: None,
            batteries,
            loads,
            load_growth: None,
            cases,
            sequence,
            sequence_timing,
            base_mode,
            mode_attitudes: mode_law_map(ctx),
        })
    })
}

/// モードの最長継続時間（`max_duration`）。次元が違えば不足入力。
fn max_duration(
    md: &fune_model::design::Mode,
    m: &mut MissingInputs,
) -> Option<sat_units::typed::Qty<Time>> {
    qty_opt::<Time>(
        md.max_duration.as_ref(),
        Target::mode(md.id.as_str(), None),
        m,
    )
}

/// 最長継続時間の評価で下に敷く基本のモード（`analysis.power.base_mode`）。
fn base_mode(ctx: &Context<'_>) -> Option<String> {
    ctx.setting(
        power_key(&["base_mode"]),
        ctx.design
            .analysis
            .power
            .as_ref()
            .and_then(|p| p.base_mode.clone()),
    )
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
    let attitude_ctx =
        Context::new(ctx.design, ctx.resolved, attitude_case)?.with_reads(ctx.reads());
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

/// 電源の解析の周回数（ケース、なければ設定。どちらも無ければ `None` で、既定は解析）。
fn power_orbits(ctx: &Context<'_>) -> Option<u32> {
    if let Some((case, orbits)) = ctx.case.and_then(|c| Some((c, c.orbits?))) {
        ctx.note(Target::case(case.id.as_str(), Some("orbits")));
        return Some(orbits);
    }
    ctx.setting(
        power_key(&["orbits"]),
        ctx.design.analysis.power.as_ref().and_then(|p| p.orbits),
    )
}

/// 電池を個別に渡す。容量の集約規則は解析が持つ。
fn batteries(ctx: &Context<'_>, m: &mut MissingInputs) -> Vec<EnergyBalanceBattery> {
    let cells: Vec<_> = by_role(ctx, Role::Battery).collect();
    if cells.is_empty() {
        m.push(
            Target::any_of_roles(&[Role::Battery], None),
            "No battery part (kind with role = battery)",
        );
        return Vec::new();
    }
    let why = "Battery values";
    let settings = ctx.design.analysis.power.as_ref();
    // 許容する DoD は要求が持つ（`power.energy_balance` の `dod`。`power.sizing` も同じ基準を読む）。無ければ入力不足
    let dod = m.need(
        ctx.criterion("power.energy_balance", "dod").map(|c| c.si),
        Target::criterion("power.energy_balance/dod"),
        "Allowed DoD (an at_most requirement verified by power.energy_balance / dod)",
    );
    let initial_soc = ctx.setting(
        power_key(&["initial_soc"]),
        settings.and_then(|s| s.initial_soc),
    );

    // 効率と初期 SOC は書かれていなければ None（既定値は解析が持つ）。
    cells
        .into_iter()
        .filter_map(|cell| {
            Some(EnergyBalanceBattery {
                capacity: parts::extensive_qty::<Energy>(ctx, cell, "capacity", why, m)?,
                voltage: parts::part_qty::<Voltage>(ctx, cell, "voltage", why, m)?,
                charge_efficiency: ctx.scalar(cell, "charge_efficiency"),
                discharge_efficiency: ctx.scalar(cell, "discharge_efficiency"),
                initial_soc,
                dod_limit: dod?,
                max_charge_power: None,
            })
        })
        .collect()
}

/// モードの姿勢則（`analysis.aocs.law.<モード>`）。書かれていなければ None（時系列の姿勢則を使う）。
fn mode_law(ctx: &Context<'_>, mode: &str) -> Option<sat_astro::attitude::AttitudeLaw> {
    let law = *ctx.laws().get(mode)?;
    ctx.note(ctx.law_target(mode));
    Some(crate::context::law_of(law))
}
