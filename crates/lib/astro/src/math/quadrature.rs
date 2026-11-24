//! 数値積分の求積則。**ガウス・ルジャンドル則の節点と重みはここだけ**。
//!
//! `n` 点則は区間 `[-1, 1]` で `n` 次ルジャンドル多項式 `P_n` の零点を節点にし、`2n − 1` 次までの多項式を厳密に積分する。
//! 節点は `P_n` の零点で、Chebyshev 点 `cos(π(k − 1/4) / (n + 1/2))` を初期値にした Newton 法で求める（`P_n` と導関数は 3 項漸化式）。
//! 重みは `2 / ((1 − x²) P_n'(x)²)`。

use std::f64::consts::PI;

/// Newton 法の反復の上限。初期値が十分近いので 3〜4 回で丸め誤差に達する。
const MAX_ITERATIONS: usize = 100;
/// Newton 法の収束判定（節点の更新量）。
const TOLERANCE: f64 = 1e-15;

/// `n` 点のガウス・ルジャンドル則の `(節点, 重み)`（区間 `[-1, 1]`、節点は昇順、重みの和は 2）。`n = 0` は空。
pub fn gauss_legendre(n: usize) -> Vec<(f64, f64)> {
    let mut nodes: Vec<(f64, f64)> = (1..=n)
        .map(|k| {
            let mut x = (PI * (k as f64 - 0.25) / (n as f64 + 0.5)).cos();
            for _ in 0..MAX_ITERATIONS {
                let (p, dp) = legendre(n, x);
                let step = p / dp;
                x -= step;
                if step.abs() < TOLERANCE {
                    break;
                }
            }
            let (_, dp) = legendre(n, x);
            (x, 2.0 / ((1.0 - x * x) * dp * dp))
        })
        .collect();
    nodes.sort_by(|a, b| a.0.total_cmp(&b.0));
    nodes
}

/// `P_n(x)` とその導関数（3 項漸化式 `k P_k = (2k − 1) x P_{k−1} − (k − 1) P_{k−2}`）。
fn legendre(n: usize, x: f64) -> (f64, f64) {
    let (mut p0, mut p1) = (1.0, x);
    if n == 0 {
        return (1.0, 0.0);
    }
    for k in 2..=n {
        let kf = k as f64;
        (p0, p1) = (p1, ((2.0 * kf - 1.0) * x * p1 - (kf - 1.0) * p0) / kf);
    }
    // P_n' = n (x P_n − P_{n−1}) / (x² − 1)
    (p1, n as f64 * (x * p1 - p0) / (x * x - 1.0))
}

#[cfg(test)]
mod tests;
