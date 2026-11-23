//! 一般化対称固有値問題 `K x = λ M x` の低次側の固有対を、シフト付き部分空間反復で求める。
//!
//! K は対称半正定値（拘束不足なら特異）、M は対称正定値（対角の集中質量か、小さなブロックを持つ一般の対称行列）。
//! (K + σM) x = μ M x を逆反復し、λ = μ − σ とする。σ は K + σM が正定値になる値（σ > −λ_min）を呼び出し側が選ぶ。
//! 開始部分空間は固定シードの疑似乱数で作るので、結果はビット単位で再現する。
//! 最後に Sturm 列（K − ρM の負のピボット数）で、返した最大固有値より下の固有値の取りこぼしがないか確かめる。

mod mass;
mod sturm;

pub use mass::Mass;

use super::skyline::{Skyline, SkylineError};
use super::sym_eigen::{SymEigenError, generalized_symmetric_eigen};
use thiserror::Error;

/// 部分空間反復の失敗。
#[derive(Debug, Clone, PartialEq, Error)]
pub enum SubspaceError {
    /// 求める個数が 0、または次数より多い。
    #[error("Invalid mode count {count} for a problem of size {n}")]
    InvalidCount { count: usize, n: usize },
    /// K と M の大きさが合わない。
    #[error("K has size {k} but M has size {m}")]
    SizeMismatch { k: usize, m: usize },
    /// K + σM が分解できない（シフトが小さすぎる、K か M が不正）。
    #[error("Cannot factor K + shift·M (shift too small or invalid matrices): {0}")]
    Factor(SkylineError),
    /// 質量行列の組み立て・積でプロファイルが合わない。
    #[error("Mass matrix does not fit the stiffness profile: {0}")]
    Mass(SkylineError),
    /// 射影した小さな問題が解けない（部分空間が縮退した）。
    #[error("Projected eigenproblem failed: {0}")]
    Projected(SymEigenError),
    /// 反復の上限に達しても収束しなかった。
    #[error(
        "No convergence after {iterations} iterations (largest relative change {change:e}, {converged} of {count} modes below the tolerance)"
    )]
    NotConverged {
        iterations: usize,
        change: f64,
        converged: usize,
        count: usize,
    },
    /// 返した最大固有値より下に、取りこぼした固有値がある（Sturm 列の判定）。
    #[error(
        "Sturm check failed: {below} eigenvalues lie below the largest returned one, but only {returned_below} were returned"
    )]
    MissedModes { below: usize, returned_below: usize },
    /// Sturm 列の判定の途中で K − ρM の分解に失敗した。
    #[error("Sturm check could not factor K - rho*M: {0}")]
    SturmFactor(SkylineError),
}

/// 部分空間反復の設定。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SubspaceOptions {
    /// 固有値 μ の反復間の相対変化の許容値（求める個数すべてがこれ以下で収束）。
    /// 固有ベクトルの残差は固有値の誤差の平方根程度に遅れて縮むので、ベクトルまで厳しく求めるなら 1e-14 まで下げる
    /// （丸め誤差の床に近いので、条件の悪い問題では収束しないことがある）。
    pub tolerance: f64,
    /// 反復の上限回数。
    pub max_iterations: usize,
    /// 反復ベクトルの数。`None` は `2·count + 8`（次数で頭打ち）。個数以上、次数以下に丸める。
    pub vectors: Option<usize>,
    /// Sturm 列の判定をするか。
    pub check_missed: bool,
    /// Sturm 列の判定で同じ固有値とみなす幅（最大固有値の大きさ + σ に対する相対値）。
    pub sturm_gap: f64,
}

impl Default for SubspaceOptions {
    fn default() -> Self {
        Self {
            tolerance: 1e-12,
            max_iterations: 200,
            vectors: None,
            check_missed: true,
            sturm_gap: 1e-6,
        }
    }
}

/// 求めた固有対。
#[derive(Debug, Clone, PartialEq)]
pub struct Modes {
    /// 固有値 λ（昇順）。
    pub values: Vec<f64>,
    /// M 正規直交（`xᵢᵀ M xⱼ = δᵢⱼ`）の固有ベクトル。各ベクトルは絶対値最大の成分が正。
    pub vectors: Vec<Vec<f64>>,
    /// 反復の回数。
    pub iterations: usize,
    /// 最大固有値の近傍に、返した個数を超える固有値が続く（同じ固有値が打ち切り位置をまたぐ）。
    /// その場合、最後の固有ベクトルは重複固有空間の中の 1 つの基底にすぎない。
    pub cluster_split: bool,
}

/// `K x = λ M x` の低い側から `count` 個の固有値と固有ベクトルを求める。
pub fn lowest_modes(
    k: &Skyline,
    m: &Mass<'_>,
    count: usize,
    shift: f64,
    options: &SubspaceOptions,
) -> Result<Modes, SubspaceError> {
    let n = k.dim();
    if m.dim() != n {
        return Err(SubspaceError::SizeMismatch { k: n, m: m.dim() });
    }
    if count == 0 || count > n || !shift.is_finite() {
        return Err(SubspaceError::InvalidCount { count, n });
    }
    let p = options.vectors.unwrap_or(2 * count + 8).clamp(count, n);
    let ldl = shifted(k, m, shift)?
        .into_ldl()
        .map_err(SubspaceError::Factor)?;

    let mut x = start_vectors(n, p);
    let mut y = mass_columns(m, &x)?;
    let mut mu_prev: Option<Vec<f64>> = None;
    let mut last_change = f64::INFINITY;
    let mut last_converged = 0;
    for iteration in 1..=options.max_iterations {
        let Step { mu, x: xn, y: yn } = iterate(&ldl, m, &y)?;
        (last_change, last_converged) = change(mu_prev.as_deref(), &mu, count, options.tolerance);
        x = xn;
        y = yn;
        if last_converged == count {
            let values: Vec<f64> = mu[..count].iter().map(|v| v - shift).collect();
            return finish(k, m, shift, values, x, iteration, options);
        }
        mu_prev = Some(mu);
    }
    Err(SubspaceError::NotConverged {
        iterations: options.max_iterations,
        change: last_change,
        converged: last_converged,
        count,
    })
}

fn finish(
    k: &Skyline,
    m: &Mass<'_>,
    shift: f64,
    values: Vec<f64>,
    x: Vec<Vec<f64>>,
    iterations: usize,
    options: &SubspaceOptions,
) -> Result<Modes, SubspaceError> {
    let count = values.len();
    let mut vectors: Vec<Vec<f64>> = x.into_iter().take(count).collect();
    vectors.iter_mut().for_each(|v| orient(v));
    let mut cluster_split = false;
    if options.check_missed {
        let top = values[count - 1];
        let gap = options.sturm_gap * (top.abs() + shift.abs());
        let verdict = sturm::check(k, m, &values, top, gap)?;
        cluster_split = verdict.cluster_split;
    }
    Ok(Modes {
        values,
        vectors,
        iterations,
        cluster_split,
    })
}

/// K + σM。
fn shifted(k: &Skyline, m: &Mass<'_>, shift: f64) -> Result<Skyline, SubspaceError> {
    let mut a = k.clone();
    m.add_scaled_into(&mut a, shift)?;
    Ok(a)
}

/// 1 回の反復の結果。
struct Step {
    /// 射影した問題の固有値 μ（昇順）。
    mu: Vec<f64>,
    /// 新しい反復ベクトル（M 正規直交）。
    x: Vec<Vec<f64>>,
    /// M · x。
    y: Vec<Vec<f64>>,
}

/// 1 回の反復。Y = M X から X̂ = A⁻¹ Y、射影した問題を解き、(μ, 新しい X, 新しい Y = M X) を返す。
fn iterate(
    ldl: &super::skyline::SkylineLdl,
    m: &Mass<'_>,
    y: &[Vec<f64>],
) -> Result<Step, SubspaceError> {
    let p = y.len();
    let mut xh = y.to_vec();
    ldl.solve_many(&mut xh).map_err(SubspaceError::Factor)?;
    let w = mass_columns(m, &xh)?;
    let (mut a_bar, mut m_bar) = (vec![0.0; p * p], vec![0.0; p * p]);
    for i in 0..p {
        for j in 0..p {
            a_bar[i * p + j] = dot(&xh[i], &y[j]);
            m_bar[i * p + j] = dot(&xh[i], &w[j]);
        }
    }
    let eig = generalized_symmetric_eigen(&a_bar, &m_bar, p).map_err(SubspaceError::Projected)?;
    let x = combine(&xh, &eig.vectors, p);
    let yn = combine(&w, &eig.vectors, p);
    Ok(Step {
        mu: eig.values,
        x,
        y: yn,
    })
}

/// 列ベクトルの組 `cols`（p 本）に、p×p の係数 `q`（行優先）を右から掛けた組 `cols · q`。
fn combine(cols: &[Vec<f64>], q: &[f64], p: usize) -> Vec<Vec<f64>> {
    (0..p)
        .map(|j| {
            let mut out = vec![0.0; cols[0].len()];
            for (i, c) in cols.iter().enumerate() {
                let f = q[i * p + j];
                out.iter_mut().zip(c).for_each(|(o, v)| *o += f * v);
            }
            out
        })
        .collect()
}

fn mass_columns(m: &Mass<'_>, cols: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, SubspaceError> {
    cols.iter()
        .map(|c| {
            let mut out = vec![0.0; c.len()];
            m.mul_vec(c, &mut out)?;
            Ok(out)
        })
        .collect()
}

/// 反復間の μ の最大相対変化と、許容値以下に収まった個数（先頭 `count` 個）。
fn change(prev: Option<&[f64]>, mu: &[f64], count: usize, tol: f64) -> (f64, usize) {
    let Some(prev) = prev else {
        return (f64::INFINITY, 0);
    };
    let rel = |i: usize| (mu[i] - prev[i]).abs() / mu[i].abs().max(f64::MIN_POSITIVE);
    let worst = (0..count).map(rel).fold(0.0_f64, f64::max);
    (worst, (0..count).filter(|&i| rel(i) <= tol).count())
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// 絶対値最大の成分（同値なら先の成分）が正になる向きに揃える。
fn orient(v: &mut [f64]) {
    let big = v
        .iter()
        .fold(0.0_f64, |b, x| if x.abs() > b.abs() { *x } else { b });
    if big < 0.0 {
        v.iter_mut().for_each(|x| *x = -*x);
    }
}

/// 固定シードの splitmix64 から作る [-1, 1) の開始ベクトル（p 本、各長さ n）。
fn start_vectors(n: usize, p: usize) -> Vec<Vec<f64>> {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
    };
    (0..p).map(|_| (0..n).map(|_| next()).collect()).collect()
}

#[cfg(test)]
mod tests;
