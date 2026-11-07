// This file is a port of dop853.f by E. Hairer and G. Wanner, used under the following license
// (also reproduced in THIRD-PARTY-NOTICES.md).
//
// Copyright (c) 2004, UNIGE
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are
// met:
//
// - Redistributions of source code must retain the above copyright
// notice, this list of conditions and the following disclaimer.
//
// - Redistributions in binary form must reproduce the above copyright
// notice, this list of conditions and the following disclaimer in the
// documentation and/or other materials provided with the distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS “AS
// IS” AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
// TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A
// PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE REGENTS OR
// CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
// EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
// PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
// PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
// LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
// NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
// SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

//! DOP853（Dormand–Prince 8(5,3)、7 次の密出力付き）の本体。
//!
//! Hairer の `dop853.f` を移植し、任意の [`OdeState`] に一般化したもの。状態の操作は `axpy` だけで行い、
//! 誤差評価だけ成分（`as_slice`）を見る。密出力は「`y0 + Σ w_j(s)·k_j`」の形に直して、
//! 補間点ごとに重み `w_j` を計算する（状態型に成分からの構築を要求しないため）。

use super::Dop853;
use super::OdeState;
use super::dense_output::Step;
use super::dop853_coeffs::{A, C, E3, E5, N_STAGES, N_STAGES_EXTENDED};
use crate::AstroError;

/// 公式の次数（刻み制御の指数に使う）。
const ORDER: f64 = 8.0;
/// 安全係数。
const SAFETY: f64 = 0.9;
/// PI 制御の β（Hairer の推奨値）。
const BETA: f64 = 0.04;
/// 刻みの縮小・拡大の上限（h_new/h ∈ [0.333, 6]）。
const SHRINK_LIMIT: f64 = 0.333;
const GROW_LIMIT: f64 = 6.0;
/// 連続して棄却してよい回数。
const MAX_REJECTIONS: usize = 50;

fn is_finite<S: OdeState>(x: &S) -> bool {
    x.as_slice().iter().all(|v| v.is_finite())
}

/// `y0 + h Σ_j row[j] k_j`
fn stage_state<S: OdeState>(y0: &S, k: &[S], row: &[f64], h: f64) -> S {
    let mut ys = y0.clone();
    for (a, kj) in row.iter().zip(k.iter()) {
        if *a != 0.0 {
            ys = ys.axpy(h * a, kj);
        }
    }
    ys
}

fn scale(cfg: &Dop853, y0: f64, y1: f64) -> f64 {
    cfg.atol + cfg.rtol * y0.abs().max(y1.abs())
}

/// 1 ステップの試行。返り値は（ステップ、誤差ノルム）。追加段（密出力用）は受理後に計算する。
fn try_step<S: OdeState>(
    cfg: &Dop853,
    f: &impl Fn(f64, &S) -> S,
    t: f64,
    y0: &S,
    k0: &S,
    h: f64,
) -> Result<(Step<S>, f64), AstroError> {
    let mut k: Vec<S> = Vec::with_capacity(N_STAGES_EXTENDED);
    k.push(k0.clone());
    for stage in 1..N_STAGES {
        let ys = stage_state(y0, &k, &A[stage][..stage], h);
        if !is_finite(&ys) {
            return Err(not_finite(t));
        }
        k.push(f(t + C[stage] * h, &ys));
    }
    let y1 = stage_state(y0, &k, &A[N_STAGES][..N_STAGES], h);
    let k_end = f(t + h, &y1);
    if !is_finite(&y1) || !is_finite(&k_end) {
        return Err(not_finite(t));
    }
    let err = error_norm(cfg, y0, &y1, &k, h);
    k.push(k_end);
    Ok((
        Step {
            t0: t,
            h,
            y0: y0.clone(),
            y1,
            k,
        },
        err,
    ))
}

/// 受理したステップに密出力用の追加段（k13..k15）を足す。
fn extend_for_dense<S: OdeState>(f: &impl Fn(f64, &S) -> S, step: &mut Step<S>) {
    for stage in (N_STAGES + 1)..N_STAGES_EXTENDED {
        let ys = stage_state(&step.y0, &step.k, &A[stage][..stage], step.h);
        let kj = f(step.t0 + C[stage] * step.h, &ys);
        step.k.push(kj);
    }
}

fn error_norm<S: OdeState>(cfg: &Dop853, y0: &S, y1: &S, k: &[S], h: f64) -> f64 {
    let (y0s, y1s) = (y0.as_slice(), y1.as_slice());
    let n = y0s.len().max(1);
    let mut err3 = 0.0;
    let mut err5 = 0.0;
    for i in 0..y0s.len() {
        let sc = scale(cfg, y0s[i], y1s[i]);
        let mut e3 = 0.0;
        let mut e5 = 0.0;
        for (stage, kj) in k.iter().enumerate().take(N_STAGES) {
            let v = kj.as_slice()[i];
            e3 += E3[stage] * v;
            e5 += E5[stage] * v;
        }
        err3 += (e3 / sc) * (e3 / sc);
        err5 += (e5 / sc) * (e5 / sc);
    }
    let deno = if err5 + 0.01 * err3 > 0.0 {
        err5 + 0.01 * err3
    } else {
        1.0
    };
    h.abs() * err5 * (1.0 / (n as f64 * deno)).sqrt()
}

/// 初期刻み（Hairer の `hinit`）。
fn initial_step<S: OdeState>(
    cfg: &Dop853,
    f: &impl Fn(f64, &S) -> S,
    t0: f64,
    y0: &S,
    k0: &S,
    span: f64,
) -> f64 {
    let h_max = cfg.max_step.min(span);
    let (ys, ks) = (y0.as_slice(), k0.as_slice());
    let (mut dny, mut dnf) = (0.0, 0.0);
    for (y, k) in ys.iter().zip(ks.iter()) {
        let sc = scale(cfg, *y, *y);
        dny += (y / sc) * (y / sc);
        dnf += (k / sc) * (k / sc);
    }
    let h0 = if dnf <= 1e-10 || dny <= 1e-10 {
        1e-6
    } else {
        0.01 * (dny / dnf).sqrt()
    }
    .min(h_max);
    let y1 = y0.axpy(h0, k0);
    let k1 = f(t0 + h0, &y1);
    let mut der2 = 0.0;
    for ((y, a), b) in ys.iter().zip(ks.iter()).zip(k1.as_slice().iter()) {
        let d = (b - a) / scale(cfg, *y, *y);
        der2 += d * d;
    }
    let der2 = der2.sqrt() / h0;
    let der12 = der2.abs().max(dnf.sqrt());
    let h1 = if der12 <= 1e-15 {
        (h0 * 1e-3).max(1e-6)
    } else {
        (0.01 / der12).powf(1.0 / ORDER)
    };
    (100.0 * h0).min(h1).min(h_max)
}

fn not_finite(t: f64) -> AstroError {
    AstroError::InvalidInput(format!("積分中に非有限の値が出ました（t = {t}）"))
}

/// `t0` から昇順の `times` までを積分し、各時刻での密出力の値を返す。
pub(super) fn integrate_at<S: OdeState>(
    cfg: &Dop853,
    f: &impl Fn(f64, &S) -> S,
    t0: f64,
    x0: S,
    times: &[f64],
) -> Result<Vec<S>, AstroError> {
    validate(cfg, t0, &x0, times)?;
    let mut out = Vec::with_capacity(times.len());
    let mut idx = 0;
    while idx < times.len() && times[idx] <= t0 {
        out.push(x0.clone());
        idx += 1;
    }
    let Some(&t_end) = times.last() else {
        return Ok(out);
    };
    if idx == times.len() {
        return Ok(out);
    }
    let mut t = t0;
    let mut y = x0;
    let mut k0 = f(t, &y);
    let mut h = initial_step(cfg, f, t0, &y, &k0, t_end - t0);
    let mut facold = 1e-4_f64;
    let mut last_rejected = false;
    let mut rejections = 0usize;
    let expo = 1.0 / ORDER - BETA * 0.2;
    for _ in 0..cfg.max_steps {
        let remaining = t_end - t;
        if remaining <= 0.0 {
            return Ok(out);
        }
        h = h.min(remaining);
        if 0.1 * h <= t.abs() * f64::EPSILON || h <= 0.0 {
            return Err(AstroError::InvalidInput(format!(
                "DOP853 の刻みが小さくなりすぎました（t = {t}）"
            )));
        }
        let (mut step, err) = try_step(cfg, f, t, &y, &k0, h)?;
        let fac11 = err.powf(expo);
        if err <= 1.0 {
            let fac =
                (fac11 / facold.powf(BETA) / SAFETY).clamp(1.0 / GROW_LIMIT, 1.0 / SHRINK_LIMIT);
            let mut h_next = (h / fac).min(cfg.max_step);
            facold = err.max(1e-4);
            let t_new = if h == remaining { t_end } else { t + h };
            if idx < times.len() && times[idx] <= t_new {
                extend_for_dense(f, &mut step);
                while idx < times.len() && times[idx] <= t_new {
                    let ti = times[idx];
                    out.push(if ti >= t_new {
                        step.y1.clone()
                    } else {
                        step.interpolate(ti)
                    });
                    idx += 1;
                }
            }
            k0 = step.k[N_STAGES].clone();
            y = step.y1;
            t = t_new;
            if last_rejected && h_next > h {
                h_next = h;
            }
            last_rejected = false;
            rejections = 0;
            h = h_next;
        } else {
            rejections += 1;
            if rejections > MAX_REJECTIONS {
                return Err(AstroError::InvalidInput(format!(
                    "DOP853 が許容誤差を満たせません（t = {t}）"
                )));
            }
            last_rejected = true;
            h /= (1.0 / SHRINK_LIMIT).min(fac11 / SAFETY);
        }
    }
    Err(AstroError::InvalidInput(format!(
        "DOP853 の最大ステップ数 {} を超えました",
        cfg.max_steps
    )))
}

fn validate<S: OdeState>(cfg: &Dop853, t0: f64, x0: &S, times: &[f64]) -> Result<(), AstroError> {
    if !(cfg.rtol > 0.0 || cfg.atol > 0.0) || !cfg.rtol.is_finite() || !cfg.atol.is_finite() {
        return Err(AstroError::InvalidInput(
            "DOP853 の許容誤差が不正です".into(),
        ));
    }
    if cfg.max_step.is_nan() || cfg.max_step <= 0.0 {
        return Err(AstroError::InvalidInput(
            "DOP853 の最大刻みは正にしてください".into(),
        ));
    }
    if !t0.is_finite() || !is_finite(x0) || times.iter().any(|t| !t.is_finite()) {
        return Err(not_finite(t0));
    }
    if times.windows(2).any(|w| w[1] < w[0]) {
        return Err(AstroError::InvalidInput(
            "時刻列が昇順ではありません".into(),
        ));
    }
    if times.first().is_some_and(|&t| t < t0) {
        return Err(AstroError::InvalidInput(
            "開始時刻より前の時刻があります".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::Dop853;

    /// 二体問題（μ = 1、円軌道）: 1 周後に元の位置へ戻る。
    #[test]
    fn two_body_circular_returns_after_one_period() {
        let f = |_t: f64, x: &[f64; 4]| {
            let r3 = (x[0] * x[0] + x[1] * x[1]).powf(1.5);
            [x[2], x[3], -x[0] / r3, -x[1] / r3]
        };
        let tau = core::f64::consts::TAU;
        let times: Vec<f64> = (0..=40).map(|i| tau * i as f64 / 40.0).collect();
        let cfg = Dop853 {
            rtol: 1e-12,
            atol: 1e-12,
            ..Dop853::default()
        };
        let out = cfg
            .integrate_at(f, 0.0, [1.0, 0.0, 0.0, 1.0], &times)
            .expect("積分");
        for (t, x) in times.iter().zip(out.iter()) {
            let (s, c) = t.sin_cos();
            let e = ((x[0] - c).powi(2) + (x[1] - s).powi(2)).sqrt();
            assert!(e < 1e-9, "t={t}: 誤差 {e}");
        }
    }

    /// 密出力: 刻みの内側の値も厳密解に一致する（最大刻みを大きくしても補間で拾える）。
    #[test]
    fn dense_output_matches_exact_solution() {
        let f = |_t: f64, x: &Vec<f64>| vec![x[1], -x[0]];
        let times: Vec<f64> = (0..200).map(|i| 0.0371 * i as f64).collect();
        let out = Dop853::default()
            .integrate_at(f, 0.0, vec![0.0, 1.0], &times)
            .expect("積分");
        for (t, x) in times.iter().zip(out.iter()) {
            assert!(
                (x[0] - t.sin()).abs() < 1e-8,
                "t={t}: {} vs {}",
                x[0],
                t.sin()
            );
        }
    }

    #[test]
    fn rejects_unsorted_times() {
        let f = |_t: f64, x: &[f64; 1]| [x[0]];
        assert!(
            Dop853::default()
                .integrate_at(f, 0.0, [1.0], &[1.0, 0.5])
                .is_err()
        );
        assert!(
            Dop853::default()
                .integrate_at(f, 0.0, [1.0], &[-1.0])
                .is_err()
        );
        let at_start = Dop853::default()
            .integrate_at(f, 0.0, [1.0], &[0.0])
            .expect("t0");
        assert_eq!(at_start, vec![[1.0]]);
    }

    #[test]
    fn exponential_growth_relative_accuracy() {
        let f = |_t: f64, x: &[f64; 1]| [x[0]];
        let out = Dop853::default()
            .integrate_at(f, 0.0, [1.0], &[5.0])
            .expect("積分");
        assert!((out[0][0] / 5.0_f64.exp() - 1.0).abs() < 1e-9);
    }
}
