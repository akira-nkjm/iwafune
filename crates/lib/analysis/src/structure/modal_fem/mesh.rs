//! メッシュ。軸ごとに 1 つの共通の格子を作り、パネルの節点をその格子の点に置く。
//! 共有する辺・T 字の接合では節点が同じ格子点になるので、パネルは節点で剛につながる。

use super::input::{ModalFemInput, ModalFemPanel};
use crate::spec::AnalysisError;
use std::collections::{BTreeMap, HashMap, HashSet};

/// 既定の要素の大きさの上限 [m]。
pub const DEFAULT_MESH_CAP: f64 = 0.025;
/// 既定の要素の大きさを決める「最短の辺を何分割するか」。
pub const DEFAULT_MESH_DIVISIONS: f64 = 4.0;
/// 区間を要素の大きさで割った値の切り上げで、丸めの誤差を吸収する幅（要素の数に対する比）。
const DIVISION_SLACK: f64 = 1.0e-6;
/// 辺・面の位置を一致とみなす既定の許容差 [m]。
pub const DEFAULT_MERGE_TOLERANCE: f64 = 1.0e-3;

/// 1 枚のパネルの格子。
#[derive(Debug, Clone)]
pub struct PanelMesh {
    pub id: String,
    /// 法線の軸
    pub normal: usize,
    /// 面内の軸 (u, v)。法線から巡回順（u = n+1, v = n+2）なので (u, v, n) は右手系
    pub axes: [usize; 2],
    /// 中央面の位置（法線方向の座標）
    pub plane: f64,
    /// 面内の格子線の座標（u 方向、v 方向。昇順）
    pub lines: [Vec<f64>; 2],
    /// 節点の番号（`nodes[i * lines[1].len() + j]` が u の i 番目、v の j 番目）
    pub nodes: Vec<usize>,
    /// 元の入力の添字
    pub source: usize,
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

    /// 面内の範囲 [u0, u1, v0, v1]。
    pub fn extent(&self) -> [f64; 4] {
        let (u, v) = (&self.lines[0], &self.lines[1]);
        [u[0], u[u.len() - 1], v[0], v[v.len() - 1]]
    }

    /// 面積。
    pub fn area(&self) -> f64 {
        let [u0, u1, v0, v1] = self.extent();
        (u1 - u0) * (v1 - v0)
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

/// 全体のメッシュ。
#[derive(Debug, Clone)]
pub struct Mesh {
    /// 節点の座標（節点番号順。長い軸の順に並べてスカイラインの幅を抑える）
    pub coords: Vec<[f64; 3]>,
    pub panels: Vec<PanelMesh>,
    pub joints: Vec<Joint>,
    /// 実際に使った要素の大きさ [m]
    pub mesh_size: f64,
}

impl Mesh {
    /// 節点数。
    pub fn node_count(&self) -> usize {
        self.coords.len()
    }

    /// 要素数。
    pub fn element_count(&self) -> usize {
        self.panels
            .iter()
            .map(|p| (p.lines[0].len() - 1) * (p.lines[1].len() - 1))
            .sum()
    }

    /// すべての要素（パネル順、u の添字が外側）。
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

    /// 節点の座標の範囲 (最小, 最大)。
    pub fn bounds(&self) -> ([f64; 3], [f64; 3]) {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for c in &self.coords {
            for a in 0..3 {
                lo[a] = lo[a].min(c[a]);
                hi[a] = hi[a].max(c[a]);
            }
        }
        (lo, hi)
    }
}

fn invalid(msg: String) -> AnalysisError {
    AnalysisError::InvalidInput(msg)
}

pub use crate::plate_grid::in_plane_axes;

/// 面内の軸 `axis` 方向の辺の長さ（入力の `size` は昇順の 2 軸の順）。
fn side_along(p: &ModalFemPanel, axis: usize) -> f64 {
    let n = p.normal.index();
    let rank = (0..3)
        .filter(|a| *a != n)
        .position(|a| a == axis)
        .unwrap_or(0);
    p.size[rank].value()
}

/// 値の列を、隣との差が許容差以下のものどうしで束ねた代表値（昇順）にする。
fn cluster(mut values: Vec<f64>, tol: f64) -> Vec<f64> {
    values.sort_by(f64::total_cmp);
    let mut reps = Vec::new();
    let mut group: Vec<f64> = Vec::new();
    for v in values {
        if group.last().is_some_and(|last| v - last > tol) {
            reps.push(group.iter().sum::<f64>() / group.len() as f64);
            group.clear();
        }
        group.push(v);
    }
    if !group.is_empty() {
        reps.push(group.iter().sum::<f64>() / group.len() as f64);
    }
    reps
}

/// 代表値のうち `x` に最も近いものの添字。
fn nearest(reps: &[f64], x: f64) -> usize {
    reps.iter()
        .enumerate()
        .min_by(|a, b| (a.1 - x).abs().total_cmp(&(b.1 - x).abs()))
        .map_or(0, |(i, _)| i)
}

/// 代表値の区間を `h` 以下に等分した格子線と、代表値ごとの格子線の添字。
fn axis_grid(reps: &[f64], h: f64) -> (Vec<f64>, Vec<usize>) {
    let mut lines = vec![reps[0]];
    let mut at = vec![0];
    for w in reps.windows(2) {
        // 丸め（2.0000000000000004 など）で分割が 1 つ増えないよう、要素 1 つ分の 1e-6 だけ引いて切り上げる。
        let n = ((w[1] - w[0]) / h - DIVISION_SLACK).ceil().max(1.0) as usize;
        for k in 1..=n {
            lines.push(w[0] + (w[1] - w[0]) * k as f64 / n as f64);
        }
        at.push(lines.len() - 1);
    }
    (lines, at)
}

/// 既定の要素の大きさ min(最短の辺 / 4, 25 mm)。
fn default_mesh_size(panels: &[ModalFemPanel]) -> f64 {
    let shortest = panels
        .iter()
        .flat_map(|p| p.size.iter().map(|s| s.value()))
        .fold(f64::INFINITY, f64::min);
    (shortest / DEFAULT_MESH_DIVISIONS).min(DEFAULT_MESH_CAP)
}

/// 軸ごとの格子線と、パネルの端・面の位置が格子線の何番目か。
struct Grid {
    lines: [Vec<f64>; 3],
    /// 軸ごとの、代表値の添字 → 格子線の添字
    at: [Vec<usize>; 3],
    reps: [Vec<f64>; 3],
}

fn build_grid(input: &ModalFemInput, h: f64, tol: f64) -> Grid {
    let mut raw: [Vec<f64>; 3] = Default::default();
    for p in &input.panels {
        let n = p.normal.index();
        raw[n].push(p.center[n].value());
        for axis in in_plane_axes(n) {
            let (c, half) = (p.center[axis].value(), side_along(p, axis) / 2.0);
            raw[axis].extend([c - half, c + half]);
        }
    }
    let reps: [Vec<f64>; 3] = raw.map(|v| cluster(v, tol));
    let built: Vec<(Vec<f64>, Vec<usize>)> = reps.iter().map(|r| axis_grid(r, h)).collect();
    let mut it = built.into_iter();
    let mut take = || it.next().unwrap_or_default();
    let (l0, a0) = take();
    let (l1, a1) = take();
    let (l2, a2) = take();
    Grid {
        lines: [l0, l1, l2],
        at: [a0, a1, a2],
        reps,
    }
}

/// パネルが占める格子点（`[軸 0, 軸 1, 軸 2]` の格子添字）の範囲を返す。
fn panel_cells(grid: &Grid, p: &ModalFemPanel) -> ([usize; 3], [usize; 2], [usize; 2]) {
    let n = p.normal.index();
    let [u, v] = in_plane_axes(n);
    let idx = |axis: usize, x: f64| grid.at[axis][nearest(&grid.reps[axis], x)];
    let range = |axis: usize| {
        let (c, half) = (p.center[axis].value(), side_along(p, axis) / 2.0);
        [idx(axis, c - half), idx(axis, c + half)]
    };
    let mut base = [0; 3];
    base[n] = idx(n, p.center[n].value());
    (base, range(u), range(v))
}

/// 入力からメッシュを作る。
pub fn build(input: &ModalFemInput) -> Result<Mesh, AnalysisError> {
    let tol = input
        .merge_tolerance
        .map_or(DEFAULT_MERGE_TOLERANCE, |t| t.value());
    let h = input
        .mesh_size
        .map_or_else(|| default_mesh_size(&input.panels), |m| m.value());
    let grid = build_grid(input, h, tol);
    let mut keys: HashSet<[usize; 3]> = HashSet::new();
    let mut cells: Vec<([usize; 3], [usize; 2], [usize; 2])> = Vec::new();
    for p in &input.panels {
        let c = panel_cells(&grid, p);
        let (base, ur, vr) = c;
        let [u, v] = in_plane_axes(p.normal.index());
        for i in ur[0]..=ur[1] {
            for j in vr[0]..=vr[1] {
                let mut k = base;
                (k[u], k[v]) = (i, j);
                keys.insert(k);
            }
        }
        cells.push(c);
    }
    let order = axis_priority(&grid);
    let mut sorted: Vec<[usize; 3]> = keys.into_iter().collect();
    sorted.sort_by_key(|k| order.map(|a| k[a]));
    let id_of: HashMap<[usize; 3], usize> =
        sorted.iter().enumerate().map(|(i, k)| (*k, i)).collect();
    let coords = sorted
        .iter()
        .map(|k| {
            [
                grid.lines[0][k[0]],
                grid.lines[1][k[1]],
                grid.lines[2][k[2]],
            ]
        })
        .collect();
    let panels = make_panels(input, &grid, &cells, &id_of)?;
    check_overlap(&panels, &cells)?;
    let joints = joints(&panels, sorted.len());
    Ok(Mesh {
        coords,
        panels,
        joints,
        mesh_size: h,
    })
}

/// 節点の並べ方の軸の優先順（箱の長い軸が先。ties は軸の番号順）。
fn axis_priority(grid: &Grid) -> [usize; 3] {
    let len = |a: usize| grid.lines[a][grid.lines[a].len() - 1] - grid.lines[a][0];
    let mut order = [0, 1, 2];
    order.sort_by(|a, b| len(*b).total_cmp(&len(*a)).then(a.cmp(b)));
    order
}

fn make_panels(
    input: &ModalFemInput,
    grid: &Grid,
    cells: &[([usize; 3], [usize; 2], [usize; 2])],
    id_of: &HashMap<[usize; 3], usize>,
) -> Result<Vec<PanelMesh>, AnalysisError> {
    let mut out = Vec::new();
    for (source, (p, (base, ur, vr))) in input.panels.iter().zip(cells).enumerate() {
        let n = p.normal.index();
        let [u, v] = in_plane_axes(n);
        if ur[1] <= ur[0] || vr[1] <= vr[0] {
            return Err(invalid(format!(
                "{}: the panel collapses to a line or a point on the merged grid (size below the merge tolerance)",
                p.id
            )));
        }
        let mut nodes = Vec::new();
        for i in ur[0]..=ur[1] {
            for j in vr[0]..=vr[1] {
                let mut k = *base;
                (k[u], k[v]) = (i, j);
                nodes.push(id_of[&k]);
            }
        }
        out.push(PanelMesh {
            id: p.id.clone(),
            normal: n,
            axes: [u, v],
            plane: grid.lines[n][base[n]],
            lines: [
                grid.lines[u][ur[0]..=ur[1]].to_vec(),
                grid.lines[v][vr[0]..=vr[1]].to_vec(),
            ],
            nodes,
            source,
        });
    }
    Ok(out)
}

/// 同じ平面の同じ要素を 2 枚のパネルが占めていないか（剛性の二重計上）。
fn check_overlap(
    panels: &[PanelMesh],
    cells: &[([usize; 3], [usize; 2], [usize; 2])],
) -> Result<(), AnalysisError> {
    let mut seen: HashMap<(usize, usize, usize, usize), usize> = HashMap::new();
    for (pi, (p, (base, ur, vr))) in panels.iter().zip(cells).enumerate() {
        for i in ur[0]..ur[1] {
            for j in vr[0]..vr[1] {
                if let Some(other) = seen.insert((p.normal, base[p.normal], i, j), pi) {
                    return Err(invalid(format!(
                        "Panels {} and {} overlap in the same plane",
                        panels[other].id, p.id
                    )));
                }
            }
        }
    }
    Ok(())
}

/// 節点を共有するパネルの組（共有する節点の数つき）。
fn joints(panels: &[PanelMesh], node_count: usize) -> Vec<Joint> {
    let mut owners: Vec<Vec<usize>> = vec![Vec::new(); node_count];
    for (pi, p) in panels.iter().enumerate() {
        for &n in &p.nodes {
            owners[n].push(pi);
        }
    }
    let mut pairs: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    for o in &owners {
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
