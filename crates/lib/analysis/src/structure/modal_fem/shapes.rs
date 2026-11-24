//! モードの形（`ModalFemShapes`）。頼まれたときだけ（`Analysis::run_detailed`）固有ベクトルの並進成分を節点ごとに取り出し、
//! 最大の節点の並進が 1 になるように規格化して、メッシュの幾何と一緒に返す。回転の自由度は出さない。

use super::assemble::System;
use super::mesh::Mesh;
use super::solve::Solved;
use crate::mesh_view::{SurfaceGroup, SurfaceMesh};
use serde::{Deserialize, Serialize};

/// モードの形の規格化。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ModalFemShapeNorm {
    /// 節点の並進ベクトルの大きさの最大が 1（無次元。振幅の絶対値は意味を持たず、形だけが意味を持つ）。
    /// 符号は、大きい成分（最大の 99 % 以上）のうち節点の番号が最初のものが正になるように決める
    MaxTranslation,
}

/// 1 つのモードの節点の変位。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModalFemModeShape {
    /// `ModalFemOutput::modes` の `index`（1 から数える）
    pub index: u32,
    /// 節点ごとの並進の変位 (x, y, z)（`mesh.nodes` の並び。無次元で、最大の大きさが 1）
    pub displacement: Vec<[f32; 3]>,
    /// 節点ごとの変位の大きさ（`displacement` のノルム。最大が 1。色に使う）
    pub magnitude: Vec<f32>,
}

/// モードの形と、それを描くためのメッシュ（`structure.modal_fem` の重い任意の出力）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModalFemShapes {
    pub mesh: SurfaceMesh,
    pub normalization: ModalFemShapeNorm,
    /// 求めたモードの順（剛体のモードも含む）
    pub modes: Vec<ModalFemModeShape>,
}

/// 符号を決めるとき、最大の何割以上の成分を「大きい」とみなすか。
const BIG_FRACTION: f64 = 0.99;

/// `Analysis::DETAIL_TYPE` の値。`ModalFemShapes` の TS の型名と一致する。
pub const DETAIL_TYPE: &str = "ModalFemShapes";

/// メッシュの幾何（節点・パネルごとの四角形・フレームの部材の線）。
pub fn surface(mesh: &Mesh) -> SurfaceMesh {
    let mut groups: Vec<SurfaceGroup> = mesh
        .panels
        .iter()
        .map(|p| SurfaceGroup {
            id: p.id.clone(),
            quads: Vec::new(),
        })
        .collect();
    for el in mesh.elements() {
        groups[el.panel].quads.push(el.nodes.map(|n| n as u32));
    }
    SurfaceMesh {
        nodes: mesh.coords.iter().map(|c| c.map(|x| x as f32)).collect(),
        groups,
        lines: mesh.members().map(|m| m.nodes.map(|n| n as u32)).collect(),
    }
}

/// 固有ベクトル（自由な自由度の順）から、節点の並進を取り出して規格化する。
fn shape(index: u32, sys: &System, phi: &[f64], nodes: usize) -> ModalFemModeShape {
    let mut d = vec![[0.0_f64; 3]; nodes];
    for (r, g) in sys.global.iter().enumerate() {
        if g % 6 < 3 {
            d[g / 6][g % 6] = phi[r];
        }
    }
    let norm = |v: &[f64; 3]| v.iter().map(|x| x * x).sum::<f64>().sqrt();
    let max = d.iter().map(norm).fold(0.0, f64::max);
    let scale = if max > 0.0 { 1.0 / max } else { 0.0 };
    // 符号は固有ベクトルの計算の経路で決まるので、揃える（結果を決定的にする）。絶対値が最大の 99 % 以上の成分のうち、
    // 節点の番号が最初のものが正。最大そのものを選ぶと、反対称のモード（正負の最大が同じ大きさ）で丸めの誤差が符号を決めてしまう
    let first_big = d
        .iter()
        .flatten()
        .copied()
        .find(|x| x.abs() >= BIG_FRACTION * max)
        .unwrap_or(0.0);
    let sign = if first_big < 0.0 { -1.0 } else { 1.0 };
    ModalFemModeShape {
        index,
        displacement: d
            .iter()
            .map(|v| v.map(|x| (sign * scale * x) as f32))
            .collect(),
        magnitude: d.iter().map(|v| (scale * norm(v)) as f32).collect(),
    }
}

/// すべてのモードの形。
pub fn shapes(mesh: &Mesh, sys: &System, solved: &Solved) -> ModalFemShapes {
    ModalFemShapes {
        mesh: surface(mesh),
        normalization: ModalFemShapeNorm::MaxTranslation,
        modes: solved
            .vectors
            .iter()
            .enumerate()
            .map(|(i, phi)| shape(i as u32 + 1, sys, phi, mesh.node_count()))
            .collect(),
    }
}
