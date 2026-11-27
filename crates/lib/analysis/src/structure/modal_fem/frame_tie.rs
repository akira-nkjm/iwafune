//! フレームの部材への集中質量の取り付け。フレームに付いた部品（パネルに載っていない部品）の質量を、フレームの部材の節点に配る。
//!
//! 規則（質量の合計と、質量の位置の 1 次モーメントが保たれる）:
//! 1. 質量の位置 p から、部材になっている各辺（箱の辺。節点が並ぶ範囲に限る）への最も近い点 qₑ と距離 dₑ を求める。
//! 2. 辺の組 (i, j) のうち、dᵢ + dⱼ + dist(p, 線分 qᵢqⱼ) が最小のものを選ぶ（同じなら辺の順で先のもの。1 本しか無ければその 1 本）。
//!    p が qᵢ と qⱼ の間にあって近い辺の組が選ばれる（p が線分の上にあれば第 3 項は 0）。
//!    qᵢ に (1 − s)、qⱼ に s の割合で質量を配る（s は p の線分への投影の位置。0〜1 に収める）。
//! 3. 辺の上の点は、その点を挟む 2 つの節点に線形の重み（辺に沿った座標）で配る。
//!
//! 配った質量の重心は、p の線分への投影になる（p を面に投影するパネルへの取り付けと同じく、線分から離れたオフセットと回転慣性は考えない）。
//! 外形（フットプリント）があるときは、外形の箱を格子状に並べた点それぞれに同じ質量を配る（点ごとに 1〜3 を行う）。

use super::attachment::{Attachment, FOOTPRINT_SAMPLES};
use super::input::ModalFemLumped;
use super::mesh::{MemberEdge, Mesh};

/// 辺の上の、軸方向の座標 `t` の点（辺の節点が並ぶ範囲に収める）。
fn point_on(e: &MemberEdge, t: f64) -> [f64; 3] {
    let (b, c) = ((e.axis + 1) % 3, (e.axis + 2) % 3);
    let (lo, hi) = (e.nodes[0].0, e.nodes[e.nodes.len() - 1].0);
    let mut q = [0.0; 3];
    q[e.axis] = t.clamp(lo, hi);
    q[b] = e.fixed[0];
    q[c] = e.fixed[1];
    q
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt()
}

/// 辺の上の座標 `t` の点に質量 `m` を、挟む 2 つの節点に線形の重みで配る。
fn spread(e: &MemberEdge, t: f64, m: f64, out: &mut [f64]) {
    let last = e.nodes.len() - 2;
    let i = e
        .nodes
        .partition_point(|n| n.0 <= t)
        .saturating_sub(1)
        .min(last);
    let (t0, n0) = e.nodes[i];
    let (t1, n1) = e.nodes[i + 1];
    let w = ((t - t0) / (t1 - t0)).clamp(0.0, 1.0);
    out[n0] += m * (1.0 - w);
    out[n1] += m * w;
}

/// 点 `p` から線分 `ab` までの距離と、線分の上の最も近い点の位置 s（a から b へ 0〜1）。
fn segment_offset(p: [f64; 3], a: [f64; 3], b: [f64; 3]) -> (f64, f64) {
    let ab: [f64; 3] = std::array::from_fn(|k| b[k] - a[k]);
    let len2: f64 = ab.iter().map(|x| x * x).sum();
    let dot: f64 = (0..3).map(|k| (p[k] - a[k]) * ab[k]).sum();
    let s = if len2 > 0.0 {
        (dot / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let foot: [f64; 3] = std::array::from_fn(|k| a[k] + s * ab[k]);
    (dist(p, foot), s)
}

/// 質量を配る 2 本の辺と、第 2 の辺への割合 s。辺の組のうち、d₁ + d₂ + （p から 2 点の線分までの距離）が最小のもの
/// （dₑ は p から辺 e の最も近い点までの距離。同じ値なら辺の順で先のもの）。p が 2 点の線分の上にあれば、第 3 項は 0。
/// 辺が 1 本なら、その辺に全部（s = 0）。
fn choose_pair(near: &[([f64; 3], f64)], p: [f64; 3]) -> (usize, Option<usize>, f64) {
    let mut best: Option<(f64, usize, usize, f64)> = None;
    for i in 0..near.len() {
        for j in i + 1..near.len() {
            let (offset, s) = segment_offset(p, near[i].0, near[j].0);
            let cost = near[i].1 + near[j].1 + offset;
            if best.is_none_or(|b| cost < b.0) {
                best = Some((cost, i, j, s));
            }
        }
    }
    best.map_or((0, None, 0.0), |(_, i, j, s)| (i, Some(j), s))
}

/// 1 点の質量 `m` を 2 本の辺（[`choose_pair`]）に配る。`share[e]` に辺 `e` に配った質量を足す。
fn tie_point(edges: &[MemberEdge], p: [f64; 3], m: f64, out: &mut [f64], share: &mut [f64]) {
    let near: Vec<([f64; 3], f64)> = edges
        .iter()
        .map(|e| {
            let q = point_on(e, p[e.axis]);
            (q, dist(p, q))
        })
        .collect();
    let (first, second, s) = choose_pair(&near, p);
    let mut put = |edge: usize, w: f64| {
        let e = &edges[edge];
        spread(e, near[edge].0[e.axis], m * w, out);
        share[edge] += m * w;
    };
    put(first, 1.0 - s);
    if let Some(second) = second {
        put(second, s);
    }
}

/// サンプル点（外形の箱を軸ごとに `FOOTPRINT_SAMPLES` 等分した各区間の中央。長さ 0 の軸は 1 点）。
fn samples(l: &ModalFemLumped, pos: [f64; 3]) -> Vec<[f64; 3]> {
    let size = l.footprint.map_or([0.0; 3], |f| f.map(|x| x.value()));
    let axis = |k: usize| -> Vec<f64> {
        if size[k] > 0.0 {
            (0..FOOTPRINT_SAMPLES)
                .map(|i| pos[k] + size[k] * ((i as f64 + 0.5) / FOOTPRINT_SAMPLES as f64 - 0.5))
                .collect()
        } else {
            vec![pos[k]]
        }
    };
    let (xs, ys, zs) = (axis(0), axis(1), axis(2));
    let mut out = Vec::new();
    for x in &xs {
        for y in &ys {
            for z in &zs {
                out.push([*x, *y, *z]);
            }
        }
    }
    out
}

/// 集中質量をフレーム `frame`（`Mesh::frames` の添字）の部材に配って `out`（節点の並進質量）に足し、辺ごとの取り付けを返す。
pub fn tie(
    mesh: &Mesh,
    frame: usize,
    l: &ModalFemLumped,
    pos: [f64; 3],
    out: &mut [f64],
) -> Vec<Attachment> {
    let fm = &mesh.frames[frame];
    let points = samples(l, pos);
    let each = l.mass.value() / points.len() as f64;
    let mut share = vec![0.0; fm.edges.len()];
    for p in &points {
        tie_point(&fm.edges, *p, each, out, &mut share);
    }
    let total = l.mass.value();
    fm.edges
        .iter()
        .zip(&share)
        .filter(|(_, s)| **s > 0.0)
        .map(|(e, s)| Attachment {
            id: l.id.clone(),
            panel: fm.id.clone(),
            edge: Some(e.faces),
            distance: dist(pos, point_on(e, pos[e.axis])),
            automatic: true,
            share: if total > 0.0 { s / total } else { 0.0 },
        })
        .collect()
}
