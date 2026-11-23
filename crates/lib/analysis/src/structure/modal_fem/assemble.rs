//! 全体の剛性行列（スカイライン）と質量（対角）の組み立て。固定する自由度は取り除いて番号を詰める。

use super::masses::NodeMasses;
use super::mesh::{Element, Mesh, in_plane_axes};
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

/// 要素の 24 自由度が対応する全体の自由度（節点 × 6 + 成分）。局所 (u, v, n) は全体の軸の巡回の並べ替え。
fn element_dofs(el: &Element, normal: usize) -> [usize; ELEMENT_DOF] {
    let [u, v] = in_plane_axes(normal);
    let axes = [u, v, normal];
    let mut dofs = [0; ELEMENT_DOF];
    for (a, node) in el.nodes.iter().enumerate() {
        for c in 0..6 {
            dofs[6 * a + c] = node * 6 + axes[c % 3] + 3 * (c / 3);
        }
    }
    dofs
}

fn bad(e: impl std::fmt::Display) -> AnalysisError {
    AnalysisError::InvalidInput(format!("Cannot assemble the stiffness matrix: {e}"))
}

/// 組み立てる。`props[i]` はパネル `i` の要素の材料。
pub fn assemble(
    mesh: &Mesh,
    props: &[ShellProps],
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
    let mut first: Vec<usize> = (0..n).collect();
    for el in mesh.elements() {
        let dofs = element_dofs(&el, mesh.panels[el.panel].normal);
        let free: Vec<usize> = dofs.iter().filter_map(|g| free_of[*g]).collect();
        let lowest = free.iter().copied().min().unwrap_or(0);
        for &f in &free {
            first[f] = first[f].min(lowest);
        }
    }
    let mut k = Skyline::new(&first).map_err(bad)?;
    for el in mesh.elements() {
        let panel = &mesh.panels[el.panel];
        let ke = stiffness(el.a, el.b, &props[el.panel]);
        let dofs = element_dofs(&el, panel.normal);
        for i in 0..ELEMENT_DOF {
            let Some(ri) = free_of[dofs[i]] else { continue };
            for j in 0..ELEMENT_DOF {
                match free_of[dofs[j]] {
                    Some(rj) if ri <= rj => k.add(ri, rj, ke[i][j]).map_err(bad)?,
                    _ => {}
                }
            }
        }
    }
    let mass = global.iter().map(|g| masses.at_dof(*g)).collect();
    Ok(System { k, mass, global })
}
