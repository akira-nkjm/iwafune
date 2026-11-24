//! モードの形（`ModalFemShapes`）。四辺単純支持の正方形の板の一次のモードは sin(πx/a)·sin(πy/b)（導ける）で、
//! 規格化・節点数・四角形の数・既定の出力が変わらないことも確かめる。

use super::*;
use crate::spec::Analysis;
use std::f64::consts::PI;

const A: f64 = 0.2;
const T: f64 = 0.001;

fn ss_plate(h: f64) -> ModalFemInput {
    let mut i = input(
        vec![panel("P", ModalFemAxis::Z, [0.0; 3], [A, A], T, 0.3)],
        four_edges(false),
    );
    i.mesh_size = Some(len(h));
    i
}

fn shapes_of(i: &ModalFemInput) -> ModalFemShapes {
    let done = ModalFem::run_detailed(i, true).expect("runs");
    serde_json::from_value(done.detail.expect("the detail is returned")).expect("the detail type")
}

/// 一次のモードの節点の面外の変位と sin(πξ/a)·sin(πη/a)（ξ, η は板の隅からの座標）の差の最大。
fn plate_error(h: f64) -> f64 {
    let s = shapes_of(&ss_plate(h));
    let first = &s.modes[0];
    s.mesh
        .nodes
        .iter()
        .zip(&first.displacement)
        .map(|(p, d)| {
            let (xi, eta) = (f64::from(p[0]) + A / 2.0, f64::from(p[1]) + A / 2.0);
            let exact = (PI * xi / A).sin() * (PI * eta / A).sin();
            (f64::from(d[2]) - exact).abs()
        })
        .fold(0.0, f64::max)
}

/// 一次のモードは節点で sin·sin に合う。許容差は離散化の見込みから決める: 中心差分型の演算子の主要な誤差は
/// 波数 k = π/a と要素の大きさ h で相対 (k h)² / 12 なので、その 2 倍 (k h)² / 6 を許す（a/8 で 0.026）。
/// 規格化で最大が 1 なので、これは最大値に対する絶対誤差でもある。
#[test]
fn first_mode_of_a_simply_supported_plate_is_a_product_of_half_sines() {
    let h = A / 8.0;
    let err = plate_error(h);
    let tol = (PI * h / A).powi(2) / 6.0;
    println!("SS plate shape error {err:e} (tolerance {tol:e})");
    assert!(err < tol, "{err} vs {tol}");
}

/// 規格化: どのモードも、節点の並進の大きさの最大が 1。`magnitude` は `displacement` のノルム。
/// 許容差 1e-6: 表示用に `f32` へ丸めた分（相対 6e-8）の余裕。
#[test]
fn shapes_are_normalised_to_a_unit_maximum() {
    let s = shapes_of(&input(
        box_panels(0.1, 0.2, 0.3, 0.002),
        clamp_face(ModalFemFace::MinusZ),
    ));
    assert_eq!(s.normalization, ModalFemShapeNorm::MaxTranslation);
    assert_eq!(s.modes.len(), DEFAULT_MODE_COUNT as usize);
    for m in &s.modes {
        let max = m.magnitude.iter().copied().fold(0.0_f32, f32::max);
        assert!((max - 1.0).abs() < 1e-6, "mode {}: max {max}", m.index);
        for (d, mag) in m.displacement.iter().zip(&m.magnitude) {
            let norm = d.iter().map(|x| x * x).sum::<f32>().sqrt();
            assert!((norm - mag).abs() < 1e-6);
        }
    }
}

/// 節点・四角形の数がメッシュの大きさ（`model`）と合い、四角形の節点番号は範囲内。モードの番号は出力と同じ。
#[test]
fn counts_match_the_model_size() {
    let i = input(
        box_panels(0.1, 0.2, 0.3, 0.002),
        clamp_face(ModalFemFace::MinusZ),
    );
    let done = ModalFem::run_detailed(&i, true).expect("runs");
    let s: ModalFemShapes = serde_json::from_value(done.detail.expect("detail")).expect("type");
    assert_eq!(s.mesh.nodes.len() as u32, done.output.model.nodes);
    let quads: usize = s.mesh.groups.iter().map(|g| g.quads.len()).sum();
    assert_eq!(quads as u32, done.output.model.elements);
    assert_eq!(s.mesh.groups.len(), 6);
    assert!(
        s.mesh
            .groups
            .iter()
            .flat_map(|g| g.quads.iter().flatten())
            .all(|n| (*n as usize) < s.mesh.nodes.len())
    );
    for (shape, mode) in s.modes.iter().zip(&done.output.modes) {
        assert_eq!(shape.index, mode.index);
        assert_eq!(shape.displacement.len(), s.mesh.nodes.len());
    }
}

/// 頼まなければ詳細は無く、出力は `run` と同じ。頼んでも出力は変わらない。
#[test]
fn detail_is_opt_in_and_does_not_change_the_output() {
    let i = ss_plate(A / 4.0);
    let plain = serde_json::to_value(ModalFem::run(&i).expect("runs")).expect("json");
    let off = ModalFem::run_detailed(&i, false).expect("runs");
    assert!(off.detail.is_none());
    assert_eq!(serde_json::to_value(&off.output).expect("json"), plain);
    let on = ModalFem::run_detailed(&i, true).expect("runs");
    assert!(on.detail.is_some());
    assert_eq!(serde_json::to_value(&on.output).expect("json"), plain);
}

/// 符号は決定的: 大きい成分（最大の 99 % 以上）のうち最初のものは正。
#[test]
fn the_first_large_component_is_positive() {
    let s = shapes_of(&ss_plate(A / 4.0));
    for m in &s.modes {
        let max = m.magnitude.iter().copied().fold(0.0_f32, f32::max);
        let first = m
            .displacement
            .iter()
            .flatten()
            .copied()
            .find(|x| f64::from(x.abs()) >= 0.99 * f64::from(max))
            .expect("a large component");
        assert!(first > 0.0, "mode {}", m.index);
    }
}
