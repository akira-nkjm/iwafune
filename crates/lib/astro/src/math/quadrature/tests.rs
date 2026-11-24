//! ガウス・ルジャンドル則のテスト（既知の節点・重みと、多項式の厳密な積分）。

use super::*;

/// 2 点則: 節点 ±1/√3、重み 1。3 点則: 節点 0・±√(3/5)、重み 8/9・5/9。
#[test]
fn known_low_order_rules() {
    let two = gauss_legendre(2);
    let r = 1.0 / 3.0_f64.sqrt();
    assert!((two[0].0 + r).abs() < 1e-15 && (two[1].0 - r).abs() < 1e-15);
    assert!(two.iter().all(|(_, w)| (w - 1.0).abs() < 1e-15));
    let three = gauss_legendre(3);
    let s = (3.0_f64 / 5.0).sqrt();
    assert!((three[0].0 + s).abs() < 1e-15 && three[1].0.abs() < 1e-15);
    assert!((three[1].1 - 8.0 / 9.0).abs() < 1e-15 && (three[0].1 - 5.0 / 9.0).abs() < 1e-15);
    assert!(gauss_legendre(0).is_empty());
}

/// n 点則は 2n − 1 次までの多項式 x^p を厳密に積分する（∫₋₁¹ x^p dx = 2/(p + 1)（p が偶数）、0（奇数））。n = 1…12。
#[test]
fn exact_for_polynomials_up_to_degree_2n_minus_1() {
    for n in 1..=12 {
        let rule = gauss_legendre(n);
        for p in 0..=(2 * n - 1) {
            let got: f64 = rule.iter().map(|(x, w)| w * x.powi(p as i32)).sum();
            let want = if p % 2 == 0 {
                2.0 / (p as f64 + 1.0)
            } else {
                0.0
            };
            assert!(
                (got - want).abs() < 1e-14,
                "n = {n}, p = {p}: {got} vs {want}"
            );
        }
    }
}
