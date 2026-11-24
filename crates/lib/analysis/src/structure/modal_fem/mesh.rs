//! メッシュ。パネルの節点・要素・つながりと、フレームの部材の位置。
//!
//! - 軸に平行なパネル（[`grid`]）: 軸ごとに 1 つの共通の格子を作り、節点をその格子の点に置く。共有する辺・T 字では節点が同じ格子点になる。
//! - 軸に沿わないパネル（[`skewed`]）: パネルごとに自分の面内の格子を作り、姿勢で機体座標に置く。許容差の内で合う節点は共有する。
//! - フレームの部材（[`members`]）: フレームの箱の辺の上にある既存の節点を、辺に沿って梁要素でつなぐ（新しい節点は作らない）。
//!
//! どのパネルも同じ [`PanelMesh`]（局所の軸と格子）で表すので、要素・質量・取り付けは軸に平行かどうかを区別しない。

mod grid;
mod members;
mod skewed;

use super::input::{ModalFemFace, ModalFemInput};
use crate::spec::AnalysisError;
use std::collections::BTreeMap;

pub use crate::plate_grid::in_plane_axes;

/// 既定の要素の大きさの上限 [m]。
pub const DEFAULT_MESH_CAP: f64 = 0.025;
/// 既定の要素の大きさを決める「最短の辺を何分割するか」。
pub const DEFAULT_MESH_DIVISIONS: f64 = 4.0;
/// 辺・面の位置を一致とみなす既定の許容差 [m]。
pub const DEFAULT_MERGE_TOLERANCE: f64 = 1.0e-3;

/// 1 枚のパネルの格子。局所の座標 (u, v, n) の点の機体座標は `origin + u·frame[0] + v·frame[1] + n·frame[2]`。
#[derive(Debug, Clone)]
pub struct PanelMesh {
    pub id: String,
    /// 局所の軸 (u, v, n) の機体座標での単位ベクトル（(u, v, n) は右手系）。
    /// 軸に平行なパネルでは座標軸の巡回の並べ替え（法線 n、u = n+1、v = n+2）
    pub frame: [[f64; 3]; 3],
    /// 局所の原点の機体座標（軸に平行なパネルは中央面の位置だけを法線の成分に持つ。斜めのパネルは中心）
    pub origin: [f64; 3],
    /// 面内の格子線の局所座標（u 方向、v 方向。昇順）
    pub lines: [Vec<f64>; 2],
    /// 節点の番号（`nodes[i * lines[1].len() + j]` が u の i 番目、v の j 番目）
    pub nodes: Vec<usize>,
    /// 元の入力の添字
    pub source: usize,
    /// 機体の軸に沿わないパネルか
    pub skewed: bool,
}

impl PanelMesh {
    /// 格子の面内の節点数（u, v）。
    pub fn dims(&self) -> (usize, usize) {
        (self.lines[0].len(), self.lines[1].len())
    }

    /// (i, j) 番目の節点。
    pub fn node(&self, i: usize, j: usize) -> usize {
        self.nodes[i * self.lines[1].len() + j]
    }

    /// 面内の範囲 [u0, u1, v0, v1]（局所座標）。
    pub fn extent(&self) -> [f64; 4] {
        let (u, v) = (&self.lines[0], &self.lines[1]);
        [u[0], u[u.len() - 1], v[0], v[v.len() - 1]]
    }

    /// 面積。
    pub fn area(&self) -> f64 {
        let [u0, u1, v0, v1] = self.extent();
        (u1 - u0) * (v1 - v0)
    }

    /// 機体座標の点の局所座標 (u, v, 面からの距離 n)。
    pub fn local(&self, pos: [f64; 3]) -> [f64; 3] {
        std::array::from_fn(|i| {
            (0..3)
                .map(|k| (pos[k] - self.origin[k]) * self.frame[i][k])
                .sum()
        })
    }
}

/// 要素 1 つ（長方形。節点は局所 (u, v) の (−,−), (+,−), (+,+), (−,+) の順）。
#[derive(Debug, Clone, Copy)]
pub struct Element {
    /// パネルの添字（`Mesh::panels`）
    pub panel: usize,
    /// u 方向・v 方向の辺の長さ
    pub a: f64,
    pub b: f64,
    pub nodes: [usize; 4],
}

/// パネルの組（節点を共有する 2 枚）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Joint {
    pub a: usize,
    pub b: usize,
    pub shared_nodes: usize,
}

/// 斜めのパネルの 4 辺のつながり（辺の節点がすべて別のパネルと共有なら「つながった」、一部なら「一部」、無ければ「つながらない」）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkewedJoin {
    /// パネルの添字（`Mesh::panels`）
    pub panel: usize,
    pub joined: usize,
    pub partly: usize,
    pub unjoined: usize,
}

/// フレームの部材の辺 1 本（フレームの箱の辺。節点が 2 つ以上あるもの）。
#[derive(Debug, Clone)]
pub struct MemberEdge {
    /// 辺を作る 2 つの面（軸 b = a+1 の面、軸 c = a+2 の面の順）
    pub faces: [ModalFemFace; 2],
    /// 辺の方向の軸
    pub axis: usize,
    /// 辺の位置（軸 a+1 と a+2 の座標）
    pub fixed: [f64; 2],
    /// 辺の上の節点（辺に沿った座標の昇順）
    pub nodes: Vec<(f64, usize)>,
}

/// フレーム 1 つの部材。
#[derive(Debug, Clone)]
pub struct FrameMesh {
    /// 入力の `frames` の添字
    pub source: usize,
    pub id: String,
    pub edges: Vec<MemberEdge>,
    /// 部材にならなかった辺（節点が 2 つ未満）
    pub missing: Vec<[ModalFemFace; 2]>,
}

/// 梁要素 1 つ（フレームの部材の隣り合う 2 節点）。
#[derive(Debug, Clone, Copy)]
pub struct Member {
    /// `Mesh::frames` の添字
    pub frame: usize,
    /// 部材の方向の軸
    pub axis: usize,
    pub nodes: [usize; 2],
    pub length: f64,
}

/// 全体のメッシュ。
#[derive(Debug, Clone)]
pub struct Mesh {
    /// 節点の座標（節点番号順。長い軸の順に並べてスカイラインの幅を抑える）
    pub coords: Vec<[f64; 3]>,
    pub panels: Vec<PanelMesh>,
    pub joints: Vec<Joint>,
    /// 斜めのパネルのつながり
    pub skewed_joins: Vec<SkewedJoin>,
    pub frames: Vec<FrameMesh>,
    /// 実際に使った要素の大きさ [m]
    pub mesh_size: f64,
}

impl Mesh {
    /// 節点数。
    pub fn node_count(&self) -> usize {
        self.coords.len()
    }

    /// シェル要素の数。
    pub fn element_count(&self) -> usize {
        self.panels
            .iter()
            .map(|p| (p.lines[0].len() - 1) * (p.lines[1].len() - 1))
            .sum()
    }

    /// すべてのシェル要素（パネル順、u の添字が外側）。
    pub fn elements(&self) -> impl Iterator<Item = Element> + '_ {
        self.panels.iter().enumerate().flat_map(|(pi, p)| {
            let (nu, nv) = p.dims();
            (0..nu - 1).flat_map(move |i| {
                (0..nv - 1).map(move |j| Element {
                    panel: pi,
                    a: p.lines[0][i + 1] - p.lines[0][i],
                    b: p.lines[1][j + 1] - p.lines[1][j],
                    nodes: [
                        p.node(i, j),
                        p.node(i + 1, j),
                        p.node(i + 1, j + 1),
                        p.node(i, j + 1),
                    ],
                })
            })
        })
    }

    /// すべての梁要素（フレーム順、辺の順、辺に沿った順）。
    pub fn members(&self) -> impl Iterator<Item = Member> + '_ {
        self.frames.iter().enumerate().flat_map(|(fi, f)| {
            f.edges.iter().flat_map(move |e| {
                e.nodes.windows(2).map(move |w| Member {
                    frame: fi,
                    axis: e.axis,
                    nodes: [w[0].1, w[1].1],
                    length: w[1].0 - w[0].0,
                })
            })
        })
    }

    /// 節点の座標の範囲 (最小, 最大)。
    pub fn bounds(&self) -> ([f64; 3], [f64; 3]) {
        bounds_of(&self.coords)
    }
}

fn invalid(msg: String) -> AnalysisError {
    AnalysisError::InvalidInput(msg)
}

/// 入力からメッシュを作る。
pub fn build(input: &ModalFemInput) -> Result<Mesh, AnalysisError> {
    let tol = input
        .merge_tolerance
        .map_or(DEFAULT_MERGE_TOLERANCE, |t| t.value());
    let h = input
        .mesh_size
        .map_or_else(|| grid::default_mesh_size(&input.panels), |m| m.value());
    let (aligned, tilted): (Vec<grid::Aligned>, Vec<grid::Aligned>) = input
        .panels
        .iter()
        .enumerate()
        .partition(|(_, p)| p.orientation.is_none());
    let grid::AlignedMesh {
        mut coords,
        mut panels,
    } = grid::build(&aligned, h, tol)?;
    if !tilted.is_empty() {
        panels.extend(skewed::add(&tilted, h, tol, &mut coords)?);
        order_nodes(&mut coords, &mut panels, tol);
    }
    let joints = joints(&panels, coords.len());
    let skewed_joins = skewed_joins(&panels, coords.len());
    let frames = input
        .frames
        .iter()
        .enumerate()
        .map(|(source, f)| {
            let (edges, missing) = members::edges(f, &coords, tol);
            FrameMesh {
                source,
                id: f.id.clone(),
                edges,
                missing,
            }
        })
        .collect();
    Ok(Mesh {
        coords,
        panels,
        joints,
        skewed_joins,
        frames,
        mesh_size: h,
    })
}

/// 斜めのパネルを足したあとの節点の並べ直し（箱の長い軸の順。許容差ごとに丸めた座標で並べ、節点の番号を付け替える）。
fn order_nodes(coords: &mut Vec<[f64; 3]>, panels: &mut [PanelMesh], tol: f64) {
    let (lo, hi) = bounds_of(coords);
    let mut axes = [0, 1, 2];
    axes.sort_by(|a, b| {
        (hi[*b] - lo[*b])
            .total_cmp(&(hi[*a] - lo[*a]))
            .then(a.cmp(b))
    });
    let step = tol.max(1.0e-9);
    let key = |c: &[f64; 3]| axes.map(|a| (c[a] / step).round() as i64);
    let mut order: Vec<usize> = (0..coords.len()).collect();
    order.sort_by_key(|i| key(&coords[*i]));
    let mut new_id = vec![0; coords.len()];
    for (new, old) in order.iter().enumerate() {
        new_id[*old] = new;
    }
    *coords = order.iter().map(|i| coords[*i]).collect();
    for p in panels {
        for n in &mut p.nodes {
            *n = new_id[*n];
        }
    }
}

fn bounds_of(coords: &[[f64; 3]]) -> ([f64; 3], [f64; 3]) {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for c in coords {
        for a in 0..3 {
            lo[a] = lo[a].min(c[a]);
            hi[a] = hi[a].max(c[a]);
        }
    }
    (lo, hi)
}

/// 節点を持つパネルの一覧（節点ごと）。
fn owners(panels: &[PanelMesh], node_count: usize) -> Vec<Vec<usize>> {
    let mut owners: Vec<Vec<usize>> = vec![Vec::new(); node_count];
    for (pi, p) in panels.iter().enumerate() {
        for &n in &p.nodes {
            owners[n].push(pi);
        }
    }
    owners
}

/// 節点を共有するパネルの組（共有する節点の数つき）。
fn joints(panels: &[PanelMesh], node_count: usize) -> Vec<Joint> {
    let mut pairs: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    for o in &owners(panels, node_count) {
        for (k, &a) in o.iter().enumerate() {
            for &b in &o[k + 1..] {
                *pairs.entry((a.min(b), a.max(b))).or_insert(0) += 1;
            }
        }
    }
    pairs
        .into_iter()
        .map(|((a, b), shared_nodes)| Joint { a, b, shared_nodes })
        .collect()
}

/// 斜めのパネルの辺ごとのつながり。
fn skewed_joins(panels: &[PanelMesh], node_count: usize) -> Vec<SkewedJoin> {
    let owners = owners(panels, node_count);
    let mut out = Vec::new();
    for (pi, p) in panels.iter().enumerate().filter(|(_, p)| p.skewed) {
        let (nu, nv) = p.dims();
        let edges: [Vec<usize>; 4] = [
            (0..nu).map(|i| p.node(i, 0)).collect(),
            (0..nv).map(|j| p.node(nu - 1, j)).collect(),
            (0..nu).map(|i| p.node(i, nv - 1)).collect(),
            (0..nv).map(|j| p.node(0, j)).collect(),
        ];
        let mut join = SkewedJoin {
            panel: pi,
            joined: 0,
            partly: 0,
            unjoined: 0,
        };
        for edge in edges {
            let shared = edge
                .iter()
                .filter(|n| owners[**n].iter().any(|o| *o != pi))
                .count();
            match shared {
                0 => join.unjoined += 1,
                s if s == edge.len() => join.joined += 1,
                _ => join.partly += 1,
            }
        }
        out.push(join);
    }
    out
}
