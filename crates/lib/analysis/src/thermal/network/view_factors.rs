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
//! 板を升に分けた節点は、部品の値を升に面積で配る（`subdivide`。升ごとの形態係数は出さない）。

use super::{ThermalNetwork, ThermalNode};
use crate::spec::AnalysisError;
use crate::structure::cavity_views::{CavityViews, cavity_views};
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
    let Some(views) = cavity_views(&v.boxes) else {
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
        let key = if na <= nb {
            ((*na).to_owned(), (*nb).to_owned())
        } else {
            ((*nb).to_owned(), (*na).to_owned())
        };
        let e = gray_pair_exchange(fp.exchange, sa.face.area(), sb.face.area(), eps);
        let slot = out.entry(key).or_insert(PairValue {
            exchange_m2: 0.0,
            detail: ThermalViewFactorPair {
                area_factor: Qty::from_raw(0.0),
                face_pairs: 0,
            },
        });
        if e > 0.0 {
            slot.exchange_m2 += e;
            slot.detail.area_factor = Qty::from_raw(slot.detail.area_factor.value() + fp.exchange);
            slot.detail.face_pairs += 1;
        }
    }
    used.node_pairs = out.len() as u32;
    used.blocked_node_pairs = out.values().filter(|p| p.detail.face_pairs == 0).count() as u32;
    out
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
mod tests;
