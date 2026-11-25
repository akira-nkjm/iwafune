//! McIlwain の関数 F（`L³B/M = F(I³B/M)`）を、定義どおり中心双極子の積分から数値で求める。
//!
//! 文献の近似式（Hilton の多項式）は定数を読める資料が手元になかったので使わず、F の定義そのものを解く。
//! 中心双極子で緯度 λ の磁力線（`r = L cos²λ`）の点について
//!
//! - `L³B/M = b(λ) = √(1 + 3 sin²λ) / cos⁶λ`
//! - `I = L·J(λ)`、`J(λ) = ∫₋λ^λ cos λ′ √(1 + 3 sin²λ′) √(1 − b(λ′)/b(λ)) dλ′`（ミラー点 ±λ の間の磁力線に沿う積分）
//! - よって `I³B/M = J(λ)³ b(λ)`
//!
//! なので、`v = (I³B/M)^(1/3) = J(λ) b(λ)^(1/3)` に対して `F = b(λ)` が答えになる（`J b^(1/3)` は λ について単調増加）。
//! `v` から λ を二分法で解き、`J` は両端の √ の特異性を Gauss–Chebyshev（第 2 種）の求積で厳密に扱う。
//! `v → 0` の展開 `F = 1 + (2√4.5/π) v + O(v²)` は同じ式から解析的に出る（λ → 0 で `b ≈ 1 + 4.5λ²`、`J ≈ √4.5 π λ²/2`）。

use crate::math::bisect;
use core::f64::consts::{FRAC_PI_2, PI};

/// `J` の求積の節点数（テストで 128 点との差を確認している）。
const NODES: usize = 48;

/// 展開の 1 次の係数 `2√4.5/π`。
pub(super) const SLOPE_AT_ZERO: f64 = 1.350_474_4;

/// `v` がこれ未満なら 1 次の展開を使う（打ち切りの誤差は `0.47 v²` 程度で 1e-10 未満。それより下は 1 − B/Bm の桁落ちが出る）。
const SERIES_BELOW: f64 = 1e-5;

/// λ の上限 [rad]（`cos λ = 1e-6`。F がここで超える値より大きい v は範囲外）。
const LAMBDA_MAX: f64 = FRAC_PI_2 - 1e-6;

fn b_of(lam: f64) -> f64 {
    let (s, c) = lam.sin_cos();
    (1.0 + 3.0 * s * s).sqrt() / c.powi(6)
}

/// `J(λ)`。
fn j_of(lam: f64) -> f64 {
    let b_m = b_of(lam);
    let mut sum = 0.0;
    for i in 1..=NODES {
        let theta = i as f64 * PI / (NODES + 1) as f64;
        let l = lam * theta.cos();
        let (s, c) = l.sin_cos();
        let f = c * (1.0 + 3.0 * s * s).sqrt() * (1.0 - b_of(l) / b_m).max(0.0).sqrt();
        sum += theta.sin() * f;
    }
    lam * PI / (NODES + 1) as f64 * sum
}

/// `v = J b^(1/3)`。
fn v_of(lam: f64) -> f64 {
    j_of(lam) * b_of(lam).cbrt()
}

/// `v = (I³B/M)^(1/3)` から `F = L³B/M` を返す。`v` が負・非有限、または表せる範囲（`cos λ = 1e-6` の磁力線）を超えれば `None`。
pub(super) fn dipole_f(v: f64) -> Option<f64> {
    if !(v >= 0.0 && v.is_finite()) {
        return None;
    }
    if v < SERIES_BELOW {
        return Some(1.0 + SLOPE_AT_ZERO * v);
    }
    // v ≥ 3.3 λ²（λ が大きいほど b^(1/3) の分だけ v が増える）なので、λ ≤ √(v/3.3) の少し上が上端になる。
    let mut hi = (1.2 * (v / 3.3).sqrt()).min(LAMBDA_MAX);
    if v_of(hi) < v {
        hi = LAMBDA_MAX;
        if v_of(hi) < v {
            return None;
        }
    }
    let lo = 1e-6;
    let lam = bisect(|l| v_of(l) - v, lo, hi, 1e-13, 200)?;
    Some(b_of(lam))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 求積の節点数を増やしても J は変わらない（打ち切りの誤差が 1e-10 未満）。
    #[test]
    fn quadrature_is_converged() {
        let dense = |lam: f64, n: usize| {
            let b_m = b_of(lam);
            let mut sum = 0.0;
            for i in 1..=n {
                let theta = i as f64 * PI / (n + 1) as f64;
                let l = lam * theta.cos();
                let (s, c) = l.sin_cos();
                let f = c * (1.0 + 3.0 * s * s).sqrt() * (1.0 - b_of(l) / b_m).max(0.0).sqrt();
                sum += theta.sin() * f;
            }
            lam * PI / (n + 1) as f64 * sum
        };
        for deg in [2.0_f64, 20.0, 45.0, 70.0, 80.0, 85.0] {
            let lam = deg.to_radians();
            let (a, b) = (j_of(lam), dense(lam, 128));
            assert!((a / b - 1.0).abs() < 1e-10, "{deg}: {a} vs {b}");
        }
    }

    /// 原点での傾きは解析値 `2√4.5/π`。
    #[test]
    fn slope_at_zero_is_analytic() {
        let exact = 2.0 * 4.5_f64.sqrt() / PI;
        assert!((SLOPE_AT_ZERO - exact).abs() < 1e-7);
        // 級数の下端の少し上で、数値解が展開と 0.5 v² 程度の差で合う。
        let v = 2e-5;
        let f = dipole_f(v).expect("in range");
        assert!((f - (1.0 + exact * v)).abs() < 1.0 * v * v, "{f}");
    }

    /// 級数と数値解は境界でつながる。
    #[test]
    fn continuous_at_series_boundary() {
        let lo = dipole_f(SERIES_BELOW * 0.999_999).expect("series");
        let hi = dipole_f(SERIES_BELOW * 1.000_001).expect("numeric");
        assert!((hi - lo).abs() < 1e-9, "{lo} {hi}");
    }

    #[test]
    fn rejects_out_of_range() {
        assert!(dipole_f(-1.0).is_none());
        assert!(dipole_f(f64::NAN).is_none());
        assert!(dipole_f(1e30).is_none());
    }
}
