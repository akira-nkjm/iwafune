//! 集中質量の取り付け。質量の位置を取り付け先のパネルの面に投影し、その点を含む要素の 4 節点に双線形の重みで配る。
//! 外形（フットプリント）があるときは、投影した範囲を格子状に並べた点それぞれに同じ質量を配る。
//! 最も近いパネルが（許容差の範囲で）複数あれば、質量をそれらに等分する。
//! 面から離れた位置のオフセット（回転慣性の増え方）は考えない。

use super::input::{ModalFemInput, ModalFemLumped};
use super::mesh::{DEFAULT_MERGE_TOLERANCE, Mesh, PanelMesh};
use crate::spec::AnalysisError;

/// フットプリントの 1 辺あたりの標本点の数。
pub const FOOTPRINT_SAMPLES: usize = 5;

/// 集中質量の取り付け先。
#[derive(Debug, Clone)]
pub struct Attachment {
    pub id: String,
    /// 取り付けたパネルの ID
    pub panel: String,
    /// 質量の位置からパネルの面までの距離 [m]
    pub distance: f64,
    /// 最も近いパネルを自動で選んだか（`false` は名前で指定）
    pub automatic: bool,
    /// この部品の質量のうち、このパネルに配った割合（名前で指定なら 1。最も近いパネルが複数なら等分）
    pub share: f64,
}

/// 点からパネルの長方形までの最も近い点（面内の座標 (u, v) と距離）。
fn closest(p: &PanelMesh, pos: [f64; 3]) -> ([f64; 2], f64) {
    let [u0, u1, v0, v1] = p.extent();
    let q = [pos[p.axes[0]].clamp(u0, u1), pos[p.axes[1]].clamp(v0, v1)];
    let d = [
        pos[p.axes[0]] - q[0],
        pos[p.axes[1]] - q[1],
        pos[p.normal] - p.plane,
    ];
    (q, d.iter().map(|x| x * x).sum::<f64>().sqrt())
}

/// 格子線の列 `lines` の中で `x` を含む区間の添字と区間内の位置（0..1）。
fn locate(lines: &[f64], x: f64) -> (usize, f64) {
    let i = lines
        .partition_point(|l| *l <= x)
        .saturating_sub(1)
        .min(lines.len() - 2);
    (
        i,
        ((x - lines[i]) / (lines[i + 1] - lines[i])).clamp(0.0, 1.0),
    )
}

/// 面内の点 (u, v) に質量 `m` を双線形の重みで配る。
fn spread(p: &PanelMesh, uv: [f64; 2], m: f64, out: &mut [f64]) {
    let (i, s) = locate(&p.lines[0], uv[0]);
    let (j, t) = locate(&p.lines[1], uv[1]);
    let w = [
        ((i, j), (1.0 - s) * (1.0 - t)),
        ((i + 1, j), s * (1.0 - t)),
        ((i + 1, j + 1), s * t),
        ((i, j + 1), (1.0 - s) * t),
    ];
    for ((a, b), weight) in w {
        out[p.node(a, b)] += m * weight;
    }
}

/// フットプリントの標本点（面内の座標）。無い・点なら中心 1 点。
fn samples(p: &PanelMesh, l: &ModalFemLumped, pos: [f64; 3]) -> Vec<[f64; 2]> {
    let [u0, u1, v0, v1] = p.extent();
    let size = l
        .footprint
        .map_or([0.0, 0.0], |f| [f[p.axes[0]].value(), f[p.axes[1]].value()]);
    if size[0] <= 0.0 && size[1] <= 0.0 {
        return vec![[pos[p.axes[0]].clamp(u0, u1), pos[p.axes[1]].clamp(v0, v1)]];
    }
    let at =
        |k: usize, c: f64, len: f64| c + len * ((k as f64 + 0.5) / FOOTPRINT_SAMPLES as f64 - 0.5);
    let mut out = Vec::new();
    for a in 0..FOOTPRINT_SAMPLES {
        for b in 0..FOOTPRINT_SAMPLES {
            out.push([
                at(a, pos[p.axes[0]], size[0]).clamp(u0, u1),
                at(b, pos[p.axes[1]], size[1]).clamp(v0, v1),
            ]);
        }
    }
    out
}

/// 取り付け先のパネルの組。名前の指定があればそのパネル 1 枚。無ければ、最も近い距離から許容差 `tol` 以内のパネルすべて
/// （距離が同じ、またはほぼ同じパネルが複数あれば、質量を等分する。どれか 1 枚に偏らせる理由が無い）。
fn choose<'a>(
    mesh: &'a Mesh,
    l: &ModalFemLumped,
    pos: [f64; 3],
    tol: f64,
) -> Result<Vec<&'a PanelMesh>, AnalysisError> {
    if let Some(name) = &l.panel {
        return mesh
            .panels
            .iter()
            .find(|p| &p.id == name)
            .map(|p| vec![p])
            .ok_or_else(|| AnalysisError::InvalidInput(format!("{}: unknown panel {name}", l.id)));
    }
    let nearest = mesh
        .panels
        .iter()
        .map(|p| closest(p, pos).1)
        .min_by(f64::total_cmp)
        .ok_or_else(|| AnalysisError::InvalidInput("No panels".to_owned()))?;
    Ok(mesh
        .panels
        .iter()
        .filter(|p| closest(p, pos).1 <= nearest + tol)
        .collect())
}

/// 集中質量を節点の並進質量 `out` に足し、取り付け先（パネルごとの割合つき）を返す。
pub fn attach(
    mesh: &Mesh,
    input: &ModalFemInput,
    out: &mut [f64],
) -> Result<Vec<Attachment>, AnalysisError> {
    let tol = input
        .merge_tolerance
        .map_or(DEFAULT_MERGE_TOLERANCE, |t| t.value());
    let mut list = Vec::new();
    for l in &input.lumped {
        let pos = l.position.map(|x| x.value());
        let set = choose(mesh, l, pos, tol)?;
        let share = 1.0 / set.len() as f64;
        for p in set {
            let pts = samples(p, l, pos);
            let each = l.mass.value() * share / pts.len() as f64;
            for uv in &pts {
                spread(p, *uv, each, out);
            }
            list.push(Attachment {
                id: l.id.clone(),
                panel: p.id.clone(),
                distance: closest(p, pos).1,
                automatic: l.panel.is_none(),
                share,
            });
        }
    }
    Ok(list)
}
