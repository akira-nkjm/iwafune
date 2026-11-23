//! 細長い帯: 先端質量つきの片持ち梁（梁の理論）と、根元固定の棒の縦振動（膜の動き）。ポアソン比 0 で 1 次元の理論と一致する。

use super::*;
use crate::structure::modal_estimate::{ModalEstimateBeam, beam_frequencies_hz};
use std::f64::consts::PI;

/// x 方向に長さ `l`、幅 `w`、板厚 `t` の帯（z が法線、x = 0 の辺を固定）。
fn strip(l: f64, w: f64, t: f64) -> ModalFemPanel {
    panel("S", ModalFemAxis::Z, [l / 2.0, 0.0, 0.0], [l, w], t, 0.0)
}

/// 先端質量つきの片持ち梁の一次（z 方向の曲げ）。EI = E w t³ / 12（ν = 0 なので板の曲げ剛性と同じ）、L = 0.3 m、w = 40 mm、t = 3 mm、
/// 先端質量 0.5 kg（帯の質量 0.097 kg）。基準は 2 つ（どちらも導ける）:
/// (a) 分布質量 + 先端質量の梁の厳密解（`modal_estimate` の `beam_frequencies_hz`）、
/// (b) Rayleigh の近似 f = √(3EI/L³ / (M + 33 m/140)) / 2π（梁の質量が先端質量より十分小さいときの k = 3EI/L³）。
/// 許容差 1 %: 要素 10 mm（長さ方向 30 要素）の離散化誤差と、せん断変形（6e-5）の見込み。(b) は近似自体の誤差（約 0.01 %）も含む。
#[test]
fn tip_mass_cantilever_strip() {
    let (l, w, t, tip) = (0.3, 0.04, 0.003, 0.5);
    let mut i = input(vec![strip(l, w, t)], clamp_face(ModalFemFace::MinusX));
    i.launch_axis = ModalFemAxis::X;
    let mut m = lumped("TIP", [l, 0.0, 0.0], tip);
    m.panel = Some("S".to_owned());
    i.lumped = vec![m];
    i.mode_count = Some(4);
    let o = solve(&i);
    let ei = E_AL * w * t.powi(3) / 12.0;
    let mass = RHO_AL * w * t * l;
    let (exact, _) = beam_frequencies_hz(&ModalEstimateBeam {
        id: "B".to_owned(),
        length: len(l),
        bending_stiffness: Qty::from_raw(ei),
        axial_stiffness: None,
        mass: Qty::from_raw(mass),
        tip_mass: Some(Qty::from_raw(tip)),
    })
    .expect("beam");
    let rayleigh = (3.0 * ei / l.powi(3) / (tip + 33.0 / 140.0 * mass)).sqrt() / (2.0 * PI);
    let f = hz(&o.modes[0]);
    println!("strip with tip mass: fem {f}, beam exact {exact}, k=3EI/L^3 {rayleigh}");
    assert!(rel(f, exact) < 0.01, "{f} vs {exact}");
    assert!(rel(f, rayleigh) < 0.01, "{f} vs {rayleigh}");
    assert_eq!(o.modes[0].class, ModalFemModeClass::Lateral);
    assert_eq!(o.lowest_lateral.map(Qty::value), Some(f));
}

/// 根元固定の棒の縦振動（x 方向の膜の動き）の一次: f = (1/4L)√(E/ρ)（ν = 0。導ける）、L = 0.2 m、w = 20 mm、t = 10 mm。
/// 機軸を x にとり、機軸の最低のモードがこの値に合うこと。一次のモードの有効質量の割合は 8/π² = 0.811（導ける）。
/// 許容差 0.5 %（振動数）・3 %（有効質量）: 要素 5 mm（40 要素）の集中質量の離散化誤差 (π/2n)²/24 ≈ 0.06 % と、
/// 有効質量は台形則の誤差・固定端の節点の質量の見込み。
#[test]
fn axial_bar() {
    let (l, w, t) = (0.2, 0.02, 0.01);
    let mut i = input(vec![strip(l, w, t)], clamp_face(ModalFemFace::MinusX));
    i.launch_axis = ModalFemAxis::X;
    i.mode_count = Some(24);
    let o = solve(&i);
    let reference = (E_AL / RHO_AL).sqrt() / (4.0 * l);
    let axial = o.lowest_axial.expect("an axial mode among 24").value();
    let mode = o
        .modes
        .iter()
        .find(|m| m.class == ModalFemModeClass::Axial)
        .expect("axial mode");
    let frac = mode.effective_mass_fraction[0].value();
    println!(
        "axial bar: fem {axial}, closed form {reference}, fraction {frac} vs {}",
        8.0 / PI.powi(2)
    );
    assert!(rel(axial, reference) < 0.005, "{axial} vs {reference}");
    assert!(rel(frac, 8.0 / PI.powi(2)) < 0.03, "{frac}");
}
