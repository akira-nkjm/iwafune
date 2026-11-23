//! 再現性・不正な引数・Sturm 列の判定・密な解法との照合。

use super::*;

#[test]
fn results_are_bit_for_bit_reproducible() {
    let (n, k, m) = (150, 1.0, 1.0);
    let kmat = chain_k(n, k, Ends::FreeFree);
    let mass = vec![m; n];
    let a = lowest(&kmat, &mass, 6, 0.01);
    let b = lowest(&kmat, &mass, 6, 0.01);
    assert_eq!(a, b);
}

#[test]
fn invalid_arguments_return_typed_errors() {
    let kmat = chain_k(10, 1.0, Ends::FixedFixed);
    let mass = vec![1.0; 10];
    let go = |count, mass: &[f64], opts: &SubspaceOptions| {
        lowest_modes(&kmat, &Mass::Diagonal(mass), count, 0.1, opts)
    };
    let opts = tight();
    assert_eq!(
        go(0, &mass, &opts),
        Err(SubspaceError::InvalidCount { count: 0, n: 10 })
    );
    assert_eq!(
        go(11, &mass, &opts),
        Err(SubspaceError::InvalidCount { count: 11, n: 10 })
    );
    assert_eq!(
        go(3, &mass[..9], &opts),
        Err(SubspaceError::SizeMismatch { k: 10, m: 9 })
    );
    let capped = SubspaceOptions {
        max_iterations: 1,
        ..opts
    };
    let kmat2 = chain_k(200, 1.0, Ends::FixedFixed);
    let big = vec![1.0; 200];
    assert!(matches!(
        lowest_modes(&kmat2, &Mass::Diagonal(&big), 6, 0.1, &capped),
        Err(SubspaceError::NotConverged {
            iterations: 1,
            count: 6,
            ..
        })
    ));
    // 負のシフトが大きすぎて K + σM が正定値でない
    assert!(matches!(
        lowest_modes(&kmat, &Mass::Diagonal(&mass), 3, -1.0, &opts),
        Err(SubspaceError::Factor(_))
    ));
    // 全部（count = n）は求められる
    assert_eq!(go(10, &mass, &opts).expect("all modes").values.len(), 10);
}

/// 同一の固定‐固定列を 2 本並べる（固有値はすべて 2 重）。
fn twin_chains(len: usize) -> (Skyline, Vec<f64>) {
    let n = 2 * len;
    let mut kmat = Skyline::from_couplings(
        n,
        (0..2).flat_map(|c| (1..len).map(move |i| (c * len + i - 1, c * len + i))),
    )
    .expect("profile");
    for c in 0..2 {
        for i in 0..len {
            let g = c * len + i;
            kmat.add(g, g, 2.0).expect("d");
            if i > 0 {
                kmat.add(g - 1, g, -1.0).expect("o");
            }
        }
    }
    (kmat, vec![1.0; n])
}

#[test]
fn sturm_check_with_a_double_eigenvalue_at_the_cut() {
    let len = 60;
    let (kmat, mass) = twin_chains(len);
    let lam = |j: f64| 4.0 * sin2(j, 2.0 * (len + 1) as f64);
    let opts = tight();
    // 3 個: 3 番目と 4 番目が同じ固有値 λ_2。取りこぼしではないのでエラーにせず、cluster_split で知らせる
    let three = lowest_modes(&kmat, &Mass::Diagonal(&mass), 3, 0.01, &opts).expect("ok");
    eprintln!(
        "3 of twin chains: {:?} cluster_split {}",
        three.values, three.cluster_split
    );
    assert!(worst_rel(&three.values, &[lam(1.0), lam(1.0), lam(2.0)]) < 1e-10);
    assert!(three.cluster_split);
    let (orth, res) = quality(&kmat, &Mass::Diagonal(&mass), &three, 4.0);
    assert!(orth < 1e-9 && res < 1e-9, "{orth:e} {res:e}");
    // 4 個: 重根を残さず返すので cluster_split は偽
    let four = lowest_modes(&kmat, &Mass::Diagonal(&mass), 4, 0.01, &opts).expect("ok");
    assert!(worst_rel(&four.values, &[lam(1.0), lam(1.0), lam(2.0), lam(2.0)]) < 1e-10);
    assert!(!four.cluster_split);
}

#[test]
fn sturm_check_detects_a_missing_eigenvalue() {
    // 本当の最小 3 個は λ_1..λ_3。λ_2 を抜いた [λ_1, λ_3] を「2 個の結果」として渡すと取りこぼしとして報告される
    let n = 40;
    let kmat = chain_k(n, 1.0, Ends::FixedFixed);
    let mass = vec![1.0; n];
    let lam = |j: f64| 4.0 * sin2(j, 2.0 * (n + 1) as f64);
    let m = Mass::Diagonal(&mass);
    let gap = 1e-6 * lam(3.0);
    let err = sturm::check(&kmat, &m, &[lam(1.0), lam(3.0)], lam(3.0), gap).err();
    assert_eq!(
        err,
        Some(SubspaceError::MissedModes {
            below: 2,
            returned_below: 1
        })
    );
    assert!(sturm::check(&kmat, &m, &[lam(1.0), lam(2.0), lam(3.0)], lam(3.0), gap).is_ok());
}

#[test]
fn full_spectrum_agrees_with_the_dense_generalized_solver() {
    let n = 15;
    let ident: Vec<usize> = (0..n).collect();
    let (kmat, mass) = permuted_chain(n, &ident, 1.0, 1.0);
    let modes = lowest(&kmat, &mass, n, 0.1);
    let dense_k: Vec<f64> = (0..n * n)
        .map(|p| kmat.get(p / n, p % n).expect("in range"))
        .collect();
    let dense_m: Vec<f64> = (0..n * n)
        .map(|p| if p / n == p % n { mass[p / n] } else { 0.0 })
        .collect();
    let want = generalized_symmetric_eigen(&dense_k, &dense_m, n).expect("ok");
    let err = worst_rel(&modes.values, &want.values);
    eprintln!("all 15 modes vs dense solve: {err:e}");
    assert!(err < 1e-11, "{err:e}");
}

#[test]
fn mass_outside_the_stiffness_profile_is_a_typed_error() {
    let kmat = chain_k(5, 1.0, Ends::FixedFixed);
    let mut mmat = Skyline::from_couplings(5, [(0, 4)]).expect("profile");
    (0..5).for_each(|i| mmat.add(i, i, 1.0).expect("d"));
    mmat.add(0, 4, 0.1).expect("inside");
    let r = lowest_modes(
        &kmat,
        &Mass::Skyline(&mmat),
        2,
        0.1,
        &SubspaceOptions::default(),
    );
    assert!(
        matches!(
            r,
            Err(SubspaceError::Mass(SkylineError::OutsideProfile { .. }))
        ),
        "{r:?}"
    );
}
