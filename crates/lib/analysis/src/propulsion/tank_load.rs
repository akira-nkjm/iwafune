//! 推薬のタンクごとの内訳（`propulsion.propellant` の `tanks`）。
//!
//! スラスタが推薬を取るタンクは、スラスタが名指したタンク（[`PropulsionInput::thruster_tanks`]）か、名指さなければ同じ推薬のタンク。
//! - **名指したタンク**: そのスラスタが使う推薬（マージン込み + 残る分）が全部そのタンクに入る。RCS でのアンロードの推薬
//!   （区間ごとの推薬を、噴射するスラスタの流量の比で分けたもの。[`super::unloading::propellant_by_tank`]）も、名指したタンクにだけ入る。
//! - **名指さないスラスタ**: 同じ推薬のタンクのうち、どのスラスタにも名指されていないものへ容積の比で分ける
//!   （全部が名指されていれば、その推薬のタンク全部へ容積の比で分ける）。タンクを一つも名指さない設計では、推薬の名前ごとの
//!   組（`PropellantGroup`）の積む量を、組のタンクへ容積の比で分けるこれまでの計算と同じ数になる。
//! - RCS のスラスタがタンクを名指さなければ、アンロードの推薬はどのタンクにも入らない（`dv_budget` の `unloading` の行だけ）。
//!
//! 容積の判定（`Check`）は、スラスタがタンクを名指した推薬の組のタンクにだけ付ける（名指さなければ組の判定と同じ値になる）。

use super::input::{PropulsionInput, TANK_MARGIN_WARNING};
use super::sequence::Sequence;
use crate::spec::{AnalysisError, Check, Verdict, margin_verdict, quantity_text, ratio_margin};
use sat_units::typed::{Fraction, Mass, MassDensity, Qty, Volume};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 推薬のタンク 1 つの内訳。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PropellantTankRow {
    /// タンクの部品の ID
    pub id: String,
    /// 推薬の名前
    pub propellant: Option<String>,
    /// このタンクから推薬を取るスラスタ
    pub thrusters: Vec<String>,
    /// スラスタがこのタンクを名指しているか（`false` は同じ推薬のタンクとして容積の比で分けた）
    pub named: bool,
    /// マヌーバに積む推薬（マージン込みで使う分 + 残る分）
    pub maneuver_propellant: Qty<Mass>,
    /// RCS でのアンロードに積む推薬（使う分 + 残る分。RCS のスラスタがこのタンクを名指したときだけ 0 でない）
    pub unloading_propellant: Qty<Mass>,
    /// 積む推薬（マヌーバ + アンロード）
    pub loaded: Qty<Mass>,
    /// タンクの容積
    pub volume: Qty<Volume>,
    pub density: Option<Qty<MassDensity>>,
    /// 積む推薬の容積（密度が無ければ `None`）
    pub required_volume: Option<Qty<Volume>>,
    /// 入れられる推薬の質量 = 容積 × 密度（密度が無ければ `None`）
    pub capacity: Option<Qty<Mass>>,
    /// 容積の余裕 = タンク / 必要 − 1（必要が 0 か判定できなければ `None`）
    pub volume_margin: Option<Fraction>,
    pub checks: Vec<Check>,
}

/// 推薬の名前（無ければ、種類が 1 つだけのときのその名前）。
pub(super) fn key_of(name: Option<&String>, only: Option<&String>) -> Option<String> {
    name.or(only).cloned()
}

/// 推薬の名前が 1 種類だけならその名前（スラスタにもタンクにも名前の無いものをその推薬とみなす）。
pub(super) fn only_name(input: &PropulsionInput) -> Option<&String> {
    let mut names: Vec<&String> = input
        .thrusters
        .iter()
        .filter_map(|t| t.propellant.as_ref())
        .chain(input.tanks.iter().filter_map(|t| t.propellant.as_ref()))
        .collect();
    names.sort();
    names.dedup();
    (names.len() == 1).then(|| names[0])
}

/// スラスタ `id` のマヌーバが使う推薬 [kg]（マージン込み。残る分は含めない）。
pub(super) fn thruster_used_kg(seq: &Sequence<'_>, id: &str) -> f64 {
    seq.legs
        .iter()
        .filter(|l| l.thruster.is_some_and(|x| x.id == id))
        .filter_map(|l| l.propellant_kg)
        .sum()
}

/// スラスタが取る推薬の振り分け（タンクの部品の ID・推薬の名前ごと）。
struct Draws<'a> {
    /// 名指したタンクの、マヌーバに積む推薬 [kg]
    named: BTreeMap<&'a str, f64>,
    /// 名指したタンクから取るスラスタ
    named_by: BTreeMap<&'a str, Vec<String>>,
    /// 名指さないスラスタの推薬の名前ごとの、マヌーバに積む推薬 [kg] とスラスタ
    pooled: BTreeMap<Option<String>, (f64, Vec<String>)>,
}

/// 名指さないスラスタの推薬と名指したタンクの推薬を、スラスタの順に集める。
fn draws<'a>(
    input: &'a PropulsionInput,
    seq: &Sequence<'_>,
    residual: f64,
) -> Result<Draws<'a>, AnalysisError> {
    for (thruster, tank) in &input.thruster_tanks {
        if !input.tanks.iter().any(|k| &k.id == tank) {
            return Err(AnalysisError::InvalidInput(format!(
                "Thruster {thruster} draws from tank {tank}, which is not in the tank list"
            )));
        }
    }
    let only = only_name(input);
    let mut d = Draws {
        named: BTreeMap::new(),
        named_by: BTreeMap::new(),
        pooled: BTreeMap::new(),
    };
    for t in &input.thrusters {
        let loaded = thruster_used_kg(seq, &t.id) * (1.0 + residual);
        if let Some(tank) = input.thruster_tanks.get(&t.id) {
            *d.named.entry(tank.as_str()).or_insert(0.0) += loaded;
            d.named_by
                .entry(tank.as_str())
                .or_default()
                .push(t.id.clone());
        } else {
            let e = d
                .pooled
                .entry(key_of(t.propellant.as_ref(), only))
                .or_default();
            e.0 += loaded;
            e.1.push(t.id.clone());
        }
    }
    Ok(d)
}

/// 名指さないスラスタの推薬の分け先のタンク（添字）: 同じ推薬のタンクのうち名指されていないもの（全部名指されていれば、その推薬のタンク全部）。
fn pool_tanks(input: &PropulsionInput, only: Option<&String>, i: usize) -> Vec<usize> {
    let key = |j: usize| key_of(input.tanks[j].propellant.as_ref(), only);
    let k = key(i);
    let same: Vec<usize> = (0..input.tanks.len()).filter(|&j| key(j) == k).collect();
    let free: Vec<usize> = same
        .iter()
        .copied()
        .filter(|&j| !is_named(input, &input.tanks[j].id))
        .collect();
    if free.is_empty() { same } else { free }
}

/// スラスタがこのタンクを名指しているか。
fn is_named(input: &PropulsionInput, tank: &str) -> bool {
    input.thruster_tanks.values().any(|t| t == tank)
}

/// タンク `i` の、名指さないスラスタの推薬の取り分 [kg] と、そのスラスタ（分け先でなければ 0 と空）。容積の比 `v / Σv`。
fn pooled_share(
    input: &PropulsionInput,
    d: &Draws<'_>,
    only: Option<&String>,
    i: usize,
) -> (f64, Vec<String>) {
    let tanks = pool_tanks(input, only, i);
    if !tanks.contains(&i) {
        return (0.0, Vec::new());
    }
    let key = key_of(input.tanks[i].propellant.as_ref(), only);
    let Some((required, thrusters)) = d.pooled.get(&key) else {
        return (0.0, Vec::new());
    };
    let total: f64 = tanks.iter().map(|&j| input.tanks[j].volume.value()).sum();
    let v = input.tanks[i].volume.value();
    (
        required * if total > 0.0 { v / total } else { 0.0 },
        thrusters.clone(),
    )
}

/// タンクごとの内訳（入力のタンクの順）。`unloading_kg` は RCS が名指したタンクごとのアンロードの推薬 [kg]（マージン・残る分なし）。
pub(super) fn rows(
    input: &PropulsionInput,
    seq: &Sequence<'_>,
    residual: f64,
    unloading_kg: &BTreeMap<String, f64>,
) -> Result<Vec<PropellantTankRow>, AnalysisError> {
    let d = draws(input, seq, residual)?;
    let only = only_name(input);
    let group_has_named = |i: usize| {
        let k = key_of(input.tanks[i].propellant.as_ref(), only);
        input
            .tanks
            .iter()
            .any(|t| key_of(t.propellant.as_ref(), only) == k && is_named(input, &t.id))
    };
    Ok((0..input.tanks.len())
        .map(|i| {
            let tank = &input.tanks[i];
            let (share, pooled) = pooled_share(input, &d, only, i);
            let named = d.named.get(tank.id.as_str()).copied().unwrap_or(0.0);
            let maneuver = share + named;
            let unloading = unloading_kg
                .get(&tank.id)
                .map_or(0.0, |u| u * (1.0 + residual));
            let loaded = maneuver + unloading;
            let mut thrusters = d
                .named_by
                .get(tank.id.as_str())
                .cloned()
                .unwrap_or_default();
            for (id, t) in &input.thruster_tanks {
                if t == &tank.id && !thrusters.contains(id) {
                    thrusters.push(id.clone());
                }
            }
            thrusters.extend(pooled);
            let required_volume = tank.density.map(|rho| loaded / rho.value());
            let margin = required_volume.and_then(|v| ratio_margin(tank.volume.value(), v));
            let checks = if group_has_named(i) {
                volume_check(&tank.id, tank.volume.value(), required_volume, margin)
            } else {
                Vec::new()
            };
            PropellantTankRow {
                id: tank.id.clone(),
                propellant: tank.propellant.clone(),
                thrusters,
                named: is_named(input, &tank.id),
                maneuver_propellant: Qty::from_raw(maneuver),
                unloading_propellant: Qty::from_raw(unloading),
                loaded: Qty::from_raw(loaded),
                volume: tank.volume,
                density: tank.density,
                required_volume: required_volume.map(Qty::from_raw),
                capacity: tank
                    .density
                    .map(|rho| Qty::from_raw(tank.volume.value() * rho.value())),
                volume_margin: margin,
                checks,
            }
        })
        .collect())
}

/// タンク 1 つの容積の判定（密度が無ければ判定しない。組の判定が密度の警告を出す）。
fn volume_check(
    id: &str,
    volume: f64,
    required: Option<f64>,
    margin: Option<Fraction>,
) -> Vec<Check> {
    let Some(v) = required else {
        return Vec::new();
    };
    let verdict = margin.map_or(Verdict::Ok, |m| {
        margin_verdict(m.value(), TANK_MARGIN_WARNING)
    });
    vec![
        Check::new(
            "tank_volume",
            "Tank volume",
            verdict,
            format!(
                "Tank {id}: needs {} / tank {}",
                quantity_text::<Volume>(v),
                quantity_text::<Volume>(volume)
            ),
        )
        .with_ratio_margin(margin),
    ]
}
