//! 区間の積分（適応刻み・イベント付き）。`Dop853` の密出力で、区間の途中で条件が成り立つ時刻を求めて止める。
//!
//! 熱のように、入力が区間ごとに滑らかで、途中でスイッチが切り替わる（サーモスタット）系向け。
//! 切り替えの時刻は 1 刻みの中の密出力の補間に対する二分法（`math::bisect`）で求めるので、刻みの端に量子化されない。

use super::dop853::{
    MAX_REJECTIONS, accept_factor, error_exponent, extend_for_dense, initial_step, reject_factor,
    try_step, validate,
};
use super::{Dop853, OdeState};
use crate::AstroError;
use crate::math::bisect;
use core::cell::Cell;

/// 積分の費用の数え上げ。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OdeStats {
    /// 受理した刻みの数
    pub accepted: usize,
    /// 誤差が許容を超えて棄却した刻みの数
    pub rejected: usize,
    /// 右辺の評価の回数
    pub rhs_evals: usize,
    /// イベントで止まった回数
    pub events: usize,
}

impl OdeStats {
    /// 別の区間の数え上げを足す。
    pub fn add(&mut self, other: &Self) {
        self.accepted += other.accepted;
        self.rejected += other.rejected;
        self.rhs_evals += other.rhs_evals;
        self.events += other.events;
    }
}

/// [`Dop853::integrate_segment`] の結果。
#[derive(Debug, Clone)]
pub struct Segment<S> {
    /// 止まった時刻（イベントがなければ区間の終わり）
    pub t: f64,
    /// 止まった時刻の状態
    pub y: S,
    /// イベント（`event` が 0 以下になった）で止まったか
    pub event: bool,
    /// 次の区間の初期刻みの目安（刻み制御の提案）
    pub h_next: f64,
    pub stats: OdeStats,
}

/// 刻み内のイベント時刻の許容幅 [s]（刻みに対する比と、時刻の浮動小数点の粒度の大きい方）。
fn root_tolerance(h: f64, t: f64) -> f64 {
    (1e-9 * h).max(1e-13 * t.abs().max(1.0))
}

/// 1 刻みの中で `event` が 0 になる時刻と、その時刻の状態（密出力の補間）。
fn locate<S: OdeState>(
    event: &impl Fn(f64, &S) -> f64,
    step: &super::dense_output::Step<S>,
    t_new: f64,
) -> (f64, S) {
    let root = bisect(
        |t| event(t, &step.interpolate(t)),
        step.t0,
        t_new,
        root_tolerance(step.h, step.t0),
        100,
    );
    match root {
        Some(t) if t < t_new => (t, step.interpolate(t)),
        _ => (t_new, step.y1.clone()),
    }
}

pub(super) fn integrate_segment<S: OdeState>(
    cfg: &Dop853,
    f: &impl Fn(f64, &S) -> S,
    t0: f64,
    x0: S,
    t1: f64,
    h_start: Option<f64>,
    event: &impl Fn(f64, &S) -> f64,
) -> Result<Segment<S>, AstroError> {
    validate(cfg, t0, &x0, &[t1])?;
    let evals = Cell::new(0usize);
    let fc = |t: f64, x: &S| {
        evals.set(evals.get() + 1);
        f(t, x)
    };
    let mut stats = OdeStats::default();
    let done = |t: f64, y: S, event: bool, h_next: f64, mut stats: OdeStats, evals: usize| {
        stats.rhs_evals = evals;
        stats.events = usize::from(event);
        Ok(Segment {
            t,
            y,
            event,
            h_next,
            stats,
        })
    };
    if t1 <= t0 {
        return done(t0, x0, false, h_start.unwrap_or(f64::INFINITY), stats, 0);
    }
    if event(t0, &x0) <= 0.0 {
        return done(t0, x0, true, h_start.unwrap_or(f64::INFINITY), stats, 0);
    }
    let (mut t, mut y) = (t0, x0);
    let mut k0 = fc(t, &y);
    let mut h = h_start.map_or_else(
        || initial_step(cfg, &fc, t0, &y, &k0, t1 - t0),
        |h| h.min(cfg.max_step),
    );
    let (mut facold, mut rejections) = (1e-4_f64, 0usize);
    let expo = error_exponent();
    for _ in 0..cfg.max_steps {
        let remaining = t1 - t;
        h = h.min(remaining);
        if 0.1 * h <= t.abs() * f64::EPSILON || h <= 0.0 {
            return Err(AstroError::InvalidInput(format!(
                "DOP853 step size became too small (t = {t})"
            )));
        }
        let (mut step, err) = try_step(cfg, &fc, t, &y, &k0, h)?;
        let fac11 = err.powf(expo);
        if err > 1.0 {
            rejections += 1;
            stats.rejected += 1;
            if rejections > MAX_REJECTIONS {
                return Err(AstroError::InvalidInput(format!(
                    "DOP853 cannot meet the tolerance (t = {t})"
                )));
            }
            h /= reject_factor(fac11);
            continue;
        }
        stats.accepted += 1;
        rejections = 0;
        let h_next = (h / accept_factor(fac11, facold)).min(cfg.max_step);
        facold = err.max(1e-4);
        let t_new = if h == remaining { t1 } else { t + h };
        if event(t_new, &step.y1) <= 0.0 {
            extend_for_dense(&fc, &mut step);
            let (te, ye) = locate(event, &step, t_new);
            return done(te, ye, true, h_next, stats, evals.get());
        }
        k0 = step.k[super::dop853_coeffs::N_STAGES].clone();
        (y, t, h) = (step.y1, t_new, h_next);
        if t >= t1 {
            return done(t1, y, false, h_next, stats, evals.get());
        }
    }
    Err(AstroError::InvalidInput(format!(
        "DOP853 exceeded the maximum step count {}",
        cfg.max_steps
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> Dop853 {
        Dop853 {
            rtol: 1e-10,
            atol: 1e-12,
            ..Dop853::default()
        }
    }

    /// dx/dt = −x、x(0) = 1 が 0.3 に下がる時刻は −ln 0.3。刻みの端ではなく密出力の補間で求まる。
    #[test]
    fn event_time_is_located_inside_a_step() {
        let f = |_t: f64, x: &[f64; 1]| [-x[0]];
        let seg = cfg()
            .integrate_segment(f, 0.0, [1.0], 10.0, None, |_t, x: &[f64; 1]| x[0] - 0.3)
            .unwrap();
        assert!(seg.event && seg.stats.events == 1);
        assert!((seg.t - (-(0.3_f64).ln())).abs() < 1e-8, "{}", seg.t);
        assert!((seg.y[0] - 0.3).abs() < 1e-9, "{}", seg.y[0]);
        assert!(seg.stats.accepted >= 1 && seg.stats.rhs_evals >= 12);
    }

    /// イベントで止めて、そこから続けても、止めずに積分した結果と同じ。
    #[test]
    fn restarting_after_an_event_matches_the_exact_solution() {
        let f = |_t: f64, x: &[f64; 1]| [-x[0]];
        let first = cfg()
            .integrate_segment(f, 0.0, [1.0], 5.0, None, |_t, x: &[f64; 1]| x[0] - 0.3)
            .unwrap();
        let rest = cfg()
            .integrate_segment(
                f,
                first.t,
                first.y,
                5.0,
                Some(first.h_next),
                |_t, _x: &[f64; 1]| 1.0,
            )
            .unwrap();
        assert!(!rest.event && rest.t == 5.0);
        assert!((rest.y[0] - (-5.0_f64).exp()).abs() < 1e-9, "{}", rest.y[0]);
    }

    /// イベントが起きなければ区間の終わりまで進み、最大刻みを守る。
    #[test]
    fn no_event_runs_to_the_end_and_respects_max_step() {
        let f = |_t: f64, x: &[f64; 1]| [-x[0]];
        let small = Dop853 {
            max_step: 0.1,
            ..cfg()
        };
        let seg = small
            .integrate_segment(f, 0.0, [1.0], 2.0, None, |_t, _x: &[f64; 1]| 1.0)
            .unwrap();
        assert!(!seg.event && seg.t == 2.0);
        assert!(seg.stats.accepted >= 20, "{:?}", seg.stats);
        // 始めからイベントの条件を満たすなら、進まずに返す
        let at_start = cfg()
            .integrate_segment(f, 1.0, [1.0], 2.0, None, |_t, _x: &[f64; 1]| -1.0)
            .unwrap();
        assert!(at_start.event && at_start.t == 1.0);
    }
}
