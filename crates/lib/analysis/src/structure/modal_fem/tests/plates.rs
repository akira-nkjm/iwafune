//! 1 枚の板。四辺単純支持は閉じた式、四辺固定と片持ちは文献の振動数パラメータと比べる。
//!
//! 無次元の振動数パラメータ λ² = ω a² √(ρ h / D)（D = E h³ / (12 (1 − ν²))）、f = λ² / (2π a²) · √(D / (ρ h))。

use super::*;
use crate::structure::modal_estimate::{ModalEstimatePanel, panel_frequency_hz};
use std::f64::consts::PI;

const A: f64 = 0.2;
const T: f64 = 0.001;
const NU: f64 = 0.3;

fn square(t: f64, a: f64) -> ModalFemPanel {
    panel("P", ModalFemAxis::Z, [0.0; 3], [a, a], t, NU)
}

/// 板の剛性と質量から決まる速度の尺度 √(D / (ρ h)) [m²/s]。
fn scale(t: f64) -> f64 {
    let d = E_AL * t.powi(3) / (12.0 * (1.0 - NU * NU));
    (d / (RHO_AL * t)).sqrt()
}

/// 振動数パラメータ λ² から f [Hz]。
fn from_parameter(lambda2: f64, a: f64, t: f64) -> f64 {
    lambda2 / (2.0 * PI * a * a) * scale(t)
}

/// 四辺単純支持の一次: f = (π/2)(1/a² + 1/b²) √(D/(ρh))（Kirchhoff 板の閉じた式。導ける）。
fn simply_supported(a: f64, t: f64) -> f64 {
    PI / 2.0 * (2.0 / (a * a)) * scale(t)
}

fn ss_plate(t: f64) -> ModalFemInput {
    input(vec![square(t, A)], four_edges(false))
}

fn with_mesh(mut i: ModalFemInput, h: f64) -> ModalFemInput {
    i.mesh_size = Some(len(h));
    i
}

/// 拘束なしの板: 剛体のモードが 6 個、ほぼ 0 Hz。7 番目からが弾性のモード。
/// 3 つの並進の剛体モードの有効質量の割合は、軸ごとに合計 1（拘束なしの系は全質量が動く）。
#[test]
fn free_plate_has_six_rigid_modes() {
    let mut i = input(vec![square(T, A)], ModalFemSupport::Free);
    i.mode_count = Some(8);
    let o = solve(&i);
    assert_eq!(
        o.rigid_body_modes,
        6,
        "{:?}",
        o.modes.iter().map(hz).collect::<Vec<_>>()
    );
    for m in &o.modes[..6] {
        assert!(hz(m) < 0.01, "rigid mode at {} Hz", hz(m));
        assert_eq!(m.class, ModalFemModeClass::Rigid);
    }
    for m in &o.modes[6..] {
        assert!(hz(m) > 1.0, "elastic mode at {} Hz", hz(m));
    }
    for axis in 0..3 {
        let sum: f64 = o.modes[..6]
            .iter()
            .map(|m| m.effective_mass_fraction[axis].value())
            .sum();
        assert!((sum - 1.0).abs() < 1e-6, "axis {axis}: {sum}");
    }
}

/// 四辺単純支持の正方形の板の一次（既定のメッシュ）。基準は閉じた式（導ける）と `modal_estimate` のパネルの式。
/// 許容差 2 %: 既定のメッシュ（a/8）の離散化の誤差の見込み（集中質量は振動数を低めに出す。実測は収束の節で a/4・a/8・a/16 を確かめる）。
#[test]
fn simply_supported_square_plate() {
    let o = solve(&ss_plate(T));
    let reference = simply_supported(A, T);
    let estimate = panel_frequency_hz(&ModalEstimatePanel {
        id: "P".to_owned(),
        length_a: len(A),
        length_b: len(A),
        thickness: len(T),
        youngs_modulus: Qty::from_raw(E_AL),
        poisson_ratio: NU,
        density: Qty::from_raw(RHO_AL),
        mounted: vec![],
    })
    .expect("estimate");
    assert!(
        rel(estimate, reference) < 1e-12,
        "the two closed forms agree"
    );
    let f = hz(&o.modes[0]);
    println!(
        "SS plate: fem {f} Hz, closed form {reference} Hz, error {:e}",
        rel(f, reference)
    );
    assert!(rel(f, reference) < 0.02, "{f} vs {reference}");
    assert!(rel(f, estimate) < 0.02, "{f} vs modal_estimate {estimate}");
    assert_eq!(o.rigid_body_modes, 0);
}

/// (1,2) と (2,1) は正方形で同じ値（メッシュが対称なので離散でも同じ。許容差 1e-6 = 反復の収束の許容値の余裕）。
/// 値は閉じた式 (π/2)(1/a² + 4/a²)√(D/ρh) = 2.5 f₁₁ とも合う（許容差 2 %: 同じ離散化誤差の見込み）。
#[test]
fn degenerate_modes_on_a_square_plate() {
    let o = solve(&ss_plate(T));
    let f = |k: usize| hz(&o.modes[k]);
    assert!(rel(f(1), f(2)) < 1e-6, "{} vs {}", f(1), f(2));
    let reference = 2.5 * simply_supported(A, T);
    println!("SS (1,2): fem {} Hz, closed form {reference} Hz", f(1));
    assert!(rel(f(1), reference) < 0.02);
}

/// 一次のモードの有効質量の割合（法線方向）。形が sin·sin の連続体では 64/π⁴ = 0.657（導ける）。
/// 集中質量の離散化では、節点の和（台形則）が積分の代わりになるので、n×n 分割の離散の値は
/// cot⁴(π/2n) / (n² (n/2)²)（Σ sin(kπ/n) = cot(π/2n)、Σ sin² = n/2 から導ける）。n = 8 で 0.624、n = 16 で 0.649。
/// 許容差: 離散の値に対して 1 %（モードの形が標本化した sin から少しずれる分）。連続体の値には a/16 で 2 %
/// （台形則の誤差 −1.3 % の見込み）で、a/8 より近いこと。
#[test]
fn first_mode_effective_mass() {
    let discrete = |n: f64| (PI / (2.0 * n)).tan().recip().powi(4) / (n * n * (n / 2.0).powi(2));
    let frac =
        |h: f64| solve(&with_mesh(ss_plate(T), h)).modes[0].effective_mass_fraction[2].value();
    let (f8, f16) = (frac(A / 8.0), frac(A / 16.0));
    let continuum = 64.0 / PI.powi(4);
    println!(
        "fraction a/8 {f8} (discrete {}), a/16 {f16} (discrete {}), continuum {continuum}",
        discrete(8.0),
        discrete(16.0)
    );
    assert!(rel(f8, discrete(8.0)) < 0.01, "{f8}");
    assert!(rel(f16, discrete(16.0)) < 0.01, "{f16}");
    assert!(rel(f16, continuum) < 0.02 && rel(f16, continuum) < rel(f8, continuum));
}

/// 四辺固定の正方形の板（ν = 0.3）の一次: λ² = 35.99（Leissa, "Vibration of Plates", 1969, Table 4.? の値。**記憶から**。文献では未確認）。
/// 許容差 3 %: 固定辺は離散化の収束が遅い見込み（既定のメッシュ a/8）。細かいメッシュ（a/16）は 1 % 以内。
#[test]
fn clamped_square_plate() {
    let reference = from_parameter(35.99, A, T);
    let f = hz(&solve(&input(vec![square(T, A)], four_edges(true))).modes[0]);
    let fine = hz(&solve(&with_mesh(
        input(vec![square(T, A)], four_edges(true)),
        A / 16.0,
    ))
    .modes[0]);
    println!("clamped plate: fem {f} (a/8), {fine} (a/16) vs {reference}");
    assert!(rel(f, reference) < 0.03, "{f} vs {reference}");
    assert!(rel(fine, reference) < 0.01, "{fine} vs {reference}");
}

/// 片持ちの正方形の板（1 辺固定、他の 3 辺は自由、ν = 0.3）の一次: λ² = 3.494（Leissa 1969。**記憶から**。文献では未確認）。
/// 許容差 3 %: 自由辺・固定辺の離散化誤差の見込み（既定のメッシュ a/8）。a/16 は 1.5 % 以内。
#[test]
fn cantilever_square_plate() {
    let reference = from_parameter(3.494, A, T);
    let i = input(vec![square(T, A)], clamp_face(ModalFemFace::MinusX));
    let f = hz(&solve(&i).modes[0]);
    let fine = hz(&solve(&with_mesh(i, A / 16.0)).modes[0]);
    println!("cantilever plate: fem {f} (a/8), {fine} (a/16) vs {reference}");
    assert!(rel(f, reference) < 0.03, "{f} vs {reference}");
    assert!(rel(fine, reference) < 0.015, "{fine} vs {reference}");
}

/// 密度 ×4 で質量行列が 4 倍、固有値は 1/4、振動数は 1/2（離散でも厳密。許容差 1e-8 = 反復の収束の許容値の余裕）。
#[test]
fn density_scaling() {
    let f1 = hz(&solve(&ss_plate(T)).modes[0]);
    let mut heavy = ss_plate(T);
    heavy.panels[0].density = Qty::from_raw(4.0 * RHO_AL);
    let f4 = hz(&solve(&heavy).modes[0]);
    assert!(rel(f4, f1 / 2.0) < 1e-8, "{f4} vs {}", f1 / 2.0);
}

/// 板厚 ×2 で曲げの振動数は 2 倍（曲げ剛性 t³、質量 t）。薄い板 (a/t = 400) で比べる。
/// 許容差 0.5 %: Mindlin のせん断・回転慣性の補正は (t/h)² のオーダー（h = 25 mm、t = 0.5 mm で約 4e-4 に係数）。
#[test]
fn thickness_scaling() {
    let thin = hz(&solve(&ss_plate(0.0005)).modes[0]);
    let thick = hz(&solve(&ss_plate(0.001)).modes[0]);
    println!("thickness: {thin} -> {thick}, ratio {}", thick / thin);
    assert!(rel(thick, 2.0 * thin) < 0.005);
}

/// 質量を載せる位置: 中央 < 1/4 点 < 縁（縁の節点は固定なので板だけと同じ）。質量 0 なら板だけの値に戻る（離散でも厳密）。
#[test]
fn lumped_mass_position() {
    let plate = hz(&solve(&ss_plate(T)).modes[0]);
    let mass = 0.5 * RHO_AL * T * A * A;
    let with = |p: [f64; 3], kg: f64| {
        let mut i = ss_plate(T);
        i.lumped = vec![lumped("M", p, kg)];
        hz(&solve(&i).modes[0])
    };
    let centre = with([0.0, 0.0, 0.0], mass);
    let quarter = with([A / 4.0, A / 4.0, 0.0], mass);
    let edge = with([A / 2.0, 0.0, 0.0], mass);
    println!("plate {plate}, centre {centre}, quarter {quarter}, edge {edge}");
    assert!(centre < quarter && quarter < plate);
    assert!(
        rel(edge, plate) < 1e-9,
        "a mass on a supported edge changes nothing"
    );
    assert!(rel(with([0.0, 0.0, 0.0], 0.0), plate) < 1e-9);
}

/// メッシュの収束: a/4・a/8・a/16 で誤差が単調に減り（2 次の収束なら約 4 分の 1 ずつ）、既定のメッシュ（a/8）で 1 % 以内、
/// a/16 で 0.3 % 以内（誤差の見込み: 実測前に決めた値ではなく、収束の傾きを確かめるための上限）。
#[test]
fn mesh_convergence() {
    let reference = simply_supported(A, T);
    let err = |h: f64| rel(hz(&solve(&with_mesh(ss_plate(T), h)).modes[0]), reference);
    let (e4, e8, e16) = (err(A / 4.0), err(A / 8.0), err(A / 16.0));
    println!("errors: a/4 {e4:e}, a/8 {e8:e}, a/16 {e16:e}");
    assert!(e4 > e8 && e8 > e16);
    assert!(e8 < 0.01, "default mesh error {e8}");
    assert!(e16 < 0.003, "fine mesh error {e16}");
    assert!(e4 / e8 > 3.0 && e8 / e16 > 3.0, "about second order");
    // 既定のメッシュは a/8（= min(a/4 の辺の 1/4 ...) ではなく min(a/4, 25 mm) の 25 mm）。
    let o = solve(&ss_plate(T));
    assert!(rel(o.model.mesh_size.value(), 0.025) < 1e-12);
}
