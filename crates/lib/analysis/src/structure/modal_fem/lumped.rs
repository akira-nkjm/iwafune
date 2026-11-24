//! 集中質量の取り付け。つなぎ先の優先は、名前の指定のパネル → 取り付け先のフレームの部材（[`super::frame_tie`]）→ 最も近いパネル。
//! パネルへは、質量の位置をパネルの面に投影し、その点を含む要素の 4 節点に双線形の重みで配る。
//! 外形（フットプリント）があるときは、投影した範囲を格子状に並べた点それぞれに同じ質量を配る。
//! 最も近いパネルが（許容差の範囲で）複数あれば、質量をそれらに等分する。
//! 面から離れた位置のオフセット（回転慣性の増え方）は考えない。

use super::attachment::{Attachment, FOOTPRINT_SAMPLES};
use super::frame_tie;
use super::input::{ModalFemInput, ModalFemLumped};
use super::mesh::{DEFAULT_MERGE_TOLERANCE, Mesh, PanelMesh};
use crate::spec::AnalysisError;

/// 点からパネルの長方形までの最も近い点（面内の局所の座標 (u, v) と距離）。
fn closest(p: &PanelMesh, pos: [f64; 3]) -> ([f64; 2], f64) {
    let [u0, u1, v0, v1] = p.extent();
    let l = p.local(pos);
    let q = [l[0].clamp(u0, u1), l[1].clamp(v0, v1)];
    let d = [l[0] - q[0], l[1] - q[1], l[2]];
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

/// フットプリントの標本点（面内の局所の座標）。無い・点なら中心 1 点。
/// 外形の箱（機体の軸に平行）の面内の軸への投影の長さは、箱の辺に軸の向きの成分の絶対値を掛けた和
/// （軸に平行なパネルでは、面内の軸に対応する辺の長さそのもの）。
fn samples(p: &PanelMesh, l: &ModalFemLumped, pos: [f64; 3]) -> Vec<[f64; 2]> {
    let [u0, u1, v0, v1] = p.extent();
    let c = p.local(pos);
    let size = l.footprint.map_or([0.0, 0.0], |f| {
        [0, 1].map(|i| (0..3).map(|k| p.frame[i][k].abs() * f[k].value()).sum())
    });
    if size[0] <= 0.0 && size[1] <= 0.0 {
        return vec![[c[0].clamp(u0, u1), c[1].clamp(v0, v1)]];
    }
    let at =
        |k: usize, c: f64, len: f64| c + len * ((k as f64 + 0.5) / FOOTPRINT_SAMPLES as f64 - 0.5);
    let mut out = Vec::new();
    for a in 0..FOOTPRINT_SAMPLES {
        for b in 0..FOOTPRINT_SAMPLES {
            out.push([
                at(a, c[0], size[0]).clamp(u0, u1),
                at(b, c[1], size[1]).clamp(v0, v1),
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

/// 部品がつなぐフレーム（`Mesh::frames` の添字）。パネルを名前で指していず（設定・取り付け先のパネル）、
/// 取り付け先のフレームに部材がある（辺に節点が 2 つ以上ある辺が 1 本以上ある）ときだけ。
fn frame_of(mesh: &Mesh, l: &ModalFemLumped) -> Option<usize> {
    let id = l.frame.as_ref().filter(|_| l.panel.is_none())?;
    mesh.frames
        .iter()
        .position(|f| &f.id == id && !f.edges.is_empty())
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
        if let Some(frame) = frame_of(mesh, l) {
            list.extend(frame_tie::tie(mesh, frame, l, pos, out));
            continue;
        }
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
                edge: None,
                distance: closest(p, pos).1,
                automatic: l.panel.is_none(),
                share,
            });
        }
    }
    Ok(list)
}
