//! 小さな密対称行列の固有値分解（巡回 Jacobi 法）と、対称定値の一般化固有値問題。
//!
//! 部分空間反復の射影問題（次数 40 程度まで）に使う。行列は n×n の行優先 `&[f64]`。
//! 3×3 の慣性テンソル専用の [`crate::math::Mat3::symmetric_eigen`] とは別物（そちらは変えない）。

use thiserror::Error;

/// 対称固有値問題の失敗。
#[derive(Debug, Clone, PartialEq, Error)]
pub enum SymEigenError {
    /// 配列の長さが n×n と合わない。
    #[error("Matrix size mismatch: expected {expected} entries, got {got}")]
    SizeMismatch { expected: usize, got: usize },
    /// 有限でない値を含む。
    #[error("The matrix contains a non-finite value")]
    NotFinite,
    /// Jacobi 法が打ち切り回数内に収束しなかった。
    #[error("Jacobi iteration did not converge in {sweeps} sweeps")]
    NoConvergence { sweeps: usize },
    /// B が正定値でない（Cholesky 分解のピボットが正でない）。
    #[error("The matrix B is not positive definite (pivot {pivot:e} at row {index})")]
    NotPositiveDefinite { index: usize, pivot: f64 },
}

/// 固有値（昇順）と対応する固有ベクトル。
#[derive(Debug, Clone, PartialEq)]
pub struct SymEigen {
    /// 行列の次数。
    pub n: usize,
    /// 固有値（昇順）。
    pub values: Vec<f64>,
    /// 固有ベクトルを列に持つ n×n（行優先）。列 j が `values[j]` に対応する。
    pub vectors: Vec<f64>,
}

impl SymEigen {
    /// j 番目の固有ベクトル（長さ n）。
    pub fn vector(&self, j: usize) -> Vec<f64> {
        (0..self.n).map(|i| self.vectors[i * self.n + j]).collect()
    }
}

const MAX_SWEEPS: usize = 100;

fn check(a: &[f64], n: usize) -> Result<(), SymEigenError> {
    if a.len() != n * n {
        return Err(SymEigenError::SizeMismatch {
            expected: n * n,
            got: a.len(),
        });
    }
    if !a.iter().all(|v| v.is_finite()) {
        return Err(SymEigenError::NotFinite);
    }
    Ok(())
}

/// 対称行列 `a`（n×n、行優先）の固有値分解。上三角と下三角の平均を使う。
///
/// 固有ベクトルは正規直交で、各ベクトルは絶対値最大の成分が正になる向きに揃える（再現性のため）。
pub fn symmetric_eigen(a: &[f64], n: usize) -> Result<SymEigen, SymEigenError> {
    check(a, n)?;
    let mut m = a.to_vec();
    for i in 0..n {
        for j in (i + 1)..n {
            let s = 0.5 * (m[i * n + j] + m[j * n + i]);
            m[i * n + j] = s;
            m[j * n + i] = s;
        }
    }
    let mut v = vec![0.0; n * n];
    (0..n).for_each(|i| v[i * n + i] = 1.0);
    let total: f64 = m.iter().map(|x| x * x).sum();
    let mut converged = total == 0.0;
    for _ in 0..MAX_SWEEPS {
        if converged {
            break;
        }
        let off: f64 = (0..n)
            .flat_map(|i| ((i + 1)..n).map(move |j| (i, j)))
            .map(|(i, j)| 2.0 * m[i * n + j] * m[i * n + j])
            .sum();
        if off <= 1e-32 * total {
            converged = true;
            break;
        }
        for p in 0..n {
            for q in (p + 1)..n {
                rotate(&mut m, &mut v, n, p, q);
            }
        }
    }
    if !converged {
        return Err(SymEigenError::NoConvergence { sweeps: MAX_SWEEPS });
    }
    Ok(sorted(&m, &v, n))
}

/// Jacobi 回転で (p, q) 成分を消す（m と固有ベクトルの積 v を更新）。
fn rotate(m: &mut [f64], v: &mut [f64], n: usize, p: usize, q: usize) {
    let apq = m[p * n + q];
    if apq == 0.0 {
        return;
    }
    let theta = (m[q * n + q] - m[p * n + p]) / (2.0 * apq);
    let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
    let c = 1.0 / (t * t + 1.0).sqrt();
    let s = t * c;
    for k in 0..n {
        let (akp, akq) = (m[k * n + p], m[k * n + q]);
        m[k * n + p] = c * akp - s * akq;
        m[k * n + q] = s * akp + c * akq;
    }
    for k in 0..n {
        let (apk, aqk) = (m[p * n + k], m[q * n + k]);
        m[p * n + k] = c * apk - s * aqk;
        m[q * n + k] = s * apk + c * aqk;
    }
    for k in 0..n {
        let (vkp, vkq) = (v[k * n + p], v[k * n + q]);
        v[k * n + p] = c * vkp - s * vkq;
        v[k * n + q] = s * vkp + c * vkq;
    }
}

fn sorted(m: &[f64], v: &[f64], n: usize) -> SymEigen {
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&i, &j| m[i * n + i].total_cmp(&m[j * n + j]));
    let values = order.iter().map(|&j| m[j * n + j]).collect();
    let mut vectors = vec![0.0; n * n];
    for (new, &old) in order.iter().enumerate() {
        let col: Vec<f64> = (0..n).map(|i| v[i * n + old]).collect();
        let big = col
            .iter()
            .fold(0.0_f64, |b, x| if x.abs() > b.abs() { *x } else { b });
        let sign = if big < 0.0 { -1.0 } else { 1.0 };
        for i in 0..n {
            vectors[i * n + new] = sign * col[i];
        }
    }
    SymEigen { n, values, vectors }
}

/// 一般化固有値問題 `A x = λ B x`（A 対称、B 対称正定値）。B の Cholesky 分解で標準形に直して解く。
///
/// 固有ベクトルは B 正規直交（`Xᵀ B X = I`）。
pub fn generalized_symmetric_eigen(
    a: &[f64],
    b: &[f64],
    n: usize,
) -> Result<SymEigen, SymEigenError> {
    check(a, n)?;
    check(b, n)?;
    let l = cholesky(b, n)?;
    // C = L⁻¹ A L⁻ᵀ
    let mut c = a.to_vec();
    for col in 0..n {
        forward_column(&l, n, &mut c, col, true);
    }
    for row in 0..n {
        forward_column(&l, n, &mut c, row, false);
    }
    let mut eig = symmetric_eigen(&c, n)?;
    // x = L⁻ᵀ y
    for j in 0..n {
        let mut y = eig.vector(j);
        for i in (0..n).rev() {
            let s: f64 = ((i + 1)..n).map(|k| l[k * n + i] * y[k]).sum();
            y[i] = (y[i] - s) / l[i * n + i];
        }
        for i in 0..n {
            eig.vectors[i * n + j] = y[i];
        }
    }
    Ok(eig)
}

/// 行列 `c`（n×n 行優先）の列（`by_col`）または行に L⁻¹ を左から作用させる前進代入。
fn forward_column(l: &[f64], n: usize, c: &mut [f64], idx: usize, by_col: bool) {
    let at = |k: usize| if by_col { k * n + idx } else { idx * n + k };
    for i in 0..n {
        let s: f64 = (0..i).map(|k| l[i * n + k] * c[at(k)]).sum();
        c[at(i)] = (c[at(i)] - s) / l[i * n + i];
    }
}

fn cholesky(b: &[f64], n: usize) -> Result<Vec<f64>, SymEigenError> {
    let mut l = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..=i {
            let s: f64 = (0..j).map(|k| l[i * n + k] * l[j * n + k]).sum();
            let v = 0.5 * (b[i * n + j] + b[j * n + i]) - s;
            if i == j {
                if v <= 0.0 || !v.is_finite() {
                    return Err(SymEigenError::NotPositiveDefinite { index: i, pivot: v });
                }
                l[i * n + i] = v.sqrt();
            } else {
                l[i * n + j] = v / l[j * n + j];
            }
        }
    }
    Ok(l)
}

#[cfg(test)]
mod tests;
