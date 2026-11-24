//! 板の形の部品を面内の格子の節点（升）に分ける。**板の中の温度分布のモデル（升の作り方・面内の伝導・結合の配り方）はここだけ。**
//! 解法（`Compiled` 以降）は変えない。`Compiled::new` が、解く前の組み立ての段階でここの [`Plan`] を使って節点を升に置き換える。
//!
//! - 格子は `plate_grid::PlateGrid`（粗い有限要素と共有）。法線は箱の最も薄い軸で、面内の (u, v) は法線から巡回順。
//! - 升の節点の ID は `<部品>[i,j]`（0 始まり、u が先）。設計に書く名前は部品のまま（結合・発熱の場所・要求の key）。
//! - 面内の伝導は隣り合う升の間に `G = k · t · (共有する辺の長さ) / (中心間の距離)`（k は材料の熱伝導率、t は板厚 = 最も薄い辺）。
//! - 升の熱容量 = 部品の質量 × 升の面積の割合 × 員数 × 比熱。外面（入熱・宇宙への放射）と内部発熱は升の面積の割合で配る。
//! - 部品全体への結合（明示・外殻・取付の既定値）は、解いた対ごとの値を升に配る。明示の結合と放射は升の面積の割合、取付の伝導は
//!   相手の箱と各升の接触面積の割合（`OrientedBox::contact_area` を升の箱に当てる。箱が許容差の外で接していなければ、相手の箱を板の面に
//!   射影した形と升の重なりの面積）。どちらも決められない対は升の面積の割合で配り、
//!   出力の `spread_by_area` に残す。配った合計は元の値と同じ（保存）。
//! - 板に置けないものは入力の誤り: 板でない箱（最も薄い辺が次に薄い辺の 1/4 を超える）、[`MAX_CELLS`] を超える升の数、
//!   部品が節点にまとめられている、固定温度の節点、熱伝導率が正でない。サーモスタット付きのヒータは単一の節点にしか置けない。

use super::{ThermalNode, ThermalNodePart};
use crate::plate_grid::PlateGrid;
use crate::spec::AnalysisError;
use crate::structure::interference::InterferenceBox;
use crate::thermal::cell::ThermalCell;
use sat_units::typed::{Length, Qty, ThermalConductance, ThermalConductivity, Time};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// 1 枚の板を 1 軸に沿って分ける升の数の上限。陽解法の非定常は、升が細かいほど最小の時定数（板厚 × 升の寸法の二乗に比例）に
/// 刻みが縛られて費用が増えるため。
pub const MAX_CELLS: usize = 8;

/// 板の分け方 1 枚分（`ThermalNetwork::plates`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalPlate {
    /// 分ける部品の ID（その部品の節点 = 同じ名前の節点）
    pub part: String,
    /// 升の数 (u, v)。軸の順は `plate_grid`
    pub cells: [u32; 2],
    /// 材料の熱伝導率
    pub conductivity: Qty<ThermalConductivity>,
    /// 板の箱（機体座標系の位置・姿勢と、部品座標系の辺の長さ）
    pub frame: InterferenceBox,
    /// 板に取り付く（板が取り付く）相手の箱。取付の伝導をどの升に配るかを決める（相手の節点ごと）
    #[serde(default)]
    pub attachments: Vec<ThermalPlateAttachment>,
}

/// 板と取付で結ばれた相手の箱。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalPlateAttachment {
    /// 相手の部品が属する節点
    pub node: String,
    /// 相手の部品の箱
    pub footprint: InterferenceBox,
}

/// 解いた板の報告（出力の `couplings.plates`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalPlateUsed {
    pub part: String,
    /// 升の数 (u, v)
    pub cells: [u32; 2],
    /// 板厚（箱の最も薄い辺）
    pub thickness: Qty<Length>,
    /// u 方向に隣り合う升の間の面内の伝導
    pub conductance_u: Qty<ThermalConductance>,
    /// v 方向に隣り合う升の間の面内の伝導
    pub conductance_v: Qty<ThermalConductance>,
    /// 升の最小の時定数 C / ΣG（その升に付く伝導の合計。放射は含めない）。陽解法の刻みの目安
    pub min_time_constant: Qty<Time>,
    /// 相手の位置が決められず、升の面積の割合で配った結合の相手の節点
    #[serde(default)]
    pub spread_by_area: Vec<String>,
}

/// 板 1 枚の升の割り当て。
#[derive(Debug, Clone)]
pub(crate) struct Group {
    /// 升の節点の添字と面積の割合（升の通し番号の順）
    pub slots: Vec<(usize, f64)>,
    grid: PlateGrid,
    plate: ThermalPlate,
    /// 面内の伝導 (i, j, G)
    inplane: Vec<(usize, usize, f64)>,
}

/// 板を升に置き換えた節点の並びと、板ごとの割り当て。
#[derive(Debug, Clone)]
pub(crate) struct Plan {
    pub nodes: Vec<ThermalNode>,
    pub groups: BTreeMap<String, Group>,
}

fn invalid(msg: String) -> AnalysisError {
    AnalysisError::InvalidInput(msg)
}

impl Plan {
    /// 節点の並びの板を升に置き換える（板が無ければ節点はそのまま）。検査はモジュールの説明。
    pub fn new(nodes: &[ThermalNode], plates: &[ThermalPlate]) -> Result<Self, AnalysisError> {
        for (k, p) in plates.iter().enumerate() {
            if plates[..k].iter().any(|q| q.part == p.part) {
                return Err(invalid(format!("Plate {} is subdivided twice", p.part)));
            }
            if !nodes.iter().any(|n| n.id == p.part) {
                return Err(invalid(format!(
                    "Plate {}: there is no node of that name (a part lumped into a node by [nodes] cannot be subdivided)",
                    p.part
                )));
            }
        }
        let mut out = Vec::new();
        let mut groups = BTreeMap::new();
        for n in nodes {
            match plates.iter().find(|p| p.part == n.id) {
                None => out.push(n.clone()),
                Some(p) => {
                    let group = Group::new(p, n, out.len())?;
                    out.extend(group.cells(n));
                    groups.insert(n.id.clone(), group);
                }
            }
        }
        Ok(Self { nodes: out, groups })
    }

    /// 節点 `id` の（添字, 面積の割合）。板なら升すべて、そうでなければ 1 つ（割合 1）。
    pub fn slots(&self, id: &str) -> Result<Vec<(usize, f64)>, AnalysisError> {
        if let Some(g) = self.groups.get(id) {
            return Ok(g.slots.clone());
        }
        self.nodes
            .iter()
            .position(|n| n.id == id)
            .map(|i| vec![(i, 1.0)])
            .ok_or_else(|| invalid(format!("Thermal node {id} does not exist")))
    }

    /// 結合 `a`–`b` の値 `v` を升の対に配る（板でない端は 1 つの節点）。`geometric` なら板の端は相手の箱との接触面積の割合
    /// （決められなければ面積の割合で配り、`unlocated` に相手を残す）。相手も板なら面積の割合どうしの積。
    pub fn spread(
        &self,
        (a, b): (&str, &str),
        v: f64,
        geometric: bool,
        unlocated: &mut BTreeMap<String, BTreeSet<String>>,
    ) -> Result<Vec<(usize, usize, f64)>, AnalysisError> {
        let both = self.groups.contains_key(a) && self.groups.contains_key(b);
        let mut end = |id: &str, other: &str| -> Result<Vec<(usize, f64)>, AnalysisError> {
            let Some(g) = self.groups.get(id) else {
                return self.slots(id);
            };
            if !geometric || both {
                return Ok(g.slots.clone());
            }
            match g.located(other) {
                Some(w) => Ok(g.slots.iter().zip(w).map(|(s, w)| (s.0, w)).collect()),
                None => {
                    unlocated
                        .entry(id.to_owned())
                        .or_default()
                        .insert(other.to_owned());
                    Ok(g.slots.clone())
                }
            }
        };
        let (wa, wb) = (end(a, b)?, end(b, a)?);
        let mut out = Vec::with_capacity(wa.len() * wb.len());
        for &(i, x) in &wa {
            for &(j, y) in &wb {
                let value = v * x * y;
                if value != 0.0 || (wa.len() == 1 && wb.len() == 1) {
                    out.push((i, j, value));
                }
            }
        }
        Ok(out)
    }

    /// 全部の板の面内の伝導。
    pub fn inplane(&self) -> impl Iterator<Item = (usize, usize, f64)> + '_ {
        self.groups.values().flat_map(|g| g.inplane.iter().copied())
    }

    /// 板ごとの報告。`cond` は使う伝導すべて、`capacity` は節点の熱容量、`unlocated` は [`Plan::spread`] の記録。
    pub fn used(
        &self,
        cond: &[(usize, usize, f64)],
        capacity: &[f64],
        unlocated: &BTreeMap<String, BTreeSet<String>>,
    ) -> Vec<ThermalPlateUsed> {
        let mut sum = vec![0.0; capacity.len()];
        for &(i, j, g) in cond {
            sum[i] += g;
            sum[j] += g;
        }
        self.groups
            .iter()
            .map(|(part, g)| {
                let tau = g
                    .slots
                    .iter()
                    .map(|&(k, _)| capacity[k] / sum[k])
                    .fold(f64::INFINITY, f64::min);
                let (gu, gv) = g.edge_conductances();
                ThermalPlateUsed {
                    part: part.clone(),
                    cells: g.plate.cells,
                    thickness: Qty::from_raw(g.grid.thickness()),
                    conductance_u: Qty::from_raw(gu),
                    conductance_v: Qty::from_raw(gv),
                    min_time_constant: Qty::from_raw(tau),
                    spread_by_area: unlocated
                        .get(part)
                        .map(|s| s.iter().cloned().collect())
                        .unwrap_or_default(),
                }
            })
            .collect()
    }
}

impl Group {
    /// 板 `p`（節点 `n`）の升の割り当て。`first` は最初の升の節点の添字。
    fn new(p: &ThermalPlate, n: &ThermalNode, first: usize) -> Result<Self, AnalysisError> {
        let bad = |why: &str| invalid(format!("Plate {}: {why}", p.part));
        let [cu, cv] = p.cells.map(|c| c as usize);
        if !(1..=MAX_CELLS).contains(&cu) || !(1..=MAX_CELLS).contains(&cv) {
            return Err(bad(&format!(
                "a plate is split into 1 to {MAX_CELLS} cells along each axis (the explicit transient solver takes much smaller steps with more cells)"
            )));
        }
        let size = [p.frame.size.x, p.frame.size.y, p.frame.size.z];
        let grid =
            PlateGrid::new(size, [cu, cv]).ok_or_else(|| bad("the part has no usable box size"))?;
        if !grid.is_plate() {
            return Err(bad(
                "the part is not plate-like (its thinnest side must be at most 1/4 of the next thinnest)",
            ));
        }
        let k = p.conductivity.value();
        if !(k.is_finite() && k > 0.0) {
            return Err(bad("the thermal conductivity must be positive"));
        }
        if n.fixed_temperature.is_some() || n.parts.len() != 1 || n.parts[0].id != p.part {
            return Err(bad(
                "only a node of this one part can be subdivided (not a lumped or fixed-temperature node)",
            ));
        }
        let share = 1.0 / grid.count() as f64;
        let mut group = Self {
            slots: (0..grid.count()).map(|c| (first + c, share)).collect(),
            grid,
            plate: p.clone(),
            inplane: Vec::new(),
        };
        group.inplane = group.edges(k);
        Ok(group)
    }

    /// 隣り合う升の間の伝導（`G = k · t · 共有する辺 / 中心間の距離`）。
    fn edges(&self, k: f64) -> Vec<(usize, usize, f64)> {
        let (gu, gv) = self.edge_conductances_with(k);
        let [nu, nv] = self.grid.cells;
        let mut out = Vec::new();
        for i in 0..nu {
            for j in 0..nv {
                let here = self.slots[self.grid.index(i, j)].0;
                if i + 1 < nu {
                    out.push((here, self.slots[self.grid.index(i + 1, j)].0, gu));
                }
                if j + 1 < nv {
                    out.push((here, self.slots[self.grid.index(i, j + 1)].0, gv));
                }
            }
        }
        out
    }

    fn edge_conductances(&self) -> (f64, f64) {
        self.edge_conductances_with(self.plate.conductivity.value())
    }

    fn edge_conductances_with(&self, k: f64) -> (f64, f64) {
        let [du, dv] = self.grid.cell_size();
        let kt = k * self.grid.thickness();
        (kt * dv / du, kt * du / dv)
    }

    /// 升の節点（部品の質量を面積の割合で分ける。範囲・要求・冗長は部品のまま）。
    fn cells(&self, n: &ThermalNode) -> Vec<ThermalNode> {
        (0..self.grid.count())
            .map(|c| {
                let (i, j) = self.grid.position(c);
                let cell = ThermalCell {
                    part: n.id.clone(),
                    i: i as u32,
                    j: j as u32,
                };
                let share = self.slots[c].1;
                let parts = n
                    .parts
                    .iter()
                    .map(|p| ThermalNodePart {
                        mass: Qty::from_raw(p.mass.value() * share),
                        ..p.clone()
                    })
                    .collect();
                ThermalNode {
                    id: cell.node_id(),
                    parts,
                    per_part: false,
                    fixed_temperature: None,
                    required: n.required.clone(),
                    cell: Some(cell),
                }
            })
            .collect()
    }
}

mod locate;
#[cfg(test)]
mod tests;
