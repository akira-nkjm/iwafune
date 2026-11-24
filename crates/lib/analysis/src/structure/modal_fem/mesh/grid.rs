//! 軸に平行なパネルのメッシュ。軸ごとに 1 つの共通の格子を作り、パネルの節点をその格子の点に置く。
//! 共有する辺・T 字の接合では節点が同じ格子点になるので、パネルは節点で剛につながる。

use super::{PanelMesh, in_plane_axes, invalid};
use crate::spec::AnalysisError;
use crate::structure::modal_fem::input::ModalFemPanel;
use std::collections::{HashMap, HashSet};

/// 区間を要素の大きさで割った値の切り上げで、丸めの誤差を吸収する幅（要素の数に対する比）。
pub(super) const DIVISION_SLACK: f64 = 1.0e-6;

/// 軸に平行なパネル（入力の添字つき）。
pub(super) type Aligned<'a> = (usize, &'a ModalFemPanel);

/// 軸に平行なパネルの節点（長い軸の順に並べた座標）とパネル。
pub(super) struct AlignedMesh {
    pub coords: Vec<[f64; 3]>,
    pub panels: Vec<PanelMesh>,
}

/// 面内の軸 `axis` 方向の辺の長さ（入力の `size` は昇順の 2 軸の順）。
pub(super) fn side_along(p: &ModalFemPanel, axis: usize) -> f64 {
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
pub(super) fn default_mesh_size(panels: &[ModalFemPanel]) -> f64 {
    let shortest = panels
        .iter()
        .flat_map(|p| p.size.iter().map(|s| s.value()))
        .fold(f64::INFINITY, f64::min);
    (shortest / super::DEFAULT_MESH_DIVISIONS).min(super::DEFAULT_MESH_CAP)
}

/// 軸ごとの格子線と、パネルの端・面の位置が格子線の何番目か。
struct Grid {
    lines: [Vec<f64>; 3],
    /// 軸ごとの、代表値の添字 → 格子線の添字
    at: [Vec<usize>; 3],
    reps: [Vec<f64>; 3],
}

fn build_grid(panels: &[Aligned], h: f64, tol: f64) -> Grid {
    let mut raw: [Vec<f64>; 3] = Default::default();
    for (_, p) in panels {
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

/// パネルが占める格子点の範囲（法線の軸の格子添字、面内の 2 軸の格子添字の範囲）。
type Cells = ([usize; 3], [usize; 2], [usize; 2]);

fn panel_cells(grid: &Grid, p: &ModalFemPanel) -> Cells {
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

/// 軸に平行なパネルのメッシュを作る（共通の格子の点が節点。節点は長い軸の順）。
pub(super) fn build(panels: &[Aligned], h: f64, tol: f64) -> Result<AlignedMesh, AnalysisError> {
    if panels.is_empty() {
        return Ok(AlignedMesh {
            coords: Vec::new(),
            panels: Vec::new(),
        });
    }
    let grid = build_grid(panels, h, tol);
    let mut keys: HashSet<[usize; 3]> = HashSet::new();
    let mut cells: Vec<Cells> = Vec::new();
    for (_, p) in panels {
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
    let out = make_panels(panels, &grid, &cells, &id_of)?;
    check_overlap(&out, &cells)?;
    Ok(AlignedMesh {
        coords,
        panels: out,
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
    panels: &[Aligned],
    grid: &Grid,
    cells: &[Cells],
    id_of: &HashMap<[usize; 3], usize>,
) -> Result<Vec<PanelMesh>, AnalysisError> {
    let mut out = Vec::new();
    for ((source, p), (base, ur, vr)) in panels.iter().zip(cells) {
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
        let unit = |a: usize| {
            let mut e = [0.0; 3];
            e[a] = 1.0;
            e
        };
        let mut origin = [0.0; 3];
        origin[n] = grid.lines[n][base[n]];
        out.push(PanelMesh {
            id: p.id.clone(),
            frame: [unit(u), unit(v), unit(n)],
            origin,
            lines: [
                grid.lines[u][ur[0]..=ur[1]].to_vec(),
                grid.lines[v][vr[0]..=vr[1]].to_vec(),
            ],
            nodes,
            source: *source,
            skewed: false,
        });
    }
    Ok(out)
}

/// 同じ平面の同じ要素を 2 枚のパネルが占めていないか（剛性の二重計上）。
fn check_overlap(panels: &[PanelMesh], cells: &[Cells]) -> Result<(), AnalysisError> {
    let mut seen: HashMap<(usize, usize, usize, usize), usize> = HashMap::new();
    for (pi, (p, (base, ur, vr))) in panels.iter().zip(cells).enumerate() {
        let normal = p.frame[2].iter().position(|c| *c == 1.0).unwrap_or(0);
        for i in ur[0]..ur[1] {
            for j in vr[0]..vr[1] {
                if let Some(other) = seen.insert((normal, base[normal], i, j), pi) {
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
