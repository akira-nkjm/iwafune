//! 梁要素（`beam`）。要素単体の性質（対称・剛体運動で力が出ない）と、片持ち梁の振動数を閉じた式と比べる。
//! 梁の頭の質量は節点の集中質量（並進は ρ A h、回転は ρ I h。梁の回転慣性）で、モデルの質量の置き方と同じ。

use super::super::beam::{BEAM_DOF, BeamProps, stiffness};
use super::*;
use crate::structure::modal_estimate::{ModalEstimateBeam, beam_frequencies_hz};
use assemble::System;
use sat_astro::math::Skyline;
use std::f64::consts::PI;

/// 回転の質量（梁の回転慣性 ρ I h）に掛ける比。曲げの振動数の収束だけを見るので、回転慣性の効果（(β r/L)² 程度）は消しておく
/// （0 にすると質量行列が特異になるので、無視できる小さな値にする）。
const ROTARY: f64 = 1.0e-6;

/// 一辺 `a` の中実の正方形の梁（アルミ）。
fn props(a: f64) -> BeamProps {
    BeamProps {
        youngs_modulus: E_AL,
        poisson_ratio: 0.33,
        section: ModalFemMemberSection {
            width: len(a),
            wall: None,
        }
        .properties()
        .expect("a section"),
    }
}

/// 長さ `l` を `n` 要素に分けた梁（局所の x 方向）の低い固有振動数 [Hz]。節点 0 は全自由度を固定、
/// 他の節点は `keep(成分)` が真の自由度だけを自由にする。
fn chain(n: usize, l: f64, p: &BeamProps, keep: impl Fn(usize) -> bool, count: usize) -> Vec<f64> {
    let h = l / n as f64;
    let ke = stiffness(h, p);
    let free: Vec<usize> = (6..(n + 1) * 6).filter(|g| keep(g % 6)).collect();
    let rank = |g: usize| free.iter().position(|f| *f == g);
    let mut first: Vec<usize> = (0..free.len()).collect();
    for e in 0..n {
        let dofs: Vec<usize> = (0..12).map(|c| e * 6 + c).collect();
        let r: Vec<usize> = dofs.iter().filter_map(|g| rank(*g)).collect();
        let lowest = r.iter().copied().min().unwrap_or(0);
        for &i in &r {
            first[i] = first[i].min(lowest);
        }
    }
    let mut k = Skyline::new(&first).expect("a profile");
    for e in 0..n {
        for (i, row) in ke.iter().enumerate() {
            for (j, x) in row.iter().enumerate() {
                if let (Some(ri), Some(rj)) = (rank(e * 6 + i), rank(e * 6 + j))
                    && ri <= rj
                {
                    k.add(ri, rj, *x).expect("inside the profile");
                }
            }
        }
    }
    let s = p.section;
    let mass = free
        .iter()
        .map(|g| {
            let share = if g / 6 == n { 0.5 } else { 1.0 };
            let rho_h = RHO_AL * h * share;
            match g % 6 {
                0..=2 => rho_h * s.area,
                3 => ROTARY * rho_h * 2.0 * s.second_moment,
                _ => ROTARY * rho_h * s.second_moment,
            }
        })
        .collect();
    let sys = System {
        k,
        mass,
        global: free,
    };
    let solved = solve::solve(&sys, count).expect("solves");
    solved
        .values
        .iter()
        .map(|v| v.sqrt() / (2.0 * PI))
        .collect()
}

/// 断面の性質は手計算の値と合う（中実: A = a²、I = a⁴/12、J = 0.1406 a⁴。角パイプ: 外 10 mm、肉厚 1 mm）。
#[test]
fn section_properties_by_hand() {
    let solid = props(0.01).section;
    assert!(rel(solid.area, 1.0e-4) < 1e-12);
    assert!(rel(solid.second_moment, 1.0e-8 / 12.0) < 1e-12);
    assert!(rel(solid.torsion, 0.1406e-8) < 1e-12);
    let tube = ModalFemMemberSection {
        width: len(0.01),
        wall: Some(len(0.001)),
    }
    .properties()
    .expect("a tube");
    // 内側の辺 8 mm: A = 100 − 64 = 36 mm²、I = (10⁴ − 8⁴)/12 = 492 mm⁴、J = t (a − t)³ = 1 × 9³ = 729 mm⁴
    assert!(rel(tube.area, 36.0e-6) < 1e-12);
    assert!(rel(tube.second_moment, 492.0e-12) < 1e-12);
    assert!(rel(tube.torsion, 729.0e-12) < 1e-12);
    for bad in [(0.0, None), (0.01, Some(0.005)), (0.01, Some(0.0))] {
        let s = ModalFemMemberSection {
            width: len(bad.0),
            wall: bad.1.map(len),
        };
        assert!(s.properties().is_none(), "{bad:?}");
    }
}

/// 要素の剛性行列は対称で、剛体運動（3 並進と 3 回転）で力が出ない。
/// 剛体の回転 φ（z まわり: v = φ x、θz = φ。y まわり: w = −φ x、θy = φ）は、節点 2 の変位が φ L になる。
#[test]
fn element_is_symmetric_and_rigid_motions_are_free() {
    let (l, phi) = (0.07, 0.01);
    let k = stiffness(l, &props(0.006));
    for i in 0..BEAM_DOF {
        for j in 0..BEAM_DOF {
            assert!((k[i][j] - k[j][i]).abs() <= 1e-12 * k[i][i].max(k[j][j]));
        }
    }
    let mut motions = vec![[0.0; BEAM_DOF]; 6];
    for (c, m) in motions.iter_mut().take(3).enumerate() {
        m[c] = 1.0;
        m[6 + c] = 1.0;
    }
    motions[3][3] = phi;
    motions[3][9] = phi;
    motions[4][5] = phi;
    motions[4][11] = phi;
    motions[4][7] = phi * l;
    motions[5][4] = phi;
    motions[5][10] = phi;
    motions[5][8] = -phi * l;
    for (n, m) in motions.iter().enumerate() {
        for row in &k {
            let f: f64 = row.iter().zip(m).map(|(a, b)| a * b).sum();
            let scale = row.iter().map(|x| x.abs()).fold(0.0, f64::max)
                * m.iter().fold(0.0_f64, |a, x| a.max(x.abs()));
            assert!(f.abs() <= 1e-9 * scale, "{n}: {f}");
        }
    }
}

/// 片持ち梁の曲げの一次: f = β₁² /(2π) √(EI/(ρA L⁴))、β₁ = 1.8751（文献の値）。梁の厳密解（`modal_estimate`）とも比べる。
/// L = 0.4 m、a = 6 mm。並進の質量だけを節点に置く（集中質量）と、質量の積分が台形則になるので、離散化の誤差は要素の長さ h の二乗に比例する
/// （相対誤差 ≈ C (β₁ h/L)²）。許容差は要素数から決める: (1) 要素数を倍にすると誤差が 1/4 になる（二次の収束、比 4 ± 0.25）、
/// (2) n = 20 と 40 の結果から h² の項を消した値（Richardson: f₄₀ + (f₄₀ − f₂₀)/3）が閉じた式に、残りの h⁴ の項の大きさ
/// 2 (β₁/40)⁴ ≈ 1e-5 の内で合う。
#[test]
fn cantilever_bending_frequency() {
    let (l, a): (f64, f64) = (0.4, 0.006);
    let p = props(a);
    let beta1 = 1.875_104_068_7_f64;
    let closed = beta1 * beta1 / (2.0 * PI)
        * (E_AL * p.section.second_moment / (RHO_AL * p.section.area * l.powi(4))).sqrt();
    let (exact, _) = beam_frequencies_hz(&ModalEstimateBeam {
        id: "B".to_owned(),
        length: len(l),
        bending_stiffness: Qty::from_raw(E_AL * p.section.second_moment),
        axial_stiffness: None,
        mass: Qty::from_raw(RHO_AL * p.section.area * l),
        tip_mass: None,
    })
    .expect("beam");
    assert!(rel(exact, closed) < 1e-6, "{exact} vs {closed}");
    let f = |n: usize| chain(n, l, &p, |_| true, 4)[0];
    let (f20, f40, f80) = (f(20), f(40), f(80));
    let error = |x: f64| rel(x, closed);
    println!(
        "bending: errors {:e} {:e} {:e}",
        error(f20),
        error(f40),
        error(f80)
    );
    assert!(
        (error(f20) / error(f40) - 4.0).abs() < 0.25,
        "{} {}",
        error(f20),
        error(f40)
    );
    assert!(
        (error(f40) / error(f80) - 4.0).abs() < 0.25,
        "{} {}",
        error(f40),
        error(f80)
    );
    let richardson = f40 + (f40 - f20) / 3.0;
    assert!(
        error(richardson) < 2.0 * (beta1 / 40.0).powi(4),
        "{}",
        error(richardson)
    );
}

/// 根元固定の棒の縦振動の一次: f = (1/4L)√(E/ρ)（導ける）。集中質量の離散化は ω_h = (2c/h) sin(k h/2)（k = π/2L）で、
/// 相対誤差は (kh)²/24。ここでは離散化の式との一致（1e-6）と、閉じた式との差が (kh)²/24 の 1.05 倍以内を確かめる。
#[test]
fn axial_frequency() {
    let (l, a, n) = (0.4, 0.006, 40);
    let p = props(a);
    let c = (E_AL / RHO_AL).sqrt();
    let closed = c / (4.0 * l);
    let h = l / n as f64;
    let k = PI / (2.0 * l);
    let discrete = 2.0 * c / h * (k * h / 2.0).sin() / (2.0 * PI);
    let f = chain(n, l, &p, |comp| comp == 0, 3)[0];
    println!("axial: fem {f}, closed {closed}, discrete {discrete}");
    assert!(rel(f, discrete) < 1e-6, "{f} vs {discrete}");
    assert!(
        rel(f, closed) < 1.05 * (k * h).powi(2) / 24.0,
        "{f} vs {closed}"
    );
}
