use super::*;
use crate::math::solve_dense;

/// 全成分を持つ（バンド幅が最大の）スカイラインを密行列（行優先）から作る。
fn from_dense(a: &[f64], n: usize) -> Skyline {
    let mut s = Skyline::new(&vec![0; n]).expect("profile");
    for j in 0..n {
        for i in 0..=j {
            s.add(i, j, a[i * n + j]).expect("inside");
        }
    }
    s
}

/// 三重対角 [-1, 2, -1]（固定端の 1 次元ばね列）。
fn tridiag(n: usize) -> Skyline {
    let first: Vec<usize> = (0..n).map(|j| j.saturating_sub(1)).collect();
    let mut s = Skyline::new(&first).expect("profile");
    for i in 0..n {
        s.add(i, i, 2.0).expect("diag");
        if i > 0 {
            s.add(i - 1, i, -1.0).expect("off");
        }
    }
    s
}

/// 決定的な対称正定値行列 A_ij = 1/(i+j+1) + n·δij（Hilbert 行列に対角優位を足したもの）。
fn spd_dense(n: usize) -> Vec<f64> {
    let mut a = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..n {
            a[i * n + j] = 1.0 / (i + j + 1) as f64 + if i == j { n as f64 } else { 0.0 };
        }
    }
    a
}

#[test]
fn tridiagonal_solution_is_known_parabola() {
    // -x'' = 1 の離散版: [-1,2,-1] x = 1 の解は x_i = i(n+1-i)/2（i = 1..n、厳密）
    let n = 50;
    let ldl = tridiag(n).into_ldl().expect("spd");
    let mut b = vec![1.0; n];
    ldl.solve_in_place(&mut b).expect("solve");
    for (i, x) in b.iter().enumerate() {
        let want = (i + 1) as f64 * (n - i) as f64 / 2.0;
        assert!((x - want).abs() <= 1e-12 * want, "{i}: {x} vs {want}");
    }
    // ピボットは d_j = (j+2)/(j+1)（j = 0 始まり）
    for j in 0..n {
        let want = (j + 2) as f64 / (j + 1) as f64;
        assert!((ldl.pivot(j).expect("in range") - want).abs() < 1e-14);
    }
    assert_eq!(ldl.negative_pivots(), 0);
}

#[test]
fn diagonal_matrix_solves_by_division() {
    let first: Vec<usize> = (0..4).collect();
    let mut s = Skyline::new(&first).expect("profile");
    for (i, d) in [2.0, 4.0, 8.0, 0.5].iter().enumerate() {
        s.add(i, i, *d).expect("diag");
    }
    let ldl = s.into_ldl().expect("spd");
    let mut b = vec![1.0, 2.0, 4.0, 1.0];
    ldl.solve_in_place(&mut b).expect("solve");
    assert_eq!(b, vec![0.5, 0.5, 0.5, 2.0]);
}

#[test]
fn small_dense_matrix_with_hand_inverse() {
    // A = [[4,2,2],[2,5,3],[2,3,6]]。A [1,-1,2]ᵀ = [6,3,11]ᵀ なので解は [1,-1,2]。
    // 手計算の分解: d = (4, 4, 4)、u01 = 1/2, u02 = 1/2, u12 = 1/2（d2 = 6 − ½·2 − ½·2 = 4）
    let s = from_dense(&[4.0, 2.0, 2.0, 2.0, 5.0, 3.0, 2.0, 3.0, 6.0], 3);
    let ldl = s.into_ldl().expect("spd");
    for (j, d) in [4.0, 4.0, 4.0].iter().enumerate() {
        assert!((ldl.pivot(j).expect("in range") - d).abs() < 1e-14);
    }
    let mut b = vec![6.0, 3.0, 11.0];
    ldl.solve_in_place(&mut b).expect("solve");
    for (g, w) in b.iter().zip([1.0, -1.0, 2.0]) {
        assert!((g - w).abs() < 1e-14, "{g} vs {w}");
    }
}

#[test]
fn mul_vec_matches_dense_product() {
    let n = 7;
    let a = spd_dense(n);
    let s = from_dense(&a, n);
    let x: Vec<f64> = (0..n).map(|i| (i as f64 - 2.5) * 0.3).collect();
    let mut y = vec![0.0; n];
    s.mul_vec(&x, &mut y).expect("sizes");
    for i in 0..n {
        let want: f64 = (0..n).map(|j| a[i * n + j] * x[j]).sum();
        assert!((y[i] - want).abs() < 1e-14, "{i}");
    }
    // 三重対角: A·[1,1,1,…] は両端だけ 1、内部は 0
    let mut y = vec![0.0; 5];
    tridiag(5).mul_vec(&[1.0; 5], &mut y).expect("sizes");
    assert_eq!(y, vec![1.0, 0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn agrees_with_dense_solver_on_spd_formula() {
    let n = 12;
    let a = spd_dense(n);
    let b: Vec<f64> = (0..n).map(|i| ((i * 7) % 5) as f64 - 1.5).collect();
    let want = solve_dense(a.clone(), b.clone()).expect("regular");
    let ldl = from_dense(&a, n).into_ldl().expect("spd");
    let mut x = b.clone();
    ldl.solve_in_place(&mut x).expect("solve");
    for (g, w) in x.iter().zip(&want) {
        assert!((g - w).abs() < 1e-13 * w.abs().max(1.0), "{g} vs {w}");
    }
}

#[test]
fn many_right_hand_sides_equal_one_by_one() {
    let ldl = tridiag(9).into_ldl().expect("spd");
    let mut many: Vec<Vec<f64>> = (0..3)
        .map(|c| (0..9).map(|i| (i + c) as f64).collect())
        .collect();
    let singles = many.clone();
    ldl.solve_many(&mut many).expect("solve");
    for (m, mut s) in many.iter().zip(singles) {
        ldl.solve_in_place(&mut s).expect("solve");
        assert_eq!(m, &s);
    }
}

#[test]
fn varying_profile_equals_full_profile() {
    // 先頭行がばらばらのプロファイルと、同じ値を全成分で持つ行列で、解が一致する
    let first = [0, 0, 1, 1, 3, 2, 5];
    let n = first.len();
    let mut sky = Skyline::new(&first).expect("profile");
    let mut dense = vec![0.0; n * n];
    for j in 0..n {
        for i in first[j]..=j {
            let v = if i == j {
                10.0 + j as f64
            } else {
                1.0 / (1 + j - i) as f64
            };
            sky.add(i, j, v).expect("inside");
            dense[i * n + j] = v;
            dense[j * n + i] = v;
        }
    }
    let b: Vec<f64> = (0..n).map(|i| 1.0 + i as f64).collect();
    let want = solve_dense(dense, b.clone()).expect("regular");
    let mut x = b;
    sky.into_ldl()
        .expect("spd")
        .solve_in_place(&mut x)
        .expect("solve");
    for (g, w) in x.iter().zip(&want) {
        assert!((g - w).abs() < 1e-13, "{g} vs {w}");
    }
}

#[test]
fn singular_matrix_reports_the_zero_pivot() {
    // 両端自由の 2 質点ばね [[1,-1],[-1,1]]: 2 番目のピボットが 0
    let s = from_dense(&[1.0, -1.0, -1.0, 1.0], 2);
    assert!(matches!(
        s.clone().into_ldl(),
        Err(SkylineError::ZeroPivot { index: 1, .. })
    ));
    assert!(matches!(
        s.count_negative_eigenvalues(),
        Err(SkylineError::ZeroPivot { index: 1, .. })
    ));
    // 両端自由の 5 質点
    let n = 5;
    let mut chain = Skyline::from_couplings(n, (1..n).map(|i| (i - 1, i))).expect("profile");
    for i in 1..n {
        chain.add(i - 1, i - 1, 1.0).expect("d");
        chain.add(i, i, 1.0).expect("d");
        chain.add(i - 1, i, -1.0).expect("o");
    }
    assert!(matches!(
        chain.into_ldl(),
        Err(SkylineError::ZeroPivot { index: 4, .. })
    ));
}

#[test]
fn negative_pivots_count_negative_eigenvalues() {
    // 対角行列 diag(3, -1, -2, 5, -7): 負の固有値 3 個（一般の対角にも効く）
    let d = [3.0, -1.0, -2.0, 5.0, -7.0];
    let mut s = Skyline::new(&(0..5).collect::<Vec<_>>()).expect("profile");
    d.iter()
        .enumerate()
        .for_each(|(i, v)| s.add(i, i, *v).expect("d"));
    assert_eq!(s.count_negative_eigenvalues(), Ok(3));
    assert!(matches!(
        s.into_ldl(),
        Err(SkylineError::NonPositivePivot { index: 1, .. })
    ));
    // [[1,2],[2,1]] の固有値は 3 と -1: 負は 1 個
    let s = from_dense(&[1.0, 2.0, 2.0, 1.0], 2);
    assert_eq!(s.count_negative_eigenvalues(), Ok(1));
    // [-1,2,-1] 行列から 2·(1 − cos(kπ/(n+1))) の間の値 c（k = 4.3。整数 k だと部分行列の固有値と重なって零ピボットになる）を引くと、
    // c より小さい固有値（k = 1..4）の数が負のピボットの数
    let n = 20;
    let c = 2.0 * (1.0 - (4.3 * std::f64::consts::PI / (n + 1) as f64).cos());
    let mut shifted = tridiag(n);
    (0..n).for_each(|i| shifted.add(i, i, -c).expect("d"));
    assert_eq!(shifted.count_negative_eigenvalues(), Ok(4));
}

#[test]
fn rejects_bad_input_without_panicking() {
    assert_eq!(
        Skyline::new(&[0, 2]),
        Err(SkylineError::BadProfile {
            col: 1,
            first_row: 2
        })
    );
    let mut s = tridiag(4);
    assert_eq!(
        s.add(0, 3, 1.0),
        Err(SkylineError::OutsideProfile { row: 0, col: 3 })
    );
    assert_eq!(
        s.add(9, 9, 1.0),
        Err(SkylineError::OutsideProfile { row: 9, col: 9 })
    );
    let mut y = vec![0.0; 4];
    assert!(s.mul_vec(&[1.0; 3], &mut y).is_err());
    let ldl = s.into_ldl().expect("spd");
    assert!(ldl.solve_in_place(&mut [1.0; 3]).is_err());
    assert_eq!(ldl.pivot(4), None);
    assert!(Skyline::from_couplings(2, [(0, 5)]).is_err());
}
