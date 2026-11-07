//! 1 ケースの評価（負荷の決定・電池の積分・周回ごとの集計・判定）。

use super::{
    Context, EnergyBalanceCase, EnergyBalanceCaseResult, EnergyBalanceInput, EnergyBalanceLoad,
    EnergyBalanceOrbit,
};
use crate::power::battery::{self, BatteryTrace};
use crate::power::mode_load::evaluate_mode;
use crate::power::orbit_generation::span_checks;
use crate::spec::quantity_text;
use crate::spec::{AnalysisError, Channel, Check, Series, Verdict, orbit_mean};
use sat_units::typed::{Fraction, Power, Qty, Time};

/// ケースの負荷（日照中・日陰中）を求める。モードの警告は `checks` に足す。
fn case_loads(
    input: &EnergyBalanceInput,
    case: &EnergyBalanceCase,
    checks: &mut Vec<Check>,
) -> Result<(Qty<Power>, Qty<Power>), AnalysisError> {
    let mut mode_power = |id: &str| -> Result<Qty<Power>, AnalysisError> {
        let mode = input
            .loads
            .modes
            .iter()
            .find(|m| m.id == id)
            .ok_or_else(|| {
                AnalysisError::InvalidInput(format!(
                    "ケース {} のモード {id} がありません",
                    case.id
                ))
            })?;
        let r = evaluate_mode(&input.loads, mode)?;
        for c in r.checks {
            if !checks
                .iter()
                .any(|x| x.key == c.key && x.message == c.message)
            {
                checks.push(c);
            }
        }
        Ok(if input.without_growth {
            r.total
        } else {
            r.total_with_growth
        })
    };
    match &case.load {
        EnergyBalanceLoad::Mode { mode } => {
            let p = mode_power(mode)?;
            Ok((p, p))
        }
        EnergyBalanceLoad::SunlitEclipse {
            sunlit_mode,
            eclipse_mode,
        } => Ok((mode_power(sunlit_mode)?, mode_power(eclipse_mode)?)),
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
        .flags
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
        .unwrap_or_else(|| trace.delta_unclamped.iter().sum());
    let peak = |v: &[f64]| v.iter().copied().fold(0.0, f64::max);
    let peak_discharge = peak(&trace.discharge);
    let orbit_load = orbit_mean(&ctx.profile.t, &load, &ctx.ranges);
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
        orbit_average_load: Qty::from_raw(orbit_load),
        orbit_average_balance: Qty::from_raw(
            orbit_mean(&ctx.profile.t, &ctx.generation, &ctx.ranges) - orbit_load,
        ),
        energy_margin_per_orbit: Qty::from_raw(margin),
        min_soc: Fraction::new(min_soc),
        max_dod: Fraction::new(1.0 - min_soc),
        max_eclipse_dod: Fraction::new(max_eclipse_drop(&ctx.flags, &soc)),
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
    let last = soc.len().saturating_sub(1);
    ctx.ranges
        .iter()
        .enumerate()
        .map(|(index, r)| {
            let to_energy = |v: &[f64]| {
                Qty::<Power>::from_raw(orbit_mean(&ctx.profile.t, v, std::slice::from_ref(r)))
                    * Qty::<Time>::from_raw(ctx.period_s)
            };
            let end = r.end.min(last);
            // 標本の区間の長さを周期ちょうどに揃える
            let covered = ctx.profile.t[end] - ctx.profile.t[r.start];
            let scale = if covered > 0.0 {
                ctx.period_s / covered
            } else {
                0.0
            };
            let margin: f64 = trace.delta_unclamped[r.start..end].iter().sum::<f64>() * scale;
            EnergyBalanceOrbit {
                index,
                start: Qty::from_raw(ctx.profile.t[r.start]),
                generation: to_energy(&ctx.generation),
                load: to_energy(load),
                margin: Qty::from_raw(margin),
                min_soc: Fraction::new(
                    soc[r.start..=end]
                        .iter()
                        .copied()
                        .fold(f64::INFINITY, f64::min),
                ),
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
    Series {
        t: ctx.profile.t.clone(),
        channels: vec![
            Channel::new("generation", "発電電力", "Power", ctx.generation.clone()),
            Channel::new("load", "消費電力", "Power", load),
            Channel::new("soc", "SOC", Fraction::KIND, soc),
            Channel::new("charge_power", "充電電力", "Power", trace.charge.clone()),
            Channel::new(
                "discharge_power",
                "放電電力",
                "Power",
                trace.discharge.clone(),
            ),
            Channel::new("shunt_power", "余剰電力", "Power", trace.shunt.clone()),
        ],
    }
}

fn verdicts(ctx: &Context, r: &EnergyBalanceCaseResult, depleted: bool) -> Vec<Check> {
    let limit = ctx.battery.dod_limit;
    let dod_ok = r.max_dod.value() <= limit + 1e-12;
    let mut checks = vec![
        Check::new(
            "dod",
            "DoD の上限",
            if dod_ok {
                Verdict::Ok
            } else {
                Verdict::Violation
            },
            format!(
                "最大 DoD {}（上限 {}）",
                quantity_text(r.max_dod.value(), Fraction::KIND),
                quantity_text(limit, Fraction::KIND)
            ),
        ),
        Check::new(
            "energy_balance",
            "周回の電力収支",
            if r.energy_margin_per_orbit.value() >= 0.0 {
                Verdict::Ok
            } else {
                Verdict::Violation
            },
            format!(
                "1 周あたりのエネルギー余裕 {}（電池の効率込み）",
                quantity_text(r.energy_margin_per_orbit.value(), "Energy")
            ),
        ),
    ];
    if depleted {
        checks.push(Check::new(
            "depleted",
            "電池切れ",
            Verdict::Violation,
            "電池が空になり、負荷をまかなえない時間があります",
        ));
    }
    checks.extend(span_checks(ctx.ranges.len()));
    checks
}
