//! 質量行列（対角の集中質量）の組み立て。パネルの質量・分布質量・フレームの質量・集中質量を節点に配る。
//! フレームの質量は、部材（梁要素）の長さに比例して部材に配り、各部材の質量を両端の節点に半分ずつ置く（回転慣性は置かない）。
//! 部材が 1 本も無いフレームの質量は、分布質量と同じく全体に配る。
//! 並進の質量は節点ごとに 1 つ（3 方向で同じ）、回転慣性も節点ごとに 1 つ（3 軸で同じ）。

use super::attachment::Attachment;
use super::input::{ModalFemInput, ModalFemPanel};
use super::lumped::attach;
use super::mesh::Mesh;
use super::shell::node_mass;
use crate::spec::AnalysisError;

/// 節点の質量。
#[derive(Debug, Clone)]
pub struct NodeMasses {
    /// 並進の質量 [kg]（節点ごと）
    pub translational: Vec<f64>,
    /// 回転慣性 [kg·m²]（節点ごと、3 軸共通）
    pub rotary: Vec<f64>,
    /// 集中質量の取り付け先
    pub attachments: Vec<Attachment>,
    /// フレームごと（`Mesh::frames` の順）の、部材に配った質量 [kg]（部材が無ければ 0）
    pub frame_member_mass: Vec<f64>,
}

impl NodeMasses {
    /// 全体の自由度 `dof`（節点 × 6 + 成分）の質量。
    pub fn at_dof(&self, dof: usize) -> f64 {
        let (node, comp) = (dof / 6, dof % 6);
        if comp < 3 {
            self.translational[node]
        } else {
            self.rotary[node]
        }
    }

    /// 並進の質量の合計 [kg]。
    pub fn total(&self) -> f64 {
        self.translational.iter().sum()
    }
}

/// パネルの質量 [kg]。`mass` を与えていればそれ、なければ密度 × 板厚 × 面積。
pub fn panel_mass(p: &ModalFemPanel, area: f64) -> f64 {
    p.mass.map_or_else(
        || p.density.value() * p.thickness.value() * area,
        |m| m.value(),
    )
}

/// 質量を組み立てる。
pub fn build(mesh: &Mesh, input: &ModalFemInput) -> Result<NodeMasses, AnalysisError> {
    let n = mesh.node_count();
    let mut m = NodeMasses {
        translational: vec![0.0; n],
        rotary: vec![0.0; n],
        attachments: Vec::new(),
        frame_member_mass: Vec::new(),
    };
    let mut tributary = vec![0.0; n];
    let totals: Vec<f64> = mesh
        .panels
        .iter()
        .map(|p| panel_mass(&input.panels[p.source], p.area()))
        .collect();
    for el in mesh.elements() {
        let pm = &mesh.panels[el.panel];
        let src = &input.panels[pm.source];
        let share = el.a * el.b / pm.area();
        let (mt, mr) = node_mass(totals[el.panel] * share, src.thickness.value());
        for node in el.nodes {
            m.translational[node] += mt;
            m.rotary[node] += mr;
            tributary[node] += el.a * el.b / 4.0;
        }
    }
    m.frame_member_mass = frame_members(mesh, input, &mut m.translational);
    let loose: f64 = input
        .frames
        .iter()
        .zip(&m.frame_member_mass)
        .filter(|(_, placed)| **placed == 0.0)
        .map(|(f, _)| f.mass.value())
        .sum();
    if input.distributed_mass.is_some() || loose > 0.0 {
        let dm = input.distributed_mass.map_or(0.0, |d| d.value()) + loose;
        let area: f64 = tributary.iter().sum();
        for (t, a) in m.translational.iter_mut().zip(&tributary) {
            *t += dm * a / area;
        }
    }
    m.attachments = attach(mesh, input, &mut m.translational)?;
    Ok(m)
}

/// フレームの質量を部材に配り、フレームごとの配った質量を返す（部材が無ければ 0）。
fn frame_members(mesh: &Mesh, input: &ModalFemInput, out: &mut [f64]) -> Vec<f64> {
    let members: Vec<_> = mesh.members().collect();
    mesh.frames
        .iter()
        .enumerate()
        .map(|(fi, fm)| {
            let own: Vec<_> = members.iter().filter(|m| m.frame == fi).collect();
            let length: f64 = own.iter().map(|m| m.length).sum();
            let mass = input.frames[fm.source].mass.value();
            if length <= 0.0 {
                return 0.0;
            }
            for m in own {
                let half = mass * m.length / length / 2.0;
                out[m.nodes[0]] += half;
                out[m.nodes[1]] += half;
            }
            mass
        })
        .collect()
}
