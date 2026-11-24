//! 全体の剛性行列（スカイライン）と質量（対角）の組み立て。固定する自由度は取り除いて番号を詰める。
//! 要素（シェル・梁）の局所の剛性行列は、要素の局所軸の向き（`frame`）で機体の軸に回してから足す
//! （軸に平行な要素では軸の巡回の並べ替えなので、値は変わらない）。

use super::beam::{self, BEAM_DOF, BeamProps};
use super::masses::NodeMasses;
use super::mesh::{Mesh, in_plane_axes};
use super::shell::{ELEMENT_DOF, ShellProps, stiffness};
use crate::spec::AnalysisError;
use sat_astro::math::Skyline;

/// 組み立てた系。自由度は固定を除いて詰めた番号（自由な自由度の番号）。
#[derive(Debug, Clone)]
pub struct System {
    pub k: Skyline,
    /// 対角の質量
    pub mass: Vec<f64>,
    /// 自由な自由度の番号 → 全体の自由度（節点 × 6 + 成分）
    pub global: Vec<usize>,
}

/// 局所の軸（機体座標での単位ベクトル 3 本）。
type Frame = [[f64; 3]; 3];

/// 節点ごとに並進 3・回転 3 の局所の行列 `ke`（N = 6 × 節点数）を、局所軸 `frame` の向きで機体の軸の行列にする。
/// 3 × 3 のブロックごとに G = Fᵀ B F（F の行が局所の軸）。並進も回転も同じ回転で動く。
fn to_global<const N: usize>(ke: &[[f64; N]; N], frame: &Frame) -> [[f64; N]; N] {
    let mut g = [[0.0; N]; N];
    for r in 0..N / 3 {
        for c in 0..N / 3 {
            // t = B F
            let mut t = [[0.0; 3]; 3];
            for (i, ti) in t.iter_mut().enumerate() {
                for (m, x) in ti.iter_mut().enumerate() {
                    *x = (0..3).map(|j| ke[3 * r + i][3 * c + j] * frame[j][m]).sum();
                }
            }
            for k in 0..3 {
                for m in 0..3 {
                    g[3 * r + k][3 * c + m] = (0..3).map(|i| frame[i][k] * t[i][m]).sum();
                }
            }
        }
    }
    g
}

/// 部材の局所の軸（x = 部材の軸 a、y = a+1、z = a+2。右手系）。
fn member_frame(axis: usize) -> Frame {
    let [u, v] = in_plane_axes(axis);
    [axis, u, v].map(|a| {
        let mut e = [0.0; 3];
        e[a] = 1.0;
        e
    })
}

fn bad(e: impl std::fmt::Display) -> AnalysisError {
    AnalysisError::InvalidInput(format!("Cannot assemble the stiffness matrix: {e}"))
}

/// 要素の節点に対応する全体の自由度（節点 × 6 + 成分）。
fn dofs_of(nodes: &[usize]) -> Vec<usize> {
    nodes
        .iter()
        .flat_map(|n| (0..6).map(move |c| n * 6 + c))
        .collect()
}

/// 要素の（機体の軸の）剛性行列と自由度。
struct Contribution {
    dofs: Vec<usize>,
    ke: Vec<Vec<f64>>,
}

fn rows<const N: usize>(m: &[[f64; N]; N]) -> Vec<Vec<f64>> {
    m.iter().map(|r| r.to_vec()).collect()
}

/// すべての要素（シェル → 梁）の寄与。`shells[i]` は入力のパネル `i`、`beams[i]` はメッシュのフレーム `i` の材料と断面。
fn contributions(mesh: &Mesh, shells: &[ShellProps], beams: &[BeamProps]) -> Vec<Contribution> {
    let mut out: Vec<Contribution> = mesh
        .elements()
        .map(|el| {
            let panel = &mesh.panels[el.panel];
            let ke = stiffness(el.a, el.b, &shells[panel.source]);
            Contribution {
                dofs: dofs_of(&el.nodes),
                ke: rows::<ELEMENT_DOF>(&to_global(&ke, &panel.frame)),
            }
        })
        .collect();
    out.extend(mesh.members().map(|m| {
        let ke = beam::stiffness(m.length, &beams[m.frame]);
        Contribution {
            dofs: dofs_of(&m.nodes),
            ke: rows::<BEAM_DOF>(&to_global(&ke, &member_frame(m.axis))),
        }
    }));
    out
}

/// 組み立てる。
pub fn assemble(
    mesh: &Mesh,
    shells: &[ShellProps],
    beams: &[BeamProps],
    fixed: &[bool],
    masses: &NodeMasses,
) -> Result<System, AnalysisError> {
    let mut free_of: Vec<Option<usize>> = vec![None; fixed.len()];
    let mut global = Vec::new();
    for (g, f) in fixed.iter().enumerate() {
        if !f {
            free_of[g] = Some(global.len());
            global.push(g);
        }
    }
    let n = global.len();
    if n == 0 {
        return Err(AnalysisError::InvalidInput(
            "Every degree of freedom is fixed".to_owned(),
        ));
    }
    let list = contributions(mesh, shells, beams);
    let mut first: Vec<usize> = (0..n).collect();
    for c in &list {
        let free: Vec<usize> = c.dofs.iter().filter_map(|g| free_of[*g]).collect();
        let lowest = free.iter().copied().min().unwrap_or(0);
        for &f in &free {
            first[f] = first[f].min(lowest);
        }
    }
    let mut k = Skyline::new(&first).map_err(bad)?;
    for c in &list {
        for (i, gi) in c.dofs.iter().enumerate() {
            let Some(ri) = free_of[*gi] else { continue };
            for (j, gj) in c.dofs.iter().enumerate() {
                match free_of[*gj] {
                    Some(rj) if ri <= rj => k.add(ri, rj, c.ke[i][j]).map_err(bad)?,
                    _ => {}
                }
            }
        }
    }
    let mass = global.iter().map(|g| masses.at_dof(*g)).collect();
    Ok(System { k, mass, global })
}
