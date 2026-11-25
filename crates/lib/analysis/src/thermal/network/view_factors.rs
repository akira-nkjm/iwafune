//! 内部の部品と構造の殻の内側の放射を、形態係数（遮蔽つき）から出す（`view_factors` を書いたときだけ）。
//! **形態係数から出した節点間の放射の交換面積（面の組 → 節点の対）と、2 つの灰色拡散面の有効放射率の式はここだけ。**
//!
//! 幾何（空洞・面の組・遮蔽・接触の組の除外）は `structure::cavity_views`。ここは、その面積 × 形態係数 `G_ij = A_i F_ij` を節点の対にまとめる。
//!
//! 面の組 `(i, j)` の放射の交換面積は、2 つの灰色拡散面が互いだけを見るときの放射の抵抗網（Oppenheim）:
//! `E_ij = 1 / (ρ_i / (ε_i A_i) + 1 / G_ij + ρ_j / (ε_j A_j))`、`ρ = 1 − ε`。
//! 同じ大きさの面が向き合って `F = 1` なら `A / (1/ε_i + 1/ε_j − 1)`（平行平板）で、外殻のモデル（`enclosure` の Gebhart 係数）の 2 面の閉空間と同じ値。
//! ほかの面での反射（Gebhart 係数の `Σ_k F_ik ρ_k B_kj` の項）は数えない（放射率が高ければ小さい。空洞が閉じていない・接触の組を除くので、
//! 全面の Gebhart 係数の連立は使わない）。放射率は全面で 1 つの値（`view_factors.emissivity`）。
//!
//! 節点の対の値は、2 つの節点に属する部品の面の組の `E_ij` の和（同じ節点の部品どうしは数えない）。向き合う面があるのにすべて遮られた対は 0
//! （取付の既定値を置き換えて切り離す）。向き合う面の組が 1 つも無い対（接する面だけ・空洞に入らない部品）は値を出さず、下の優先順位の値のまま。
//! **板を升に分けた節点**（#456）は、面の組の値（面全体の `E_ij`。升に分けない設計と同じ数）を、升と相手の面の `A_c F_cc'`（`cavity_views` の升の組）
//! の割合で升に配る（`CellMatrix`。配った合計は面全体の値）。板の縁の面（升に分けない面）の組は升の面積の割合で配る。
//! 升に分けない節点の対は、これまでと同じ値だけを出す。

use super::{ThermalNetwork, ThermalNode};
use crate::spec::AnalysisError;
use crate::structure::cavity_views::{CavityViews, CellExchange, cavity_views_split};
use crate::structure::interference::InterferenceBox;
use sat_units::typed::{Area, Qty, Time};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Instant;

/// 内部の放射を形態係数から出すための入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalViewFactors {
    /// 内部の面の赤外の放射率（0 を超え 1 以下。壁と部品の全面で共通）
    pub emissivity: f64,
    /// 寸法を持つ部品の箱（機体座標系。`structure.external_geometry` と同じ）
    pub boxes: Vec<InterferenceBox>,
}

/// 節点対の放射のうち、形態係数から出した分の内訳（出力の `ThermalExchangeUsed::from_view_factors`）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalViewFactorPair {
    /// 面の組の `A_i F_ij` の和（放射率を入れる前の、見える面積）
    pub area_factor: Qty<Area>,
    /// 数えた面の組の数（遮られて 0 の組は数えない）
    pub face_pairs: u32,
}

/// 節点の、面の見え方の収支（品質の指標）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalViewFactorNode {
    pub node: String,
    /// 空洞に面を出す部品の面積の合計
    pub area: Qty<Area>,
    /// `1 − 面積で重みづけした Σ_j F_ij`。閉じた空洞で、接触の組を除かず、求積の誤差が 0 なら 0。
    /// 接触の組を除いた分・重なった箱の中に入った視線・求積の誤差の合計の目安（0 に近いほど、面の見え方を数えきれている）
    pub residual: f64,
}

/// 形態係数のモデルを使ったか、使ったときの要約。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalViewFactorsUsed {
    /// 使ったか（空洞が無ければ使わない）
    pub applied: bool,
    /// 使わなかった理由
    #[serde(default)]
    pub note: Option<String>,
    /// 使った放射率
    pub emissivity: f64,
    /// 空洞を作る容器の部品（箱の殻の内側が壁）
    #[serde(default)]
    pub cavities: Vec<String>,
    /// 空洞に面を出さない部品（容器の外にある。ID の昇順）
    #[serde(default)]
    pub outside: Vec<String>,
    /// 空洞の面の数（壁と部品）
    pub surfaces: u32,
    /// 向き合う面の組の数（接触の組と、遮られて 0 の組を含む）
    pub face_pairs: u32,
    /// 接していて、放射でなく伝導とした面の組の数
    pub contact_pairs: u32,
    /// すべての視線が遮られた面の組の数
    pub blocked_face_pairs: u32,
    /// 値を出した節点の対の数（取付の既定値を置き換えた対と、新しく加わった対）
    pub node_pairs: u32,
    /// そのうち、すべて遮られて 0 とした対の数
    pub blocked_node_pairs: u32,
    /// 節点ごとの面の見え方の収支（節点の ID の順）
    #[serde(default)]
    pub nodes: Vec<ThermalViewFactorNode>,
    /// 形態係数の計算にかかった時間（解析 1 回につき 1 度）
    pub elapsed: Qty<Time>,
}

type Pair = (String, String);

/// 節点の対の値。
pub(super) struct PairValue {
    pub exchange_m2: f64,
    pub detail: ThermalViewFactorPair,
    /// 板を升に分けた端があるときの、升の組への配り方
    pub cells: Option<CellMatrix>,
}

/// 節点対の値を升の組に配る割合（合計 1。板でない端は 1 つの「升」）。対のキーの順（昇順の ID の前の節点が行）で、添字は `行の升 * nb + 列の升`。
#[derive(Debug, Clone, PartialEq)]
pub(super) struct CellMatrix {
    pub na: usize,
    pub nb: usize,
    pub weight: Vec<f64>,
}

/// 形態係数から出した節点の対の値と、要約。
pub(super) struct ViewOutcome {
    pub pairs: BTreeMap<Pair, PairValue>,
    pub used: ThermalViewFactorsUsed,
}

/// 2 つの灰色拡散面の放射の交換面積 [m²]（モジュールの説明の式）。`g` は `A_i F_ij`、`a_i`・`a_j` は面積、`eps` は放射率。
pub fn gray_pair_exchange(g: f64, a_i: f64, a_j: f64, eps: f64) -> f64 {
    if !(g > 0.0 && a_i > 0.0 && a_j > 0.0 && eps > 0.0) {
        return 0.0;
    }
    let rho = 1.0 - eps;
    1.0 / (rho / (eps * a_i) + 1.0 / g + rho / (eps * a_j))
}

fn invalid(msg: &str) -> AnalysisError {
    AnalysisError::InvalidInput(msg.to_owned())
}

fn node_of(nodes: &[ThermalNode]) -> BTreeMap<&str, &str> {
    nodes
        .iter()
        .flat_map(|n| n.parts.iter().map(move |p| (p.id.as_str(), n.id.as_str())))
        .collect()
}

/// 形態係数から節点の対の値を出す。放射率が範囲外なら入力の誤り。空洞が無ければ値は空で、理由を要約に残す（誤りにはしない）。
pub(super) fn resolve(
    net: &ThermalNetwork,
    v: &ThermalViewFactors,
) -> Result<ViewOutcome, AnalysisError> {
    if !(v.emissivity > 0.0 && v.emissivity <= 1.0) {
        return Err(invalid(
            "View factors: the emissivity must be above 0 and at most 1",
        ));
    }
    let started = Instant::now();
    let mut used = ThermalViewFactorsUsed {
        applied: false,
        note: None,
        emissivity: v.emissivity,
        cavities: Vec::new(),
        outside: Vec::new(),
        surfaces: 0,
        face_pairs: 0,
        contact_pairs: 0,
        blocked_face_pairs: 0,
        node_pairs: 0,
        blocked_node_pairs: 0,
        nodes: Vec::new(),
        elapsed: Qty::from_raw(0.0),
    };
    let plates: BTreeMap<String, [usize; 2]> = net
        .plates
        .iter()
        .map(|p| (p.part.clone(), p.cells.map(|c| c as usize)))
        .collect();
    let Some(views) = cavity_views_split(&v.boxes, &plates) else {
        used.note = Some(
            "No part lies wholly inside another part's box, so there is no cavity (the structure box that encloses the components)"
                .to_owned(),
        );
        return Ok(ViewOutcome {
            pairs: BTreeMap::new(),
            used,
        });
    };
    let pairs = node_pairs(net, &views, v.emissivity, &mut used);
    used.applied = true;
    used.cavities = views.containers.clone();
    used.outside = views.outside.clone();
    used.surfaces = views.surfaces.len() as u32;
    used.face_pairs = (views.pairs.len() + views.contact_pairs) as u32;
    used.contact_pairs = views.contact_pairs as u32;
    used.blocked_face_pairs = views.pairs.iter().filter(|p| p.blocked).count() as u32;
    used.nodes = node_residuals(net, &views);
    used.elapsed = Qty::from_raw(started.elapsed().as_secs_f64());
    Ok(ViewOutcome { pairs, used })
}

/// 面の組を節点の対へまとめる。
fn node_pairs(
    net: &ThermalNetwork,
    views: &CavityViews,
    eps: f64,
    used: &mut ThermalViewFactorsUsed,
) -> BTreeMap<Pair, PairValue> {
    let nodes = node_of(&net.nodes);
    let counts = cell_counts(net);
    let mut out: BTreeMap<Pair, PairValue> = BTreeMap::new();
    for fp in &views.pairs {
        let (sa, sb) = (&views.surfaces[fp.a], &views.surfaces[fp.b]);
        let (Some(na), Some(nb)) = (nodes.get(sa.part.as_str()), nodes.get(sb.part.as_str()))
        else {
            continue;
        };
        if na == nb {
            continue;
        }
        let swap = na > nb;
        let key = if swap {
            ((*nb).to_owned(), (*na).to_owned())
        } else {
            ((*na).to_owned(), (*nb).to_owned())
        };
        let e = gray_pair_exchange(fp.exchange, sa.face.area(), sb.face.area(), eps);
        let slot = out.entry(key).or_insert(PairValue {
            exchange_m2: 0.0,
            detail: ThermalViewFactorPair {
                area_factor: Qty::from_raw(0.0),
                face_pairs: 0,
            },
            cells: None,
        });
        if e > 0.0 {
            slot.exchange_m2 += e;
            slot.detail.area_factor = Qty::from_raw(slot.detail.area_factor.value() + fp.exchange);
            slot.detail.face_pairs += 1;
            let ends = [
                counts.get(sa.part.as_str()).copied().unwrap_or(1),
                counts.get(sb.part.as_str()).copied().unwrap_or(1),
            ];
            if ends != [1, 1] {
                add_cell_shares(slot, views.cell_pairs.get(&(fp.a, fp.b)), e, ends, swap);
            }
        }
    }
    used.node_pairs = out.len() as u32;
    used.blocked_node_pairs = out.values().filter(|p| p.detail.face_pairs == 0).count() as u32;
    normalise_cells(&mut out);
    out
}

/// 板の節点（部品の ID）ごとの升の数。
fn cell_counts(net: &ThermalNetwork) -> BTreeMap<&str, usize> {
    net.plates
        .iter()
        .map(|p| {
            (
                p.part.as_str(),
                p.cells.iter().map(|c| *c as usize).product(),
            )
        })
        .collect()
}

/// 節点対の升への配り方に、面の組の値 `e` を足す（`add_face_pair`）。`ends` は面の組の 2 端の節点の升の数、`swap` は節点対のキーの順が面の組の順と逆か。
fn add_cell_shares(
    slot: &mut PairValue,
    split: Option<&CellExchange>,
    e: f64,
    ends: [usize; 2],
    swap: bool,
) {
    let (nl, nr) = if swap {
        (ends[1], ends[0])
    } else {
        (ends[0], ends[1])
    };
    let cells = slot.cells.get_or_insert_with(|| CellMatrix {
        na: nl,
        nb: nr,
        weight: vec![0.0; nl * nr],
    });
    add_face_pair(
        cells,
        e,
        ends,
        split.map(|c| (c.na, c.nb, c.exchange.as_slice())),
        swap,
    );
}

/// 升への配り方の和を 1 にそろえる（和が 0 なら配り方は無い）。
fn normalise_cells(out: &mut BTreeMap<Pair, PairValue>) {
    for p in out.values_mut() {
        if let Some(c) = &mut p.cells {
            let total: f64 = c.weight.iter().sum();
            if total > 0.0 {
                c.weight.iter_mut().for_each(|w| *w /= total);
            } else {
                p.cells = None;
            }
        }
    }
}

/// 面の組の値 `e`（面全体の交換面積）を、節点の升の組の表 `cells`（正規化の前。値の和 = 節点対の値）に足す。
/// `ends` は面の組の 2 端の節点の升の数（板でなければ 1）、`split` は面の組の升の値 `(面 a の升の数, 面 b の升の数, A_c F_cc')`
/// （どちらも升に分けない面の組は `None`）、`swap` は節点対のキーの順が面の組の順と逆か。
/// 升に分けた面は升の値の割合で、板の節点だが升に分けない面（縁の面）は升の面積の割合で配る。
fn add_face_pair(
    cells: &mut CellMatrix,
    e: f64,
    ends: [usize; 2],
    split: Option<(usize, usize, &[f64])>,
    swap: bool,
) {
    let (fa, fb, x) = split.unwrap_or((1, 1, &[1.0]));
    let total: f64 = x.iter().sum();
    if total <= 0.0 {
        return;
    }
    for ia in 0..ends[0] {
        for ib in 0..ends[1] {
            // 面が升に分かれていれば升の番号、分かれていなければ（面全体で 1 つ）0 と、板の節点なら面積の割合
            let (ca, wa) = if fa == ends[0] {
                (ia, 1.0)
            } else {
                (0, 1.0 / ends[0] as f64)
            };
            let (cb, wb) = if fb == ends[1] {
                (ib, 1.0)
            } else {
                (0, 1.0 / ends[1] as f64)
            };
            let w = e * wa * wb * x[ca * fb + cb] / total;
            let at = if swap {
                ib * cells.nb + ia
            } else {
                ia * cells.nb + ib
            };
            cells.weight[at] += w;
        }
    }
}

/// 節点ごとの `1 − Σ_j F_ij`（面積で重みづけ）。
fn node_residuals(net: &ThermalNetwork, views: &CavityViews) -> Vec<ThermalViewFactorNode> {
    let nodes = node_of(&net.nodes);
    let mut acc: BTreeMap<&str, (f64, f64)> = BTreeMap::new();
    for (s, sum) in views.surfaces.iter().zip(&views.row_sums) {
        let Some(node) = nodes.get(s.part.as_str()) else {
            continue;
        };
        let a = s.face.area();
        let e = acc.entry(node).or_insert((0.0, 0.0));
        e.0 += a;
        e.1 += a * sum;
    }
    acc.into_iter()
        .map(|(node, (area, weighted))| ThermalViewFactorNode {
            node: node.to_owned(),
            area: Qty::from_raw(area),
            residual: 1.0 - weighted / area,
        })
        .collect()
}

#[cfg(test)]
mod cells_tests;
#[cfg(test)]
mod tests;
