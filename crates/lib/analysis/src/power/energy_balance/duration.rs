//! 最長継続時間での評価: 基本のモードを回し続け、その上でケースのモードを最長継続時間だけ 1 回続けたときの DoD。
//!
//! 開始時刻はモードの時刻の指定（`start`・`when`。[`super::timing`]）に従って選び、指定が無いときは 1 周の中で
//! サンプルごとにずらして DoD が最大の開始を取る（蝕に重なる開始が最悪になりやすい）。窓はサンプルの刻みで丸める
//! （窓の中のサンプル `t_k ≤ t < t_k + 最長継続時間` にモードの負荷・発電を使い、刻みの間は線形補間）。
//!
//! 判定に使う DoD・電池切れは窓から後だけの値（解析の始めの過渡を含まない。始めの過渡は基本のモードの連続のケースが判定する）。

use super::case::{lit_load, mode_loads};
use super::context::mode_generation;
use super::timing::{DurationTiming, ModeTiming, TimeWindow, anchor_name, select_starts};
use super::verdict::{Judged, verdicts};
use super::{
    Context, EnergyBalanceCase, EnergyBalanceCaseResult, EnergyBalanceEvaluation, EnergyBalanceLoad,
};
use crate::power::battery::{self, BatteryTrace};
use crate::spec::{
    AnalysisError, Channel, ChannelKind, Check, Series, Verdict, ensure_positive, quantity_text,
};
use sat_astro::timeline::time_mean;
use sat_units::typed::{Energy, Fraction, Power, Qty, Time};
use serde::{Deserialize, Serialize};

/// 最長継続時間で評価した結果（DoD が最大の開始時刻のもの）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EnergyBalanceDuration {
    /// 下に敷いた基本のモード
    pub base_mode: String,
    /// モードを続けた時間（モードの最長継続時間）
    pub max_duration: Qty<Time>,
    /// 開始をどう決めたか（指定の開始・許された範囲でずらした・タイミングの指定なし）
    #[serde(default)]
    pub timing: DurationTiming,
    /// ずらした開始時刻の数
    pub starts: usize,
    /// 最悪の開始時刻（元期からの経過）
    pub worst_start: Qty<Time>,
    /// 最悪の開始時刻の周回内の位置（解析の開始から周期で割った余り）
    pub worst_start_in_orbit: Qty<Time>,
    /// 最悪の開始での、窓が日陰に重なる時間
    pub eclipse_overlap: Qty<Time>,
    /// 最悪の開始での最小 SOC（解析期間全体）
    pub min_soc: Fraction,
    /// 最悪の開始での最大 DoD（1 − 解析期間全体の最小 SOC。解析の始めの過渡を含むので参考値）
    pub max_dod: Fraction,
    /// 最悪の開始での、窓から後だけの最大 DoD（解析の始めの過渡を含まない。最悪の開始はこれで選び、判定にも使う）
    pub max_dod_from_start: Fraction,
    /// 最良の開始での、窓から後だけの最大 DoD（開始によるばらつきを見るため）
    pub best_max_dod: Fraction,
    /// 基本のモードだけを回したときの最大 DoD
    pub base_max_dod: Fraction,
    /// 最悪の開始から 1 周の軌道平均の発電 − 負荷（1 周に 1 回このモードを入れたとき。電池の効率を含まない）
    pub orbit_average_balance: Qty<Power>,
    /// 最悪の開始から 1 周のエネルギー余裕（電池の効率込み、上下限で切る前）。開始から 1 周が期間に収まらなければ None
    pub energy_margin_per_orbit: Option<Qty<Energy>>,
    /// 窓から後に、電池が空になり負荷をまかなえない時間があるか（判定に使う。始めの過渡は含まない）
    pub depleted: bool,
    /// 最悪の開始の時系列: `load`・`soc`
    pub series: Series,
}

/// 最長継続時間の評価で下に敷く基本のモード（負荷と発電の列）。
pub(super) struct BaseMode {
    id: String,
    load: Vec<f64>,
    generation: Vec<f64>,
}

impl BaseMode {
    /// 基本のモード（`base_mode`、無ければ並びの最初の区間）。どちらも無ければ None。
    pub(super) fn new(ctx: &Context) -> Result<Option<Self>, AnalysisError> {
        let input = ctx.input;
        let id = input
            .base_mode
            .clone()
            .or_else(|| input.sequence.first().map(|s| s.mode.clone()));
        let Some(id) = id else {
            return Ok(None);
        };
        // 基本のモードの警告はそのモードのケースが出す
        let (sun, ecl) = mode_loads(input, &id, &mut Vec::new())?;
        Ok(Some(Self {
            load: lit_load(&ctx.profile.lit, sun, ecl),
            generation: mode_generation(ctx, &id)?,
            id,
        }))
    }
}

/// 最長継続時間を持つモードのケースを、基本のモードの上で評価し直す（判定を差し替える）。
/// 最長継続時間が無い・基本のモードが無い・自身が基本のモードのときは連続のまま、`evaluation` にそう書く。
/// 差し替えたときは、最悪の開始での刻みごとのエネルギーの増分（容量の上下限で切る前）を返す（時系列の逆算が使う）。
pub(super) fn apply(
    ctx: &Context,
    case: &EnergyBalanceCase,
    base: Option<&BaseMode>,
    result: &mut EnergyBalanceCaseResult,
) -> Result<Option<Vec<f64>>, AnalysisError> {
    let (EnergyBalanceLoad::Mode { mode }, Some(d)) = (&case.load, case.max_duration) else {
        return Ok(None);
    };
    ensure_positive("Longest mode duration", d.value())?;
    let Some(base) = base else {
        result.evaluation = EnergyBalanceEvaluation::NoBaseMode;
        return Ok(None);
    };
    if base.id == *mode {
        result.evaluation = EnergyBalanceEvaluation::BaseMode;
        return Ok(None);
    }
    let load = lit_load(&ctx.profile.lit, result.load_sunlit, result.load_eclipse);
    let Some((duration, delta, note)) = sweep(ctx, base, &load, d.value(), &case.timing) else {
        return Ok(None);
    };
    let basis = format!(
        "when the base mode {} is sustained on top for the longest duration of {} ({})",
        base.id,
        quantity_text::<Time>(d.value()),
        timing_basis(duration.timing, &case.timing)
    );
    result
        .checks
        .retain(|c| !matches!(c.key.as_str(), "dod" | "energy_balance" | "depleted"));
    result.checks.extend(verdicts(
        ctx.battery.dod_limit,
        &Judged {
            max_dod: duration.max_dod_from_start.value(),
            margin: duration.energy_margin_per_orbit,
            margin_over_span: None,
            depleted: duration.depleted,
        },
        &basis,
    ));
    if let Some(note) = note {
        result
            .checks
            .push(Check::new("timing", "Mode timing", Verdict::Warning, note));
    }
    result.evaluation = EnergyBalanceEvaluation::MaxDuration;
    result.by_max_duration = Some(duration);
    Ok(Some(delta))
}

/// 判定の括弧書きのうち、開始の決め方の部分。
fn timing_basis(how: DurationTiming, t: &ModeTiming) -> String {
    match (how, &t.start) {
        (DurationTiming::Fixed, Some(r)) => format!(
            "start fixed at {} {} {}",
            anchor_name(r.from),
            if r.offset.value() < 0.0 { "-" } else { "+" },
            quantity_text::<Time>(r.offset.value().abs())
        ),
        (DurationTiming::Windowed, _) => format!(
            "worst start within the allowed {} windows",
            if t.when == TimeWindow::Sunlit {
                "sunlit"
            } else {
                "eclipse"
            }
        ),
        _ => "worst start within the orbit; timing unspecified".to_owned(),
    }
}

/// 正味電力の列（基本のモードと窓の中のモード）と、窓。
struct Window<'a> {
    base_net: Vec<f64>,
    mode_net: Vec<f64>,
    base: &'a BaseMode,
    mode_load: &'a [f64],
    duration: f64,
}

impl Window<'_> {
    /// 開始 `k` の窓にサンプル `j` が入るか。
    fn contains(&self, t: &[f64], k: usize, j: usize) -> bool {
        j >= k && t[j] < t[k] + self.duration
    }

    fn net(&self, t: &[f64], k: usize, j: usize) -> f64 {
        if self.contains(t, k, j) {
            self.mode_net[j]
        } else {
            self.base_net[j]
        }
    }

    fn load(&self, t: &[f64], k: usize, j: usize) -> f64 {
        if self.contains(t, k, j) {
            self.mode_load[j]
        } else {
            self.base.load[j]
        }
    }
}

/// 開始 `k` の窓で、基本のモードの積分の途中（`k − 1`）から積分し直した分。
struct Run {
    /// 積分し直した最初のサンプル
    from: usize,
    trace: BatteryTrace,
    /// 積分し直した分（窓の立ち上がりから後）の最小 SOC。最悪の開始はこれで選ぶ
    tail_min: f64,
    /// 解析期間全体の最小 SOC
    min_soc: f64,
}

/// 基本のモードだけを回した積分と、各サンプルまでの最小 SOC。
struct BaseRun {
    trace: BatteryTrace,
    prefix_min: Vec<f64>,
}

impl BaseRun {
    fn new(ctx: &Context, w: &Window) -> Self {
        let trace = battery::simulate(ctx.battery, &ctx.profile.t, &w.base_net);
        let capacity = ctx.battery.capacity.value();
        let prefix_min = trace
            .energy
            .iter()
            .scan(f64::INFINITY, |m, e| {
                *m = m.min(e / capacity);
                Some(*m)
            })
            .collect();
        Self { trace, prefix_min }
    }

    /// 開始 `k` の窓を入れて、窓の立ち上がりの刻み（`k − 1`）から積分し直す。
    fn rerun(&self, ctx: &Context, w: &Window, k: usize) -> Run {
        let t = &ctx.profile.t;
        let capacity = ctx.battery.capacity.value();
        let from = k.saturating_sub(1);
        let mut battery = ctx.battery.clone();
        battery.initial_soc = Some(self.trace.energy[from] / capacity);
        let net: Vec<f64> = (from..t.len()).map(|j| w.net(t, k, j)).collect();
        let trace = battery::simulate(&battery, &t[from..], &net);
        let tail = trace
            .energy
            .iter()
            .fold(f64::INFINITY, |m, e| m.min(e / capacity));
        let before = from
            .checked_sub(1)
            .map_or(f64::INFINITY, |i| self.prefix_min[i]);
        Run {
            from,
            trace,
            tail_min: tail.min(1.0),
            min_soc: tail.min(before).min(1.0),
        }
    }
}

/// 開始時刻を指定に従って選び（[`select_starts`]）、その中で最悪を取る。最悪は窓から後の最小 SOC が最も低い開始
/// （解析の始めの過渡は開始によらないので比べない）。開始が 1 つに決まっていればそれだけ。
/// 最悪の開始での刻みごとのエネルギーの増分（基本のモードの積分の前半と積分し直した分をつないだもの）と、
/// 指定どおりに選べなかった理由（あれば）も返す。サンプルが無ければ None。
fn sweep(
    ctx: &Context,
    base: &BaseMode,
    mode_load: &[f64],
    duration: f64,
    timing: &ModeTiming,
) -> Option<(EnergyBalanceDuration, Vec<f64>, Option<String>)> {
    let t = &ctx.profile.t;
    if t.is_empty() {
        return None;
    }
    let net = |g: &[f64], l: &[f64]| g.iter().zip(l).map(|(g, l)| g - l).collect();
    let w = Window {
        base_net: net(&base.generation, &base.load),
        mode_net: net(&ctx.generation, mode_load),
        base,
        mode_load,
        duration,
    };
    let base_run = BaseRun::new(ctx, &w);
    let selection = select_starts(t, &ctx.profile.lit, ctx.period_s(), duration, timing);
    let starts = selection.starts;
    let mut worst: Option<(usize, Run)> = None;
    let mut best_dod = f64::INFINITY;
    for &k in &starts {
        let r = base_run.rerun(ctx, &w, k);
        best_dod = best_dod.min(1.0 - r.tail_min);
        if worst.as_ref().is_none_or(|(_, w)| r.tail_min < w.tail_min) {
            worst = Some((k, r));
        }
    }
    let (k, r) = worst?;
    let base_min = base_run.prefix_min.last().copied().unwrap_or(1.0).min(1.0);
    let delta = base_run.trace.delta_unclamped[..r.from]
        .iter()
        .chain(&r.trace.delta_unclamped)
        .copied()
        .collect();
    let mut summary = summarize(
        ctx,
        &w,
        (k, &r, starts.len()),
        &base_run.trace,
        (best_dod, base_min),
    );
    summary.timing = selection.timing;
    Some((summary, delta, selection.note))
}

/// 最悪の開始の結果をまとめる。
fn summarize(
    ctx: &Context,
    w: &Window,
    (k, r, starts): (usize, &Run, usize),
    base_trace: &BatteryTrace,
    (best_dod, base_min): (f64, f64),
) -> EnergyBalanceDuration {
    let t = &ctx.profile.t;
    let n = t.len();
    let capacity = ctx.battery.capacity.value();
    let in_window: Vec<usize> = (k..n).filter(|&j| w.contains(t, k, j)).collect();
    let eclipse_overlap: f64 = in_window
        .iter()
        .filter(|&&j| j + 1 < n && !ctx.profile.lit[j])
        .map(|&j| t[j + 1] - t[j])
        // 空の和は -0.0 になる（`Sum` の単位元）ので 0.0 から足す
        .fold(0.0, |total, dt| total + dt);
    let (balance, margin) = orbit_after(ctx, w, k, r);
    let load: Vec<f64> = (0..n).map(|j| w.load(t, k, j)).collect();
    let soc: Vec<f64> = base_trace.energy[..r.from]
        .iter()
        .chain(&r.trace.energy)
        .map(|e| e / capacity)
        .collect();
    let phase = if ctx.period_s() > 0.0 {
        (t[k] - t[0]).rem_euclid(ctx.period_s())
    } else {
        0.0
    };
    EnergyBalanceDuration {
        base_mode: w.base.id.clone(),
        timing: DurationTiming::Unspecified,
        max_duration: Qty::from_raw(w.duration),
        starts,
        worst_start: Qty::from_raw(t[k]),
        worst_start_in_orbit: Qty::from_raw(phase),
        eclipse_overlap: Qty::from_raw(eclipse_overlap),
        min_soc: Fraction::new(r.min_soc),
        max_dod: Fraction::new(1.0 - r.min_soc),
        max_dod_from_start: Fraction::new(1.0 - r.tail_min),
        best_max_dod: Fraction::new(best_dod),
        base_max_dod: Fraction::new(1.0 - base_min),
        orbit_average_balance: Qty::from_raw(balance),
        energy_margin_per_orbit: margin.map(Qty::from_raw),
        depleted: r.trace.depleted,
        series: Series {
            t: t.clone(),
            channels: vec![
                Channel::of::<Power>("load", ChannelKind::Load, load),
                Channel::of::<Fraction>("soc", ChannelKind::Soc, soc),
            ],
        },
    }
}

/// 開始 `k` から 1 周の平均の収支 [W] と、エネルギー余裕 [J]（1 周が期間に収まらなければ None）。
/// 1 周は窓の立ち上がりの刻み（`r.from`）から測る。終わりが刻みの途中なら、最後の刻みの増分を時間の割合で按分する。
fn orbit_after(ctx: &Context, w: &Window, k: usize, r: &Run) -> (f64, Option<f64>) {
    let t = &ctx.profile.t;
    let (from, period) = (r.from, ctx.period_s());
    let tol = 1e-9 * period.abs().max(1.0);
    let end = (from + 1..t.len()).find(|&j| t[j] - t[from] >= period - tol);
    let last = end.unwrap_or(t.len() - 1);
    let net: Vec<f64> = (from..=last).map(|j| w.net(t, k, j)).collect();
    let mean = time_mean(&t[from..=last], &net);
    let balance = if mean.is_finite() {
        mean
    } else {
        net.first().copied().unwrap_or(0.0)
    };
    let Some(end) = end else {
        return (balance, None);
    };
    let delta = |j: usize| r.trace.delta_unclamped[j - from];
    let full: f64 = (from..end - 1).map(delta).sum();
    let step = t[end] - t[end - 1];
    let frac = if step > 0.0 {
        ((t[from] + period - t[end - 1]) / step).clamp(0.0, 1.0)
    } else {
        1.0
    };
    (balance, Some(full + frac * delta(end - 1)))
}
