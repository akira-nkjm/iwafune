//! 小さな密行列の連立一次方程式（熱ネットワークの定常解などに使う）。

/// `A x = b` を部分ピボット付きの Gauss 消去で解く。`a` は n×n の行優先、`b` は長さ n。
///
/// 大きさが合わない・特異（ピボットが行列の最大要素 × 1e-14 以下）・有限でない値を含むときは `None`。
pub fn solve_dense(a: Vec<f64>, b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    if a.len() != n * n || !a.iter().chain(b.iter()).all(|v| v.is_finite()) {
        return None;
    }
    let (mut a, mut b) = (a, b);
    let scale = a.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
    let tol = if scale > 0.0 {
        scale * 1e-14
    } else {
        f64::MIN_POSITIVE
    };
    for col in 0..n {
        let (pivot_row, pivot_abs) =
            (col..n)
                .map(|r| (r, a[r * n + col].abs()))
                .fold(
                    (col, -1.0),
                    |best, cur| if cur.1 > best.1 { cur } else { best },
                );
        if pivot_abs <= tol {
            return None;
        }
        if pivot_row != col {
            for k in 0..n {
                a.swap(col * n + k, pivot_row * n + k);
            }
            b.swap(col, pivot_row);
        }
        let pivot = a[col * n + col];
        for r in (col + 1)..n {
            let factor = a[r * n + col] / pivot;
            if factor == 0.0 {
                continue;
            }
            for k in col..n {
                a[r * n + k] -= factor * a[col * n + k];
            }
            b[r] -= factor * b[col];
        }
    }
    let mut x = vec![0.0; n];
    for r in (0..n).rev() {
        let sum: f64 = ((r + 1)..n).map(|k| a[r * n + k] * x[k]).sum();
        x[r] = (b[r] - sum) / a[r * n + r];
    }
    x.iter().all(|v| v.is_finite()).then_some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_small_system_with_pivoting() {
        // 先頭のピボットが 0 なので行の入れ替えが要る
        let a = vec![0.0, 2.0, 1.0, 1.0, 1.0, 1.0, 2.0, 1.0, 3.0];
        let x_true = [1.0, -2.0, 3.0];
        let b: Vec<f64> = (0..3)
            .map(|r| (0..3).map(|c| a[r * 3 + c] * x_true[c]).sum())
            .collect();
        let x = solve_dense(a, b).expect("regular");
        for (got, want) in x.iter().zip(x_true.iter()) {
            assert!((got - want).abs() < 1e-12, "{got} vs {want}");
        }
    }

    #[test]
    fn rejects_singular_and_mismatched() {
        assert!(solve_dense(vec![1.0, 2.0, 2.0, 4.0], vec![1.0, 2.0]).is_none());
        assert!(solve_dense(vec![1.0, 2.0, 3.0], vec![1.0, 2.0]).is_none());
        assert!(solve_dense(vec![f64::NAN], vec![1.0]).is_none());
        assert_eq!(solve_dense(vec![], vec![]), Some(vec![]));
    }

    #[test]
    fn larger_diagonally_dominant_system() {
        // 熱ネットワークに似た 20 節点の三重対角
        let n = 20;
        let mut a = vec![0.0; n * n];
        for i in 0..n {
            a[i * n + i] = 2.5;
            if i > 0 {
                a[i * n + i - 1] = -1.0;
            }
            if i + 1 < n {
                a[i * n + i + 1] = -1.0;
            }
        }
        let x_true: Vec<f64> = (0..n).map(|i| i as f64 * 0.5 - 3.0).collect();
        let b: Vec<f64> = (0..n)
            .map(|r| (0..n).map(|c| a[r * n + c] * x_true[c]).sum())
            .collect();
        let x = solve_dense(a, b).expect("regular");
        assert!(
            x.iter()
                .zip(x_true.iter())
                .all(|(g, w)| (g - w).abs() < 1e-10)
        );
    }
}
