//! 1 ケースの評価（負荷の決定・電池の積分・周回ごとの集計・判定）。

use super::verdict::{Judged, verdicts};
use super::{
    Context, EnergyBalanceCase, EnergyBalanceCaseResult, EnergyBalanceDuty,
    EnergyBalanceEvaluation, EnergyBalanceInput, EnergyBalanceLoad, EnergyBalanceOrbit, LoadGrowth,
};
use crate::modes::find_mode;
use crate::power::battery::{self, BatteryTrace};
use crate::power::mode_load::evaluate_mode;
use crate::spec::{
    AnalysisError, Channel, ChannelKind, Check, Series, SpanFallback, orbit_mean, orbit_segment,
    span_fallback,
};
use sat_units::typed::{Energy, Fraction, Power, Qty, Time};

/// モードの日照中・日陰中の負荷（成長余裕の扱いは `load_growth`）。モードの警告は `checks` に足す（同じものは足さない）。
pub(super) fn mode_loads(
    input: &EnergyBalanceInput,
    id: &str,
    checks: &mut Vec<Check>,
) -> Result<(Qty<Power>, Qty<Power>), AnalysisError> {
    let r = evaluate_mode(&input.loads, find_mode(&input.loads.modes, id)?)?;
    for c in r.checks {
        if !checks
            .iter()
            .any(|x| x.key == c.key && x.message == c.message)
        {
            checks.push(c);
        }
    }
    Ok(match input.load_growth() {
        LoadGrowth::Included => (r.total_with_growth, r.total_eclipse_with_growth),
        LoadGrowth::Excluded => (r.total, r.total_eclipse),
    })
}

/// ケースの負荷（日照中・日陰中）を求める。モードの警告は `checks` に足す。
pub(super) fn case_loads(
    input: &EnergyBalanceInput,
    case: &EnergyBalanceCase,
    checks: &mut Vec<Check>,
) -> Result<(Qty<Power>, Qty<Power>), AnalysisError> {
    match &case.load {
        EnergyBalanceLoad::Mode { mode } => mode_loads(input, mode, checks),
        EnergyBalanceLoad::SunlitEclipse {
            sunlit_mode,
            eclipse_mode,
        } => Ok((
            mode_loads(input, sunlit_mode, checks)?.0,
            mode_loads(input, eclipse_mode, checks)?.1,
        )),
        EnergyBalanceLoad::Constant { power } => Ok((*power, *power)),
    }
}

/// ケースが使うモードの ID（一定の負荷なら空）。
pub(super) fn case_modes(case: &EnergyBalanceCase) -> Vec<&str> {
    match &case.load {
        EnergyBalanceLoad::Mode { mode } => vec![mode.as_str()],
        EnergyBalanceLoad::SunlitEclipse {
            sunlit_mode,
            eclipse_mode,
        } => vec![sunlit_mode.as_str(), eclipse_mode.as_str()],
        EnergyBalanceLoad::Constant { .. } => Vec::new(),
    }
}

/// 使うモードのどれかが duty（時間割合）を持てば平均として扱った印。
pub(super) fn duty_of<'a>(
    input: &EnergyBalanceInput,
    modes: impl IntoIterator<Item = &'a str>,
) -> EnergyBalanceDuty {
    let averaged = modes
        .into_iter()
        .any(|id| find_mode(&input.loads.modes, id).is_ok_and(crate::modes::ModeDef::uses_duty));
    if averaged {
        EnergyBalanceDuty::Averaged
    } else {
        EnergyBalanceDuty::NotUsed
    }
}

/// 日照中のサンプルは `sun`、日陰中は `ecl` の負荷の列。
pub(super) fn lit_load(lit: &[bool], sun: Qty<Power>, ecl: Qty<Power>) -> Vec<f64> {
    lit.iter()
        .map(|&f| if f { sun.value() } else { ecl.value() })
        .collect()
}

/// 連続で回したときの判定の括弧書き。
const CONTINUOUS_BASIS: &str = "when sustained over the whole analysis span";

/// 1 ケースを解析期間ずっと続けたとして評価する（最長継続時間の評価は [`super::duration`] が上書きする）。
/// 電池の積分（`trace`）も返す（時系列の逆算が刻みごとの増分を使う）。
pub(super) fn evaluate_case(
    ctx: &Context,
    case: &EnergyBalanceCase,
) -> Result<(EnergyBalanceCaseResult, BatteryTrace), AnalysisError> {
    let mut checks = Vec::new();
    let (sun, ecl) = case_loads(ctx.input, case, &mut checks)?;
    let load = lit_load(&ctx.profile.lit, sun, ecl);
    let (mut result, trace) = balance(
        ctx,
        (&case.id, &case.label),
        (sun, ecl),
        load,
        checks,
        CONTINUOUS_BASIS,
    );
    result.duty = duty_of(ctx.input, case_modes(case));
    Ok((result, trace))
}

/// 負荷の時系列 `load`（発電は `ctx.generation`）で電池を積分し、1 ケースの結果を作る。
/// `checks` はモードの警告など、判定の前に置くもの。`basis` はどの運用で回したか（判定の文の括弧書き）。
/// 電池の積分（`trace`）も返す。
pub(super) fn balance(
    ctx: &Context,
    (id, label): (&str, &str),
    (sun, ecl): (Qty<Power>, Qty<Power>),
    load: Vec<f64>,
    mut checks: Vec<Check>,
    basis: &str,
) -> (EnergyBalanceCaseResult, BatteryTrace) {
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
    let (margin, margin_over_span) = margins(ctx, &orbits, &trace);
    let peak = |v: &[f64]| v.iter().copied().fold(0.0, f64::max);
    let peak_discharge = peak(&trace.discharge);
    let orbit_gen = orbit_mean(&ctx.profile.t, &ctx.generation, &ctx.ranges);
    let orbit_load = orbit_mean(&ctx.profile.t, &load, &ctx.ranges);
    let sunlit_mean = |v: &[f64]| {
        Qty::from_raw(
            crate::spec::masked_time_mean(&ctx.profile.t, v, &ctx.profile.lit).unwrap_or(0.0),
        )
    };
    checks.extend(verdicts(
        ctx.battery.dod_limit,
        &Judged {
            max_dod: 1.0 - min_soc,
            margin,
            margin_over_span,
            depleted: trace.depleted,
        },
        basis,
    ));
    let result = EnergyBalanceCaseResult {
        id: id.to_owned(),
        label: label.to_owned(),
        load_sunlit: sun,
        load_eclipse: ecl,
        orbit_average_generation: Qty::from_raw(orbit_gen),
        sunlit_average_generation: sunlit_mean(&ctx.generation),
        sunlit_average_charge: sunlit_mean(&trace.charge),
        sunlit_average_discharge: sunlit_mean(&trace.discharge),
        sunlit_average_shunt: sunlit_mean(&trace.shunt),
        orbit_average_load: Qty::from_raw(orbit_load),
        orbit_average_balance: Qty::from_raw(orbit_gen - orbit_load),
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
        checks,
        evaluation: EnergyBalanceEvaluation::Continuous,
        by_max_duration: None,
        duty: EnergyBalanceDuty::NotUsed,
    };
    (result, trace)
}

/// 1 周あたりのエネルギー余裕（周回の最小）と、1 周に満たないときの解析期間全体の収支。
fn margins(
    ctx: &Context,
    orbits: &[EnergyBalanceOrbit],
    trace: &BatteryTrace,
) -> (Option<Qty<Energy>>, Option<Qty<Energy>>) {
    let margin = orbits
        .iter()
        .map(|o| o.margin.value())
        .reduce(f64::min)
        .map(Qty::from_raw);
    let over_span = span_fallback(ctx.ranges.len(), SpanFallback::OrbitMeanOverSpan)
        .map(|_| Qty::from_raw(trace.delta_unclamped.iter().sum()));
    (margin, over_span)
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
                    * Qty::<Time>::from_raw(ctx.period_s())
            };
            let seg = orbit_segment(r, soc.len());
            let (start, end) = (*seg.start(), *seg.end());
            // サンプルの区間の長さを周期ちょうどに揃える
            let covered = ctx.profile.t[end] - ctx.profile.t[start];
            let scale = if covered > 0.0 {
                ctx.period_s() / covered
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
                ctx.profile.panel(i, ctx.input.life()),
            )
            .about(&panel.id),
        );
    }
    series
}
