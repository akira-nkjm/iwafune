//! 1 ケースの評価（負荷の決定・電池の積分・周回ごとの集計・判定）。

use super::{
    Context, EnergyBalanceCase, EnergyBalanceCaseResult, EnergyBalanceInput, EnergyBalanceLoad,
    EnergyBalanceOrbit, LoadGrowth,
};
use crate::modes::find_mode;
use crate::power::battery::{self, BatteryTrace};
use crate::power::mode_load::evaluate_mode;
use crate::spec::quantity_text;
use crate::spec::{
    AnalysisError, Channel, ChannelKind, Check, Series, SpanFallback, Verdict, orbit_mean,
    orbit_segment, span_fallback,
};
use sat_units::typed::{Energy, Fraction, Power, Qty, Time};

/// DoD の上限との比較の許容（SOC の積分の丸めで上限ちょうどが超過にならないように）。
const DOD_TOLERANCE: f64 = 1e-12;

/// ケースの負荷（日照中・日陰中）を求める。モードの警告は `checks` に足す。
pub(super) fn case_loads(
    input: &EnergyBalanceInput,
    case: &EnergyBalanceCase,
    checks: &mut Vec<Check>,
) -> Result<(Qty<Power>, Qty<Power>), AnalysisError> {
    let mut mode_power = |id: &str, eclipse: bool| -> Result<Qty<Power>, AnalysisError> {
        let r = evaluate_mode(&input.loads, find_mode(&input.loads.modes, id)?)?;
        for c in r.checks {
            if !checks
                .iter()
                .any(|x| x.key == c.key && x.message == c.message)
            {
                checks.push(c);
            }
        }
        Ok(match input.load_growth {
            LoadGrowth::Included => {
                if eclipse {
                    r.total_eclipse_with_growth
                } else {
                    r.total_with_growth
                }
            }
            LoadGrowth::Excluded => {
                if eclipse {
                    r.total_eclipse
                } else {
                    r.total
                }
            }
        })
    };
    match &case.load {
        EnergyBalanceLoad::Mode { mode } => Ok((mode_power(mode, false)?, mode_power(mode, true)?)),
        EnergyBalanceLoad::SunlitEclipse {
            sunlit_mode,
            eclipse_mode,
        } => Ok((
            mode_power(sunlit_mode, false)?,
            mode_power(eclipse_mode, true)?,
        )),
        EnergyBalanceLoad::Constant { power } => Ok((*power, *power)),
    }
}

pub(super) fn evaluate_case(
    ctx: &Context,
    case: &EnergyBalanceCase,
) -> Result<EnergyBalanceCaseResult, AnalysisError> {
    let mut checks = Vec::new();
    let (sun, ecl) = case_loads(ctx.input, case, &mut checks)?;
    let load: Vec<f64> = ctx
        .profile
        .lit
        .iter()
        .map(|&f| if f { sun.value() } else { ecl.value() })
        .collect();
    let net: Vec<f64> = ctx
        .generation
        .iter()
        .zip(&load)
        .map(|(g, l)| g - l)
        .collect();
    let trace = battery::simulate(ctx.battery, &ctx.profile.t, &net);
    let capacity = ctx.battery.capacity.value();
    let soc: Vec<f64> = trace.energy.iter().map(|e| e / capacity).collect();
    let orbits = orbit_summaries(ctx, &load, &trace, &soc);
    let min_soc = soc.iter().copied().fold(f64::INFINITY, f64::min).min(1.0);
    let margin = orbits
        .iter()
        .map(|o| o.margin.value())
        .reduce(f64::min)
        .map(Qty::from_raw);
    let margin_over_span = span_fallback(ctx.ranges.len(), SpanFallback::OrbitMeanOverSpan)
        .map(|_| Qty::from_raw(trace.delta_unclamped.iter().sum()));
    let peak = |v: &[f64]| v.iter().copied().fold(0.0, f64::max);
    let peak_discharge = peak(&trace.discharge);
    let orbit_load = orbit_mean(&ctx.profile.t, &load, &ctx.ranges);
    let sunlit_mean = |v: &[f64]| {
        Qty::from_raw(
            crate::spec::masked_time_mean(&ctx.profile.t, v, &ctx.profile.lit).unwrap_or(0.0),
        )
    };
    let mut result = EnergyBalanceCaseResult {
        id: case.id.clone(),
        label: case.label.clone(),
        load_sunlit: sun,
        load_eclipse: ecl,
        orbit_average_generation: Qty::from_raw(orbit_mean(
            &ctx.profile.t,
            &ctx.generation,
            &ctx.ranges,
        )),
        sunlit_average_generation: sunlit_mean(&ctx.generation),
        sunlit_average_charge: sunlit_mean(&trace.charge),
        sunlit_average_discharge: sunlit_mean(&trace.discharge),
        sunlit_average_shunt: sunlit_mean(&trace.shunt),
        orbit_average_load: Qty::from_raw(orbit_load),
        orbit_average_balance: Qty::from_raw(
            orbit_mean(&ctx.profile.t, &ctx.generation, &ctx.ranges) - orbit_load,
        ),
        energy_margin_per_orbit: margin,
        energy_margin_over_span: margin_over_span,
        min_soc: Fraction::new(min_soc),
        max_dod: Fraction::new(1.0 - min_soc),
        max_eclipse_dod: Fraction::new(max_eclipse_drop(&ctx.profile.lit, &soc)),
        final_soc: Fraction::new(soc.last().copied().unwrap_or(ctx.battery.start_soc())),
        peak_charge_power: Qty::from_raw(peak(&trace.charge)),
        peak_discharge_power: Qty::from_raw(peak_discharge),
        peak_discharge_current: Qty::<Power>::from_raw(peak_discharge) / ctx.battery.voltage,
        orbits,
        series: case_series(ctx, load, soc, &trace),
        checks: Vec::new(),
    };
    checks.extend(verdicts(ctx, &result, trace.depleted));
    result.checks = checks;
    Ok(result)
}

fn orbit_summaries(
    ctx: &Context,
    load: &[f64],
    trace: &BatteryTrace,
    soc: &[f64],
) -> Vec<EnergyBalanceOrbit> {
    ctx.ranges
        .iter()
        .enumerate()
        .map(|(index, r)| {
            let to_energy = |v: &[f64]| {
                Qty::<Power>::from_raw(orbit_mean(&ctx.profile.t, v, std::slice::from_ref(r)))
                    * Qty::<Time>::from_raw(ctx.period_s)
            };
            let seg = orbit_segment(r, soc.len());
            let (start, end) = (*seg.start(), *seg.end());
            // 標本の区間の長さを周期ちょうどに揃える
            let covered = ctx.profile.t[end] - ctx.profile.t[start];
            let scale = if covered > 0.0 {
                ctx.period_s / covered
            } else {
                0.0
            };
            let margin: f64 = trace.delta_unclamped[start..end].iter().sum::<f64>() * scale;
            EnergyBalanceOrbit {
                index,
                start: Qty::from_raw(ctx.profile.t[start]),
                generation: to_energy(&ctx.generation),
                load: to_energy(load),
                margin: Qty::from_raw(margin),
                min_soc: Fraction::new(soc[seg].iter().copied().fold(f64::INFINITY, f64::min)),
                end_soc: Fraction::new(soc[end]),
            }
        })
        .collect()
}

/// 1 回の日陰で減る SOC の最大。日陰に入る直前（最初のサンプルが日陰ならそのサンプル）の SOC から測る。
pub(super) fn max_eclipse_drop(sunlit: &[bool], soc: &[f64]) -> f64 {
    let mut worst = 0.0_f64;
    let mut entry: Option<f64> = None;
    let mut low = f64::INFINITY;
    for (k, (&lit, &s)) in sunlit.iter().zip(soc).enumerate() {
        if lit {
            if let Some(e) = entry.take() {
                worst = worst.max(e - low.min(s));
            }
            continue;
        }
        if entry.is_none() {
            entry = Some(if k == 0 { s } else { soc[k - 1] });
            low = f64::INFINITY;
        }
        low = low.min(s);
    }
    if let Some(e) = entry {
        worst = worst.max(e - low);
    }
    worst
}

fn case_series(ctx: &Context, load: Vec<f64>, soc: Vec<f64>, trace: &BatteryTrace) -> Series {
    let mut series = Series {
        t: ctx.profile.t.clone(),
        channels: vec![
            Channel::of::<Power>(
                "generation",
                ChannelKind::Generation,
                ctx.generation.clone(),
            ),
            Channel::of::<Power>("load", ChannelKind::Load, load),
            Channel::of::<Fraction>("soc", ChannelKind::Soc, soc),
            Channel::of::<Power>(
                "charge_power",
                ChannelKind::ChargePower,
                trace.charge.clone(),
            ),
            Channel::of::<Power>(
                "discharge_power",
                ChannelKind::DischargePower,
                trace.discharge.clone(),
            ),
            Channel::of::<Power>("shunt_power", ChannelKind::ShuntPower, trace.shunt.clone()),
        ],
    };
    for (i, panel) in ctx.input.array.panels.iter().enumerate() {
        series.channels.push(
            Channel::of::<Power>(
                format!("panel:{}", panel.id),
                ChannelKind::PanelGeneration,
                ctx.profile.panel(i, ctx.input.life),
            )
            .about(&panel.id),
        );
    }
    series
}

fn energy_balance_check(r: &EnergyBalanceCaseResult) -> Check {
    let (verdict, message) = match r.energy_margin_per_orbit {
        Some(margin) => (
            Verdict::from_ok(margin.value() >= 0.0),
            format!(
                "1 周あたりのエネルギー余裕 {}（電池の効率込み）",
                quantity_text::<Energy>(margin.value())
            ),
        ),
        None => (
            Verdict::Warning,
            format!(
                "解析期間が 1 周に満たないため 1 周あたりの余裕を判定できない{}",
                r.energy_margin_over_span
                    .map_or_else(String::new, |margin| format!(
                        "（解析期間の収支は {}、電池の効率込み）",
                        quantity_text::<Energy>(margin.value())
                    ))
            ),
        ),
    };
    Check::new("energy_balance", "周回の電力収支", verdict, message)
}

fn verdicts(ctx: &Context, r: &EnergyBalanceCaseResult, depleted: bool) -> Vec<Check> {
    let limit = ctx.battery.dod_limit;
    let dod_ok = r.max_dod.value() <= limit + DOD_TOLERANCE;
    let mut checks = vec![
        Check::new(
            "dod",
            "DoD の上限",
            Verdict::from_ok(dod_ok),
            format!(
                "最大 DoD {}（上限 {}）",
                quantity_text::<Fraction>(r.max_dod.value()),
                quantity_text::<Fraction>(limit)
            ),
        ),
        energy_balance_check(r),
    ];
    if depleted {
        checks.push(Check::new(
            "depleted",
            "電池切れ",
            Verdict::Violation,
            "電池が空になり、負荷をまかなえない時間があります",
        ));
    }
    checks
}
