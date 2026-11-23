use super::*;
use crate::math::generalized_symmetric_eigen;
use std::f64::consts::PI;

/// 端の条件。
#[derive(Clone, Copy, PartialEq)]
enum Ends {
    FixedFixed,
    FixedFree,
    FreeFree,
}

/// n 個の質点（質量 m）を n−1 本の内部ばね（ばね定数 k）でつないだ列と、端の条件による端のばね。
/// 固定端は壁へのばね 1 本（FixedFixed は両端、FixedFree は左端だけ）。自由度は 0..n。
fn chain_k(n: usize, k: f64, ends: Ends) -> Skyline {
    let mut s = Skyline::from_couplings(n, (1..n).map(|i| (i - 1, i))).expect("profile");
    let mut spring = |a: Option<usize>, b: Option<usize>| {
        for (i, j, sign) in [(a, a, 1.0), (b, b, 1.0), (a, b, -1.0)] {
            if let (Some(i), Some(j)) = (i, j) {
                s.add(i, j, sign * k).expect("inside");
            }
        }
    };
    (1..n).for_each(|i| spring(Some(i - 1), Some(i)));
    if ends != Ends::FreeFree {
        spring(None, Some(0));
    }
    if ends == Ends::FixedFixed {
        spring(Some(n - 1), None);
    }
    s
}

/// 相対誤差の最大値。
fn worst_rel(got: &[f64], want: &[f64]) -> f64 {
    got.iter()
        .zip(want)
        .map(|(g, w)| (g - w).abs() / w.abs())
        .fold(0.0, f64::max)
}

/// M 正規直交性（最大誤差）と K x = λ M x の残差（‖Kx − λMx‖∞ を ‖Mx‖∞ と最大固有値の尺度 `scale` で割った値）。
fn quality(k: &Skyline, m: &Mass<'_>, modes: &Modes, scale: f64) -> (f64, f64) {
    let n = k.dim();
    let mx: Vec<Vec<f64>> = modes
        .vectors
        .iter()
        .map(|x| {
            let mut y = vec![0.0; n];
            m.mul_vec(x, &mut y).expect("sizes");
            y
        })
        .collect();
    let mut orth = 0.0_f64;
    for (i, xi) in modes.vectors.iter().enumerate() {
        for (j, mxj) in mx.iter().enumerate() {
            let d: f64 = xi.iter().zip(mxj).map(|(a, b)| a * b).sum();
            orth = orth.max((d - if i == j { 1.0 } else { 0.0 }).abs());
        }
    }
    let mut res = 0.0_f64;
    for (j, x) in modes.vectors.iter().enumerate() {
        let mut kx = vec![0.0; n];
        k.mul_vec(x, &mut kx).expect("sizes");
        let r = (0..n)
            .map(|i| (kx[i] - modes.values[j] * mx[j][i]).abs())
            .fold(0.0, f64::max);
        let norm = mx[j].iter().fold(0.0_f64, |b, v| b.max(v.abs()));
        res = res.max(r / (norm * scale));
    }
    (orth, res)
}

/// 固有ベクトルの残差まで確かめるテスト用の厳しい許容値（既定は 1e-12）。
fn tight() -> SubspaceOptions {
    SubspaceOptions {
        tolerance: 1e-14,
        ..SubspaceOptions::default()
    }
}

fn sin2(j: f64, denom: f64) -> f64 {
    (j * PI / denom).sin().powi(2)
}

#[test]
fn fixed_fixed_chain_n200_lowest_six() {
    // λ_j = (4k/m) sin²(jπ / (2(n+1)))
    let (n, k, m) = (200, 3.0, 0.5);
    let kmat = chain_k(n, k, Ends::FixedFixed);
    let mass = vec![m; n];
    let a = 4.0 * k / m;
    let modes =
        lowest_modes(&kmat, &Mass::Diagonal(&mass), 6, 1e-3 * a, &tight()).expect("converges");
    let want: Vec<f64> = (1..=6)
        .map(|j| a * sin2(j as f64, 2.0 * (n + 1) as f64))
        .collect();
    let err = worst_rel(&modes.values, &want);
    let (orth, res) = quality(&kmat, &Mass::Diagonal(&mass), &modes, a);
    eprintln!(
        "ff n=200: rel err {err:e}, orth {orth:e}, residual {res:e}, iterations {}",
        modes.iterations
    );
    assert!(err < 1e-10, "{err:e}");
    assert!(orth < 1e-10 && res < 1e-9, "{orth:e} {res:e}");
    assert!(!modes.cluster_split);
}

#[test]
fn fixed_fixed_chain_n2000_lowest_six() {
    let (n, k, m) = (2000, 1.0, 1.0);
    let kmat = chain_k(n, k, Ends::FixedFixed);
    let mass = vec![m; n];
    let a = 4.0 * k / m;
    let start = std::time::Instant::now();
    let modes =
        lowest_modes(&kmat, &Mass::Diagonal(&mass), 6, 1e-4 * a, &tight()).expect("converges");
    eprintln!("n=2000 wall time: {:?}", start.elapsed());
    let want: Vec<f64> = (1..=6)
        .map(|j| a * sin2(j as f64, 2.0 * (n + 1) as f64))
        .collect();
    let err = worst_rel(&modes.values, &want);
    let (orth, res) = quality(&kmat, &Mass::Diagonal(&mass), &modes, a);
    eprintln!(
        "ff n=2000: rel err {err:e}, orth {orth:e}, residual {res:e}, iterations {}",
        modes.iterations
    );
    assert!(err < 1e-9, "{err:e}");
    assert!(orth < 1e-9 && res < 1e-8, "{orth:e} {res:e}");
}

#[test]
fn fixed_free_chain_matches_closed_form() {
    // λ_j = (4k/m) sin²((2j−1)π / (2(2n+1)))
    let (n, k, m) = (300, 2.0, 1.5);
    let kmat = chain_k(n, k, Ends::FixedFree);
    let mass = vec![m; n];
    let a = 4.0 * k / m;
    let modes =
        lowest_modes(&kmat, &Mass::Diagonal(&mass), 6, 1e-3 * a, &tight()).expect("converges");
    let want: Vec<f64> = (1..=6)
        .map(|j| a * sin2((2 * j - 1) as f64, 2.0 * (2 * n + 1) as f64))
        .collect();
    let err = worst_rel(&modes.values, &want);
    eprintln!("fixed-free n=300: rel err {err:e}");
    assert!(err < 1e-10, "{err:e}");
}

#[test]
fn free_free_chain_has_one_zero_mode_through_the_shift() {
    // 特異な K: λ_1 = 0、以降 λ_j = (4k/m) sin²((j−1)π / (2n))
    let (n, k, m) = (200, 1.0, 1.0);
    let kmat = chain_k(n, k, Ends::FreeFree);
    let mass = vec![m; n];
    let a = 4.0 * k / m;
    let modes =
        lowest_modes(&kmat, &Mass::Diagonal(&mass), 6, 1e-2 * a, &tight()).expect("converges");
    let want: Vec<f64> = (2..=6)
        .map(|j| a * sin2((j - 1) as f64, 2.0 * n as f64))
        .collect();
    let err = worst_rel(&modes.values[1..], &want);
    eprintln!(
        "free-free n=200: |lambda_1| = {:e}, rel err of the rest {err:e}",
        modes.values[0]
    );
    assert!(
        modes.values[0].abs() < 1e-12 * modes.values[1],
        "{:e}",
        modes.values[0]
    );
    assert!(err < 1e-10, "{err:e}");
    // 零モードは一様並進 x = 1/√(n m)
    let v = &modes.vectors[0];
    let want = 1.0 / (n as f64 * m).sqrt();
    let dev = v
        .iter()
        .map(|x| (x - want).abs() / want)
        .fold(0.0, f64::max);
    eprintln!("zero mode: largest relative deviation from the uniform vector {dev:e}");
    // 固有ベクトルの誤差は ≈ 残差 / 固有値の間隔（λ_2/a ≈ 6e-5、残差は 1e-11 台）なので 1e-7 を許容値とする
    assert!(dev < 1e-7, "{dev:e}");
}

#[test]
fn rigid_body_count_two_point_masses_in_3d() {
    // 3 次元の 2 点（x 軸上）を x 方向のばね 1 本でつなぐ。自由度 6、ばねは x 方向のみ:
    // 剛体モードは 並進 3 + 回転 2（x 軸まわりは質点なので無関係、y, z 方向の相対運動は力なし）= 5、弾性モードは λ = 2k/m が 1 つ。
    let (k, m) = (4.0, 2.0);
    let mut kmat = Skyline::from_couplings(6, [(0, 3)]).expect("profile");
    for (i, j, s) in [(0, 0, 1.0), (3, 3, 1.0), (0, 3, -1.0)] {
        kmat.add(i, j, s * k).expect("inside");
    }
    let mass = vec![m; 6];
    let modes = lowest_modes(&kmat, &Mass::Diagonal(&mass), 6, 0.5, &tight()).expect("converges");
    eprintln!("3-D pair: {:?}", modes.values);
    assert!(
        modes.values[..5].iter().all(|v| v.abs() < 1e-12),
        "{:?}",
        modes.values
    );
    assert!((modes.values[5] - 2.0 * k / m).abs() < 1e-12 * 2.0 * k / m);
}

#[test]
fn three_uncoupled_free_chains_give_three_zero_modes() {
    // 自由‐自由の 50 質点列が 3 本（互いに結合なし）: 零モード 3 個、その次は λ_2 = 4 sin²(π/100) の 3 重根
    let (len, copies) = (50, 3);
    let n = len * copies;
    let mut kmat = Skyline::from_couplings(
        n,
        (0..copies).flat_map(|c| (1..len).map(move |i| (c * len + i - 1, c * len + i))),
    )
    .expect("profile");
    for c in 0..copies {
        for i in 1..len {
            let (a, b) = (c * len + i - 1, c * len + i);
            kmat.add(a, a, 1.0).expect("d");
            kmat.add(b, b, 1.0).expect("d");
            kmat.add(a, b, -1.0).expect("o");
        }
    }
    let mass = vec![1.0; n];
    let opts = tight();
    let modes = lowest_modes(&kmat, &Mass::Diagonal(&mass), 6, 0.05, &opts).expect("converges");
    let lam2 = 4.0 * sin2(1.0, 2.0 * len as f64);
    eprintln!("3 chains: {:?} (want 0,0,0 then {lam2:e} x3)", modes.values);
    assert!(modes.values[..3].iter().all(|v| v.abs() < 1e-12 * lam2));
    assert!(worst_rel(&modes.values[3..], &[lam2; 3]) < 1e-10);
    assert!(
        !modes.cluster_split,
        "the 3-fold root is returned completely"
    );
    let (orth, res) = quality(&kmat, &Mass::Diagonal(&mass), &modes, 4.0);
    assert!(orth < 1e-9 && res < 1e-9, "{orth:e} {res:e}");
}

/// 非一様な質量の固定‐固定列（k = 1）を、自由度の並べ替え `perm`（元の i を perm[i] 番に置く）で組む。
fn permuted_chain(n: usize, perm: &[usize], mass_scale: f64, k_scale: f64) -> (Skyline, Vec<f64>) {
    let mut kmat =
        Skyline::from_couplings(n, (1..n).map(|i| (perm[i - 1], perm[i]))).expect("profile");
    let mut spring = |a: Option<usize>, b: Option<usize>| {
        for (i, j, s) in [(a, a, 1.0), (b, b, 1.0), (a, b, -1.0)] {
            if let (Some(i), Some(j)) = (i, j) {
                kmat.add(perm[i], perm[j], s * k_scale).expect("inside");
            }
        }
    };
    (1..n).for_each(|i| spring(Some(i - 1), Some(i)));
    spring(None, Some(0));
    spring(Some(n - 1), None);
    let mut mass = vec![0.0; n];
    for i in 0..n {
        mass[perm[i]] = mass_scale * (1.0 + (i % 3) as f64 * 0.5);
    }
    (kmat, mass)
}

fn lowest(kmat: &Skyline, mass: &[f64], count: usize, shift: f64) -> Modes {
    lowest_modes(kmat, &Mass::Diagonal(mass), count, shift, &tight()).expect("converges")
}

#[test]
fn scaling_laws_and_permutation_invariance() {
    let n = 120;
    let ident: Vec<usize> = (0..n).collect();
    let (k0, m0) = permuted_chain(n, &ident, 1.0, 1.0);
    let base = lowest(&k0, &m0, 5, 1e-3);
    // M を 4 倍 → すべての λ が 1/4
    let (k1, m1) = permuted_chain(n, &ident, 4.0, 1.0);
    let quarter: Vec<f64> = base.values.iter().map(|v| v / 4.0).collect();
    let e_m = worst_rel(&lowest(&k1, &m1, 5, 1e-3 / 4.0).values, &quarter);
    // K を 4 倍 → すべての λ が 4 倍
    let (k2, m2) = permuted_chain(n, &ident, 1.0, 4.0);
    let quad: Vec<f64> = base.values.iter().map(|v| v * 4.0).collect();
    let e_k = worst_rel(&lowest(&k2, &m2, 5, 4e-3).values, &quad);
    // 偶数番を先、奇数番を後ろに並べ替える（バンド幅が大きく変わる）
    let perm: Vec<usize> = (0..n)
        .map(|i| if i % 2 == 0 { i / 2 } else { n / 2 + i / 2 })
        .collect();
    let (k3, m3) = permuted_chain(n, &perm, 1.0, 1.0);
    let permuted = lowest(&k3, &m3, 5, 1e-3);
    let e_p = worst_rel(&permuted.values, &base.values);
    eprintln!("scaling: M x4 {e_m:e}, K x4 {e_k:e}, permutation {e_p:e}");
    assert!(e_m < 1e-10 && e_k < 1e-10 && e_p < 1e-10);
    // 固有ベクトルも並べ替えで対応する（符号は向きの規約で揃わないので内積の絶対値で比べる）
    for (j, x) in base.vectors.iter().enumerate() {
        let y: Vec<f64> = (0..n).map(|i| permuted.vectors[j][perm[i]]).collect();
        let d: f64 = x.iter().zip(&y).zip(&m0).map(|((a, b), m)| a * b * m).sum();
        assert!((d.abs() - 1.0).abs() < 1e-8, "mode {j}: {d}");
    }
}

/// 一貫質量の 1 次元棒（要素剛性 k[[1,-1],[-1,1]]、要素質量 (m/6)[[2,1],[1,2]]）の両端固定、内部節点 n 個。
fn consistent_bar(n: usize, k: f64, m: f64) -> (Skyline, Skyline) {
    let couplings = || (1..n).map(|i| (i - 1, i));
    let mut kmat = Skyline::from_couplings(n, couplings()).expect("profile");
    let mut mmat = Skyline::from_couplings(n, couplings()).expect("profile");
    // n+1 要素: 要素 e は節点 e−1 と e をつなぐ（節点 −1 と n は壁。壁の自由度は消去）
    for e in 0..=n {
        let nodes = [e.checked_sub(1), (e < n).then_some(e)];
        for (a, ia) in nodes.iter().enumerate() {
            for (b, ib) in nodes.iter().enumerate() {
                // 対称行列なので (a, b) と (b, a) の二重に足さないよう a ≤ b だけを足す
                if let (Some(i), Some(j), true) = (ia, ib, a <= b) {
                    let same = a == b;
                    kmat.add(*i, *j, if same { k } else { -k }).expect("inside");
                    mmat.add(*i, *j, m / 6.0 * if same { 2.0 } else { 1.0 })
                        .expect("inside");
                }
            }
        }
    }
    (kmat, mmat)
}

#[test]
fn consistent_mass_bar_matches_derived_closed_form() {
    // 内部節点 i の方程式に u_i = sin(iθ) を代入すると
    //   K 行: k(−u_{i−1} + 2u_i − u_{i+1}) = k (2 − 2cosθ) u_i
    //   M 行: (m/6)(u_{i−1} + 4u_i + u_{i+1}) = (m/6)(4 + 2cosθ) u_i
    // よって λ = (6k/m)(2 − 2cosθ)/(4 + 2cosθ) = (6k/m)(1 − cosθ)/(2 + cosθ)。
    // 両端固定（u_0 = u_{n+1} = 0）から θ_j = jπ/(n+1)。
    let closed = |n: usize, k: f64, m: f64, j: usize| {
        let th = j as f64 * PI / (n + 1) as f64;
        6.0 * k / m * (1.0 - th.cos()) / (2.0 + th.cos())
    };
    // (1) 導出を密な一般化固有値解法で数値的に確かめる
    let (n0, k, m) = (12, 2.0, 0.7);
    let (kd, md) = consistent_bar(n0, k, m);
    let dense = |s: &Skyline| -> Vec<f64> {
        (0..n0 * n0)
            .map(|p| s.get(p / n0, p % n0).expect("in range"))
            .collect()
    };
    let e = generalized_symmetric_eigen(&dense(&kd), &dense(&md), n0).expect("ok");
    let derr = worst_rel(
        &e.values,
        &(1..=n0).map(|j| closed(n0, k, m, j)).collect::<Vec<_>>(),
    );
    eprintln!("consistent bar derivation vs dense solve (n=12): {derr:e}");
    assert!(derr < 1e-12, "{derr:e}");
    // (2) 部分空間反復（M は一般の対称行列）
    let n = 400;
    let (kmat, mmat) = consistent_bar(n, k, m);
    let mass = Mass::Skyline(&mmat);
    let modes = lowest_modes(&kmat, &mass, 6, 1e-3, &tight()).expect("converges");
    let want: Vec<f64> = (1..=6).map(|j| closed(n, k, m, j)).collect();
    let err = worst_rel(&modes.values, &want);
    let (orth, res) = quality(&kmat, &mass, &modes, 12.0 * k / m);
    eprintln!("consistent bar n=400: rel err {err:e}, orth {orth:e}, residual {res:e}");
    assert!(err < 1e-10, "{err:e}");
    assert!(orth < 1e-9 && res < 1e-9, "{orth:e} {res:e}");
}

mod edge;
