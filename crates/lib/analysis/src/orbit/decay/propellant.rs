//! 軌道維持に使う推薬と、それを使い切ったときの ΔV。
//!
//! 推薬の量は推進系の解析（`propulsion.propellant`）だけが決める。ここはその値を読むだけで、推薬を別に持たない。
//! 推薬の種類（推薬の名前ごとの組 = `PropellantGroup`）をまたいで換算しない（ある推薬を別の推薬のスラスタの比推力で
//! 換算しない）。軌道維持に使うスラスタと推薬は次の順に決める。
//!
//! 1. 区分が軌道維持（`DvCategory::StationKeeping`）でスラスタを名指すマヌーバがあれば、名指されたスラスタ
//!    （複数なら比推力が最も低いもの。保守側。同じ組の名指されていないスラスタは見ない）と、そのマヌーバが使う推薬（推進系の解析と同じ設計値の列から。ΔV のマージン込み）。
//!    名指したスラスタが複数の推薬の組にまたがれば決められない。
//! 2. そうしたマヌーバが無く、スラスタを持つ推薬の組が 1 つだけなら、その組の最も低い比推力と、推進系の使える推薬
//!    （`usable_propellant`）の全量。スラスタを持つ組が複数あれば決められない。
//!
//! 噴射前の質量は推進系の解析のウェット質量（`wet_mass` = 乾燥質量 + 積む推薬）。推進系が推薬を求めた質量の列の
//! 出発点と同じ基準で、全推薬を使う換算（2.）は設計の ΔV（マージン込み）に戻る。軌道維持の分だけを換算する（1.）ときは、
//! その推薬を最初に使うとした値で、ほかの区分の推薬が先に減った質量は見込まない（軽い質量ほど ΔV は大きいので、
//! 先に減るなら実際の ΔV はこれより大きい。保守側の見積もり）。
//! 決められないときは `Undetermined` を返し、軌道維持のマヌーバにスラスタを書くよう判定（`checks`）で伝える。

use crate::propulsion::{
    DvCategory, Propellant, PropellantOutput, PropulsionInput, PropulsionThruster,
};
use crate::spec::{AnalysisError, ensure_positive};
use sat_astro::rocket::{delta_v, exhaust_velocity};
use sat_units::typed::{Mass, Qty, Velocity};

/// 軌道維持に使う推進系の値（`DecayUsed` に残す）。
pub(super) struct Capacity {
    /// 比推力を取ったスラスタ
    pub thruster: String,
    /// 使える推薬
    pub propellant: Qty<Mass>,
    /// 使える推薬をすべて使ったときの ΔV
    pub delta_v: Qty<Velocity>,
}

/// 軌道維持の推進系の決まり方。
pub(super) enum StationKeeper {
    /// 推進系にスラスタが無い
    NoThruster,
    /// 決められた
    Found(Capacity),
    /// 推薬が複数あり、どれで軌道維持をするか決められない（推薬の名前。名前が無ければスラスタの ID）
    Undetermined(Vec<String>),
}

/// 推進系の入力から、軌道維持に使えるスラスタ・推薬・ΔV を決める。
pub(super) fn capacity(p: &PropulsionInput) -> Result<StationKeeper, AnalysisError> {
    if p.thrusters.is_empty() {
        return Ok(StationKeeper::NoThruster);
    }
    let (out, named) = Propellant::run_for_category(p, DvCategory::StationKeeping)?;
    let found = if named.is_empty() {
        let groups = thruster_groups(&out);
        match groups.as_slice() {
            [only] => Some(candidate(p, only, out.usable_propellant.value())?),
            _ => None,
        }
    } else {
        let mut groups: Vec<Vec<&str>> = Vec::new();
        for n in &named {
            let g = group_of(&out, &n.thruster);
            if !groups.contains(&g) {
                groups.push(g);
            }
        }
        let ids: Vec<&str> = named.iter().map(|n| n.thruster.as_str()).collect();
        let used = named.iter().map(|n| n.propellant.value()).sum();
        (groups.len() == 1)
            .then(|| candidate(p, &ids, used))
            .transpose()?
    };
    let Some((thruster, used)) = found else {
        return Ok(StationKeeper::Undetermined(propellant_names(&out)));
    };
    ensure_positive("比推力", thruster.isp.value())?;
    let dv = delta_v(
        exhaust_velocity(thruster.isp.value()),
        out.wet_mass.value(),
        used,
    )
    .ok_or_else(|| {
        AnalysisError::InvalidInput("推進系の推薬と質量から ΔV を求められません".into())
    })?;
    Ok(StationKeeper::Found(Capacity {
        thruster: thruster.id.clone(),
        propellant: Qty::from_raw(used),
        delta_v: Qty::from_raw(dv),
    }))
}

/// スラスタを持つ推薬の組（スラスタの ID の並び）。
fn thruster_groups(out: &PropellantOutput) -> Vec<Vec<&str>> {
    out.groups
        .iter()
        .filter(|g| !g.thrusters.is_empty())
        .map(|g| g.thrusters.iter().map(String::as_str).collect())
        .collect()
}

/// スラスタ `id` の推薬の組のスラスタの ID（組が見つからなければ `id` だけ）。
fn group_of<'a>(out: &'a PropellantOutput, id: &'a str) -> Vec<&'a str> {
    out.groups
        .iter()
        .find(|g| g.thrusters.iter().any(|t| t == id))
        .map_or_else(
            || vec![id],
            |g| g.thrusters.iter().map(String::as_str).collect(),
        )
}

/// 候補のスラスタ（規則 1 は名指されたもの、規則 2 は組の全部）のうち比推力が最も低いものと、換算する推薬 [kg]。
fn candidate<'a>(
    p: &'a PropulsionInput,
    group: &[&str],
    used: f64,
) -> Result<(&'a PropulsionThruster, f64), AnalysisError> {
    p.thrusters
        .iter()
        .filter(|t| group.contains(&t.id.as_str()))
        .min_by(|a, b| a.isp.value().total_cmp(&b.isp.value()))
        .map(|t| (t, used))
        .ok_or_else(|| AnalysisError::InvalidInput("軌道維持のスラスタが見つかりません".into()))
}

/// 推薬の名前の並び（スラスタを持つ組。名前が無ければスラスタの ID）。
fn propellant_names(out: &PropellantOutput) -> Vec<String> {
    out.groups
        .iter()
        .filter(|g| !g.thrusters.is_empty())
        .map(|g| {
            g.propellant
                .clone()
                .unwrap_or_else(|| g.thrusters.join("・"))
        })
        .collect()
}
