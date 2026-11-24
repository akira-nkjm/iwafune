//! 節点間の熱結合を決める。**どの結合を使うか（優先順位）と、使った結合の一覧はここだけ。**
//!
//! 節点対ごとに 1 つの値を使う。優先順位は 明示（`conductive`・`radiative`）＞ 外殻（`enclosure`。放射のみ）＞ 形態係数（`view_factors`。放射のみ）＞ 取付の既定値（`mounts`）。
//! 取付の既定値の伝導は、`contact_conductance_per_area` を書けば取付ごとに接触面積から出し（出せない取付は 1 取付あたりの値）、
//! 出所は、接触面積から出した取付をその対に 1 つでも含めば `contact`（内訳は `contact`）、なければ `mount`。
//! 明示した対は取付から作った値を **置き換える**（0 なら切り離す）。書いていない対は取付の既定値のまま。
//! 形態係数の値（内部の部品と構造の殻の内側の面どうし。計算は `view_factors`）は、値を出した対の取付の既定値を置き換え、外殻と明示の値には置き換えられる。
//! 外殻の 6 面と節点の対応は、外面の法線（機体座標の ±X・±Y・±Z）と面積（箱の面の面積に近いもの）から決める。
//! 6 面が 1 つずつの別の節点に決まらないときは外殻の放射を使わず、理由を出力に残す（誤りにはしない）。

use super::mount_contact::{ThermalContactPair, ThermalContactUsed};
use super::mounts::{MountConductance, MountLinks};
use super::view_factors::{ThermalViewFactorPair, ThermalViewFactorsUsed};
use super::{ThermalNetwork, ThermalRadiativeLink, ThermalSurface};
use crate::spec::AnalysisError;
use crate::thermal::enclosure::{BoxExchange, ThermalEnclosure, box_exchange};
use sat_astro::math::Vec3;
use sat_astro::view_factor::BOX_FACES;
use sat_units::typed::{Area, Qty, ThermalConductance};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 面の法線が軸と一致するとみなす余弦の下限（約 2.6°）。
const FACE_NORMAL_COS: f64 = 0.999;
/// 外面の面積を箱の面の面積とみなす相対許容差。
const FACE_AREA_TOLERANCE: f64 = 0.05;

/// 結合の出どころ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ThermalCouplingSource {
    /// 取付から作った既定値（1 取付あたりの値 × 取付の数）
    Mount,
    /// 取付の両端の部品の箱の接触面積から出した既定値（`h × A`。出せない取付が混じる対は `contact.fixed_mounts` に数が出る）
    Contact,
    /// 設計に明示した値
    Explicit,
    /// 箱の外殻の内面の放射（形態係数と Gebhart 係数）
    Enclosure,
    /// 内部の部品と構造の殻の内側の面どうしの放射（遮蔽つきの形態係数。`from_view_factors` に内訳）
    ViewFactor,
}

/// 使った伝導の結合（節点対は ID の昇順）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalConductanceUsed {
    pub a: String,
    pub b: String,
    pub conductance: Qty<ThermalConductance>,
    pub source: ThermalCouplingSource,
    /// 接触面積から出した取付の内訳（出所が `contact` のときだけ）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub from_contact: Option<ThermalContactPair>,
}

/// 使った放射の結合（有効放射交換面積。節点対は ID の昇順）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalExchangeUsed {
    pub a: String,
    pub b: String,
    pub exchange_area: Qty<Area>,
    pub source: ThermalCouplingSource,
    /// 形態係数から出した対の内訳（出所が `view_factor` のときだけ）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub from_view_factors: Option<ThermalViewFactorPair>,
}

/// 外殻の 1 面。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalEnclosureFace {
    /// 外向き法線（`+X` など）
    pub face: String,
    /// 面を担う節点
    pub node: String,
    /// 箱の面の面積
    pub area: Qty<Area>,
}

/// 外殻の放射のモデルを使ったか、使ったときの形態係数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalEnclosureUsed {
    /// 使ったか（面と節点の対応が決まらなければ使わない）
    pub applied: bool,
    /// 使わなかった理由
    #[serde(default)]
    pub note: Option<String>,
    /// 内面の放射率
    pub emissivity: f64,
    /// 6 面（`+X, -X, +Y, -Y, +Z, -Z` の順）。使わなかったときは空
    #[serde(default)]
    pub faces: Vec<ThermalEnclosureFace>,
    /// 面どうしの形態係数（`faces` の順の 6×6）。使わなかったときは空
    #[serde(default)]
    pub view_factors: Vec<Vec<f64>>,
}

/// 熱ネットワークが使う結合の一覧。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalCouplings {
    pub conductive: Vec<ThermalConductanceUsed>,
    pub radiative: Vec<ThermalExchangeUsed>,
    /// 外殻の放射を設計が求めたときだけ
    #[serde(default)]
    pub enclosure: Option<ThermalEnclosureUsed>,
    /// 内部の放射を形態係数から出す設定のときだけ（使えなければ理由つき）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub view_factors: Option<ThermalViewFactorsUsed>,
    /// 取付の伝導を接触面積から出す設定のときだけ（出せなかった取付はここに理由つきで残る）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mount_contact: Option<ThermalContactUsed>,
    /// 升に分けた板ごとの報告（板を分けたときだけ。結合は板の部品の名前のまま上に並び、升への配分はここの規則）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plates: Vec<super::ThermalPlateUsed>,
}

type Pair = (String, String);

fn invalid(msg: String) -> AnalysisError {
    AnalysisError::InvalidInput(msg)
}

/// 節点対を ID の昇順にそろえる。
fn pair(a: &str, b: &str) -> Pair {
    if a <= b {
        (a.to_owned(), b.to_owned())
    } else {
        (b.to_owned(), a.to_owned())
    }
}

/// 明示の結合を対ごとに表へ入れる（同じ対の重複・同じ節点どうしは入力の誤り）。
fn explicit<T>(
    links: &[T],
    ends: impl Fn(&T) -> (&str, &str, f64),
) -> Result<BTreeMap<Pair, f64>, AnalysisError> {
    let mut out = BTreeMap::new();
    for l in links {
        let (a, b, v) = ends(l);
        if a == b {
            return Err(invalid(format!("Coupling {a}–{b} joins a node to itself")));
        }
        if out.insert(pair(a, b), v).is_some() {
            return Err(invalid(format!("Coupling {a}–{b} is listed twice")));
        }
    }
    Ok(out)
}

/// 優先順位に従って、使う結合を決める。
pub(super) fn resolve(net: &ThermalNetwork) -> Result<ThermalCouplings, AnalysisError> {
    let MountLinks {
        conductive: mount_cond,
        radiative: mount_rad,
        contact: mount_contact,
    } = net
        .mounts
        .as_ref()
        .map(super::ThermalMounts::links)
        .transpose()?
        .unwrap_or_default();
    let (mut cond, mut details) = mounted_conductance(&mount_cond);
    let mut rad = mounted(&mount_rad, |l: &ThermalRadiativeLink| {
        (&l.a, &l.b, l.exchange_area.value())
    });
    let (view_factors, mut view_details) = apply_view_factors(net, &mut rad)?;
    let enclosure = match &net.enclosure {
        Some(e) => Some(apply_enclosure(net, e, &mut rad)?),
        None => None,
    };
    let explicit_cond = explicit(&net.conductive, |l| (&l.a, &l.b, l.conductance.value()))?;
    let explicit_rad = explicit(&net.radiative, |l| (&l.a, &l.b, l.exchange_area.value()))?;
    for (p, v) in explicit_cond {
        details.remove(&p);
        cond.insert(p, (v, ThermalCouplingSource::Explicit));
    }
    for (p, v) in explicit_rad {
        rad.insert(p, (v, ThermalCouplingSource::Explicit));
    }
    Ok(ThermalCouplings {
        conductive: cond
            .into_iter()
            .map(|((a, b), (v, source))| ThermalConductanceUsed {
                from_contact: details.get(&(a.clone(), b.clone())).copied(),
                a,
                b,
                conductance: Qty::from_raw(v),
                source,
            })
            .collect(),
        radiative: radiative_used(rad, &mut view_details),
        enclosure,
        view_factors,
        mount_contact,
        plates: Vec::new(),
    })
}

type Table = BTreeMap<Pair, (f64, ThermalCouplingSource)>;

/// 形態係数の値を表へ入れる（値を出した対の取付の既定値を置き換える）。要約と、対ごとの内訳を返す。
fn apply_view_factors(
    net: &ThermalNetwork,
    rad: &mut Table,
) -> Result<
    (
        Option<ThermalViewFactorsUsed>,
        BTreeMap<Pair, ThermalViewFactorPair>,
    ),
    AnalysisError,
> {
    let mut details = BTreeMap::new();
    let Some(v) = &net.view_factors else {
        return Ok((None, details));
    };
    let found = super::view_factors::resolve(net, v)?;
    for (p, value) in found.pairs {
        rad.insert(
            p.clone(),
            (value.exchange_m2, ThermalCouplingSource::ViewFactor),
        );
        details.insert(p, value.detail);
    }
    Ok((Some(found.used), details))
}

/// 使った放射の一覧。形態係数の値のまま残った対にだけ内訳を付ける（外殻・明示が置き換えた対には付けない）。
fn radiative_used(
    rad: Table,
    details: &mut BTreeMap<Pair, ThermalViewFactorPair>,
) -> Vec<ThermalExchangeUsed> {
    rad.into_iter()
        .map(|((a, b), (v, source))| ThermalExchangeUsed {
            from_view_factors: (source == ThermalCouplingSource::ViewFactor)
                .then(|| details.remove(&(a.clone(), b.clone())))
                .flatten(),
            a,
            b,
            exchange_area: Qty::from_raw(v),
            source,
        })
        .collect()
}

/// 取付から作った伝導の表と、接触面積から出した対の内訳。接触面積から出した取付を含む対の出所は `contact`。
fn mounted_conductance(links: &[MountConductance]) -> (Table, BTreeMap<Pair, ThermalContactPair>) {
    let mut cond = Table::new();
    let mut details = BTreeMap::new();
    for l in links {
        let source = if l.contact.is_some() {
            ThermalCouplingSource::Contact
        } else {
            ThermalCouplingSource::Mount
        };
        cond.insert(pair(&l.a, &l.b), (l.conductance.value(), source));
        if let Some(c) = l.contact {
            details.insert(pair(&l.a, &l.b), c);
        }
    }
    (cond, details)
}

/// 取付から作った結合（取付の対はすでに昇順で 1 対 1 つ）。
fn mounted<T>(links: &[T], ends: impl Fn(&T) -> (&String, &String, f64)) -> Table {
    links
        .iter()
        .map(|l| {
            let (a, b, v) = ends(l);
            (pair(a, b), (v, ThermalCouplingSource::Mount))
        })
        .collect()
}

/// 外殻の放射を表へ入れる（面どうしの対が取付の既定値を置き換える）。
fn apply_enclosure(
    net: &ThermalNetwork,
    e: &ThermalEnclosure,
    rad: &mut Table,
) -> Result<ThermalEnclosureUsed, AnalysisError> {
    let eps = e.emissivity;
    let not_applied = |note: &str| ThermalEnclosureUsed {
        applied: false,
        note: Some(note.to_owned()),
        emissivity: eps,
        faces: Vec::new(),
        view_factors: Vec::new(),
    };
    let Some(size) = &e.size else {
        return Ok(not_applied(
            "The box size is not known (the root part needs a `size`)",
        ));
    };
    let size = [size.x.value(), size.y.value(), size.z.value()];
    let Some(exchange) = box_exchange(size, eps) else {
        return Err(invalid(
            "Enclosure: the box size must be positive and the emissivity above 0 and at most 1"
                .to_owned(),
        ));
    };
    let nodes = match face_nodes(&net.surfaces, &exchange) {
        Ok(n) => n,
        Err(note) => return Ok(not_applied(&note)),
    };
    for i in 0..6 {
        for j in (i + 1)..6 {
            rad.insert(
                pair(&nodes[i], &nodes[j]),
                (exchange.exchange_m2[i][j], ThermalCouplingSource::Enclosure),
            );
        }
    }
    Ok(ThermalEnclosureUsed {
        applied: true,
        note: None,
        emissivity: eps,
        faces: (0..6)
            .map(|i| ThermalEnclosureFace {
                face: BOX_FACES[i].to_owned(),
                node: nodes[i].clone(),
                area: Qty::from_raw(exchange.area_m2[i]),
            })
            .collect(),
        view_factors: exchange.view_factors.clone(),
    })
}

/// 箱の面の外向き法線（`BOX_FACES` の順）。
fn face_normal(i: usize) -> Vec3 {
    let mut v = [0.0; 3];
    v[i / 2] = if i.is_multiple_of(2) { 1.0 } else { -1.0 };
    Vec3::new(v[0], v[1], v[2])
}

/// 6 面を担う節点。面ごとに、法線がその面を向き面積が箱の面に近い外面の節点が 1 つに決まり、
/// 6 面の節点がすべて異なるときだけ `Ok`。そうでなければ理由（英文）を返す。
fn face_nodes(surfaces: &[ThermalSurface], exchange: &BoxExchange) -> Result<Vec<String>, String> {
    let mut nodes: Vec<String> = Vec::with_capacity(6);
    for i in 0..6 {
        let want = exchange.area_m2[i];
        let mut found: Vec<&str> = surfaces
            .iter()
            .filter(|s| {
                let area = s.area.value() * f64::from(s.count);
                s.normal
                    .normalized()
                    .is_some_and(|n| n.dot(face_normal(i)) >= FACE_NORMAL_COS)
                    && (area - want).abs() <= FACE_AREA_TOLERANCE * want
            })
            .map(|s| s.node.as_str())
            .collect();
        found.sort_unstable();
        found.dedup();
        match found.as_slice() {
            [node] => nodes.push((*node).to_owned()),
            [] => {
                return Err(format!(
                    "No outer surface node was found for the {} face of the box",
                    BOX_FACES[i]
                ));
            }
            _ => {
                return Err(format!(
                    "The {} face of the box matches several nodes",
                    BOX_FACES[i]
                ));
            }
        }
    }
    let mut sorted = nodes.clone();
    sorted.sort_unstable();
    if sorted.windows(2).any(|w| w[0] == w[1]) {
        return Err("Two faces of the box belong to the same node".to_owned());
    }
    Ok(nodes)
}

#[cfg(test)]
mod contact_tests;
#[cfg(test)]
pub(super) mod tests;
