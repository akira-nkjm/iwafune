use super::*;
use crate::math::{Mat3, Vec3};
use std::f64::consts::PI;

/// 直交性 VᵀV = I と A V = V Λ の最大誤差（A の最大要素で正規化しない絶対誤差）。
fn residuals(a: &[f64], e: &SymEigen) -> (f64, f64) {
    let n = e.n;
    let mut orth = 0.0_f64;
    let mut res = 0.0_f64;
    for j in 0..n {
        let vj = e.vector(j);
        for i in 0..n {
            let vi = e.vector(i);
            let d: f64 = vi.iter().zip(&vj).map(|(x, y)| x * y).sum();
            orth = orth.max((d - if i == j { 1.0 } else { 0.0 }).abs());
        }
        for r in 0..n {
            let av: f64 = (0..n).map(|c| a[r * n + c] * vj[c]).sum();
            res = res.max((av - e.values[j] * vj[r]).abs());
        }
    }
    (orth, res)
}

fn second_difference(n: usize) -> Vec<f64> {
    let mut a = vec![0.0; n * n];
    for i in 0..n {
        a[i * n + i] = 2.0;
        if i > 0 {
            a[i * n + i - 1] = -1.0;
            a[(i - 1) * n + i] = -1.0;
        }
    }
    a
}

#[test]
fn diagonal_matrix_returns_sorted_entries() {
    let a = [3.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 2.0];
    let e = symmetric_eigen(&a, 3).expect("ok");
    assert_eq!(e.values, vec![-1.0, 2.0, 3.0]);
    assert_eq!(e.vector(0), vec![0.0, 1.0, 0.0]);
    assert_eq!(e.vector(2), vec![1.0, 0.0, 0.0]);
}

#[test]
fn two_by_two_has_known_vectors() {
    let e = symmetric_eigen(&[2.0, 1.0, 1.0, 2.0], 2).expect("ok");
    let s = 0.5_f64.sqrt();
    assert!((e.values[0] - 1.0).abs() < 1e-15 && (e.values[1] - 3.0).abs() < 1e-15);
    // λ = 1 のベクトルは (1, -1)/√2、λ = 3 は (1, 1)/√2（符号は絶対値最大の成分が正になる向き）
    for (g, w) in e.vector(0).iter().zip([s, -s]) {
        assert!((g - w).abs() < 1e-15, "{g} vs {w}");
    }
    for (g, w) in e.vector(1).iter().zip([s, s]) {
        assert!((g - w).abs() < 1e-15, "{g} vs {w}");
    }
}

#[test]
fn second_difference_matrix_matches_closed_form() {
    // 固有値 2 − 2cos(kπ/(n+1))（k = 1..n）
    for n in [5, 17, 40] {
        let a = second_difference(n);
        let e = symmetric_eigen(&a, n).expect("ok");
        let mut worst = 0.0_f64;
        for k in 1..=n {
            let want = 2.0 - 2.0 * (k as f64 * PI / (n + 1) as f64).cos();
            worst = worst.max((e.values[k - 1] - want).abs() / want);
        }
        assert!(worst < 1e-13, "n = {n}: worst relative error {worst:e}");
        let (orth, res) = residuals(&a, &e);
        // 丸め誤差の目安は n·ε ≈ 1e-14（n = 40 で実測 1.0e-14）なので 1e-13 を許容値にする
        assert!(orth < 1e-13 && res < 1e-13, "n = {n}: {orth:e} {res:e}");
    }
}

#[test]
fn repeated_eigenvalues_give_orthonormal_basis() {
    let n = 4;
    let mut id = vec![0.0; n * n];
    (0..n).for_each(|i| id[i * n + i] = 1.0);
    let e = symmetric_eigen(&id, n).expect("ok");
    assert_eq!(e.values, vec![1.0; 4]);
    let (orth, res) = residuals(&id, &e);
    assert!(orth < 1e-15 && res < 1e-15);
    // 2 重根: 3 I − (u uᵀ)·… として A = 5 I − 4 u uᵀ（u = (1,1,1,1)/2）。固有値 1 が 1 個、5 が 3 個。
    let u = [0.5; 4];
    let mut a = vec![0.0; 16];
    for i in 0..4 {
        for j in 0..4 {
            a[i * 4 + j] = if i == j { 5.0 } else { 0.0 } - 4.0 * u[i] * u[j];
        }
    }
    let e = symmetric_eigen(&a, 4).expect("ok");
    for (g, w) in e.values.iter().zip([1.0, 5.0, 5.0, 5.0]) {
        assert!((g - w).abs() < 1e-14, "{g} vs {w}");
    }
    let (orth, res) = residuals(&a, &e);
    assert!(orth < 1e-14 && res < 1e-14, "{orth:e} {res:e}");
}

#[test]
fn random_free_dense_matrix_reconstructs() {
    let n = 12;
    let a: Vec<f64> = (0..n * n)
        .map(|k| {
            let (i, j) = (k / n, k % n);
            let (lo, hi) = (i.min(j), i.max(j));
            ((lo * 31 + hi * 17) % 23) as f64 / 7.0 - 1.5
        })
        .collect();
    let e = symmetric_eigen(&a, n).expect("ok");
    let (orth, res) = residuals(&a, &e);
    assert!(orth < 1e-14 && res < 1e-13, "{orth:e} {res:e}");
    assert!(e.values.windows(2).all(|w| w[0] <= w[1]));
    // trace は固有値の和
    let tr: f64 = (0..n).map(|i| a[i * n + i]).sum();
    assert!((tr - e.values.iter().sum::<f64>()).abs() < 1e-12);
}

#[test]
fn agrees_with_mat3_symmetric_eigen() {
    let inertia = Mat3::from_rows(
        Vec3::new(0.30, 0.02, -0.01),
        Vec3::new(0.02, 0.25, 0.015),
        Vec3::new(-0.01, 0.015, 0.12),
    );
    let (w, v) = inertia.symmetric_eigen();
    let a = [0.30, 0.02, -0.01, 0.02, 0.25, 0.015, -0.01, 0.015, 0.12];
    let e = symmetric_eigen(&a, 3).expect("ok");
    for (g, want) in e.values.iter().zip(w.to_array()) {
        assert!((g - want).abs() < 1e-14, "{g} vs {want}");
    }
    for j in 0..3 {
        let dot: f64 = e
            .vector(j)
            .iter()
            .zip(v.column(j).to_array())
            .map(|(x, y)| x * y)
            .sum();
        assert!((dot.abs() - 1.0).abs() < 1e-12, "vector {j}: {dot}");
    }
}

#[test]
fn generalized_problem_with_diagonal_and_full_b() {
    // B = diag(m) のとき A x = λ B x は B^{-1/2} A B^{-1/2} の固有値。2 質点の固定‐固定ばね（k = 1, m1 = 1, m2 = 4）:
    // det([[2-λ, -1], [-1, 2-4λ]]) = 4λ² − 10λ + 3 = 0 → λ = (5 ∓ √13)/4
    let e =
        generalized_symmetric_eigen(&[2.0, -1.0, -1.0, 2.0], &[1.0, 0.0, 0.0, 4.0], 2).expect("ok");
    let want = [(5.0 - 13.0_f64.sqrt()) / 4.0, (5.0 + 13.0_f64.sqrt()) / 4.0];
    for (g, w) in e.values.iter().zip(want) {
        assert!((g - w).abs() < 1e-14, "{g} vs {w}");
    }
    // 一般の B: 固有ベクトルは Xᵀ B X = I、A X = B X Λ
    let n = 6;
    let a = second_difference(n);
    let mut b = vec![0.0; n * n];
    for i in 0..n {
        b[i * n + i] = 4.0 + i as f64;
        if i > 0 {
            b[i * n + i - 1] = 0.5;
            b[(i - 1) * n + i] = 0.5;
        }
    }
    let e = generalized_symmetric_eigen(&a, &b, n).expect("ok");
    for j in 0..n {
        let xj = e.vector(j);
        for i in 0..n {
            let xi = e.vector(i);
            let bx: f64 = (0..n)
                .map(|r| xi[r] * (0..n).map(|c| b[r * n + c] * xj[c]).sum::<f64>())
                .sum();
            assert!(
                (bx - if i == j { 1.0 } else { 0.0 }).abs() < 1e-13,
                "{i} {j} {bx}"
            );
        }
        for r in 0..n {
            let ax: f64 = (0..n).map(|c| a[r * n + c] * xj[c]).sum();
            let bx: f64 = (0..n).map(|c| b[r * n + c] * xj[c]).sum();
            assert!((ax - e.values[j] * bx).abs() < 1e-13);
        }
    }
}

#[test]
fn rejects_bad_input_without_panicking() {
    assert!(matches!(
        symmetric_eigen(&[1.0, 2.0, 3.0], 2),
        Err(SymEigenError::SizeMismatch { .. })
    ));
    assert_eq!(
        symmetric_eigen(&[f64::NAN], 1),
        Err(SymEigenError::NotFinite)
    );
    assert!(matches!(
        generalized_symmetric_eigen(&[1.0, 0.0, 0.0, 1.0], &[1.0, 2.0, 2.0, 1.0], 2),
        Err(SymEigenError::NotPositiveDefinite { index: 1, .. })
    ));
    let e = symmetric_eigen(&[], 0).expect("empty");
    assert!(e.values.is_empty());
}
