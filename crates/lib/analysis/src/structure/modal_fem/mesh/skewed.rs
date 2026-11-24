//! 機体の軸に沿わないパネル（`orientation` を持つもの）のメッシュ。パネルごとに自分の面内の格子（均等な升）を作り、
//! 節点をパネルの姿勢で機体座標に置く。節点は、すでにある節点（軸に平行なパネルや別の斜めのパネルの）と
//! 許容差の内で一致すれば同じ節点にする（共有する辺で格子が合えばつながる）。合わない節点どうしは、つながらない。
//! 辺が別のパネルの面の途中に当たる接合（節点が面の上にあって、相手の節点と合わない）は扱わない（辺ごとのつながりを出力に出す）。

use super::grid::{Aligned, DIVISION_SLACK, side_along};
use super::{PanelMesh, in_plane_axes, invalid};
use crate::spec::AnalysisError;
use sat_astro::math::{Quat, Vec3};
use std::collections::HashMap;

/// 節点の位置の索引（許容差の内の節点を探す。格子の升の大きさ = 許容差）。
pub(super) struct NodeIndex {
    cell: f64,
    tol: f64,
    cells: HashMap<[i64; 3], Vec<usize>>,
}

impl NodeIndex {
    /// 既存の節点から作る。
    pub fn new(coords: &[[f64; 3]], tol: f64) -> Self {
        let mut s = Self {
            cell: tol.max(1.0e-9),
            tol,
            cells: HashMap::new(),
        };
        for (i, c) in coords.iter().enumerate() {
            s.insert(i, *c);
        }
        s
    }

    fn key(&self, c: [f64; 3]) -> [i64; 3] {
        c.map(|x| (x / self.cell).floor() as i64)
    }

    fn insert(&mut self, id: usize, c: [f64; 3]) {
        let key = self.key(c);
        self.cells.entry(key).or_default().push(id);
    }

    /// 許容差の内で最も近い節点（無ければ `None`）。
    fn find(&self, coords: &[[f64; 3]], c: [f64; 3]) -> Option<usize> {
        let k = self.key(c);
        let mut best: Option<(f64, usize)> = None;
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let key = [k[0] + dx, k[1] + dy, k[2] + dz];
                    for &id in self.cells.get(&key).into_iter().flatten() {
                        let d = (0..3).map(|a| (coords[id][a] - c[a]).powi(2)).sum::<f64>();
                        if d.sqrt() <= self.tol && best.is_none_or(|b| d < b.0) {
                            best = Some((d, id));
                        }
                    }
                }
            }
        }
        best.map(|b| b.1)
    }

    /// 位置の節点の番号（許容差の内にあるものを使い、無ければ `coords` に足す）。
    fn get_or_add(&mut self, coords: &mut Vec<[f64; 3]>, c: [f64; 3]) -> usize {
        if let Some(id) = self.find(coords, c) {
            return id;
        }
        coords.push(c);
        let id = coords.len() - 1;
        self.insert(id, c);
        id
    }
}

/// 箱の座標系の単位ベクトル（箱の軸 `a` の、機体座標での向き）。
fn box_axis(q: Quat, a: usize) -> [f64; 3] {
    let e = [Vec3::X, Vec3::Y, Vec3::Z][a];
    let v = q.rotate(e);
    [v.x, v.y, v.z]
}

/// 局所の軸 (u, v, n) の機体座標での向き。箱の法線の軸 `n` から巡回順（u = n+1, v = n+2）なので右手系。
fn local_frame(orientation: [f64; 4], n: usize) -> [[f64; 3]; 3] {
    let q = Quat::from_wxyz(orientation).normalized();
    let [u, v] = in_plane_axes(n);
    [box_axis(q, u), box_axis(q, v), box_axis(q, n)]
}

/// 斜めのパネルを節点に分け、`coords` に節点を足す（一致する節点は共有する）。
pub(super) fn add(
    panels: &[Aligned],
    h: f64,
    tol: f64,
    coords: &mut Vec<[f64; 3]>,
) -> Result<Vec<PanelMesh>, AnalysisError> {
    let mut index = NodeIndex::new(coords, tol);
    let mut out = Vec::new();
    for (source, p) in panels {
        let Some(q) = p.orientation else { continue };
        if !q.iter().all(|x| x.is_finite()) || q.iter().all(|x| *x == 0.0) {
            return Err(invalid(format!(
                "{}: the orientation is not a rotation",
                p.id
            )));
        }
        let n = p.normal.index();
        let [u, v] = in_plane_axes(n);
        let size = [side_along(p, u), side_along(p, v)];
        if size.iter().any(|s| *s <= tol) {
            return Err(invalid(format!(
                "{}: the panel collapses to a line or a point (size below the merge tolerance)",
                p.id
            )));
        }
        let frame = local_frame(q, n);
        let center = p.center.map(|c| c.value());
        let lines = size.map(|len| {
            let cells = ((len / h - DIVISION_SLACK).ceil().max(1.0)) as usize;
            (0..=cells)
                .map(|k| -len / 2.0 + len * k as f64 / cells as f64)
                .collect::<Vec<f64>>()
        });
        let mut nodes = Vec::with_capacity(lines[0].len() * lines[1].len());
        for &a in &lines[0] {
            for &b in &lines[1] {
                let pt = std::array::from_fn(|k| center[k] + a * frame[0][k] + b * frame[1][k]);
                nodes.push(index.get_or_add(coords, pt));
            }
        }
        out.push(PanelMesh {
            id: p.id.clone(),
            frame,
            origin: center,
            lines,
            nodes,
            source: *source,
            skewed: true,
        });
    }
    Ok(out)
}
