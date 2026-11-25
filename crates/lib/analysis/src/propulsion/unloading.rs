//! RCS でのアンロードの推薬と ΔV（`propulsion.dv_budget` の区分 `unloading` に足す計算値）。
//!
//! 区間（leg）ごとに `aocs.rw_unloading` を、その区間の軌道・姿勢則・形態で 1 回だけ評価する（時系列は区間の代表の期間。区間の長さ全体は積分しない）。
//! 区間の長さ `T`（フェーズの `duration`。フェーズに期間が無い設計はミッション期間の 1 区間）に対し、
//! 捨てる角運動量 `ΔH = H_day × T / 1 日`、推薬 `m = m_day × T / 1 日`、ΔV `Δv = Δv_day × T / 1 日`（`H_day`・`m_day`・`Δv_day` は
//! [`crate::aocs::RwUnloadingRcs`]。推薬の量 `m = ΔH / (腕 · Isp · g₀)` と ΔV 換算は `rcs_unload` と `sat_astro::rocket` が持つ）。
//! ΔV は日数に比例するとみなす（質量は入力の機体の質量のまま）ので区間の和が使える。
//!
//! - 期間の無い区間は足さずに一覧にだけ載せる（`counted = false`）。
//! - 捨てる向きのスラスタが無い区間は足さず、**警告**を出す（0 として黙って足さない）。足せる区間が 1 つも無ければ理由（[`UnloadingNote`]）を返す。
//! - 磁気トルカでのアンロードは足さない（RCS が無ければ [`UnloadingNote::NoRcs`]）。

use super::input::{UnloadingInput, UnloadingLeg, UnloadingNote};
use crate::aocs::RwUnloading;
use crate::spec::{Analysis, AnalysisError, Check, Verdict};
use sat_astro::constants::SECONDS_PER_DAY;
use sat_astro::rocket::{exhaust_velocity, mass_flow_kg_s};
use sat_units::typed::{AngularMomentum, Mass, Qty, Time, Velocity};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 区間 1 つのアンロード。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DvUnloadingLeg {
    /// フェーズの ID（ミッション期間の区間は `None`）
    pub phase: Option<String>,
    /// 区間の長さ（無ければ足していない）
    pub duration: Option<Qty<Time>>,
    /// 合計に足したか（期間が無い・捨てる向きが無い区間は `false`）
    pub counted: bool,
    /// 区間で捨てる角運動量
    pub dumped_momentum: Option<Qty<AngularMomentum>>,
    /// 最も推薬の要る向き（機体の ±x・±y・±z）
    pub worst_direction: Option<String>,
    /// 区間の推薬
    pub propellant: Option<Qty<Mass>>,
    /// 区間の ΔV 換算
    pub delta_v: Option<Qty<Velocity>>,
}

/// RCS でのアンロードの合計（足した区間の和）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DvUnloading {
    /// 区間（入力の順。足していない区間も載せる）
    pub legs: Vec<DvUnloadingLeg>,
    /// 足した区間の長さの和
    pub covered_duration: Qty<Time>,
    /// 捨てる角運動量の合計
    pub dumped_momentum: Qty<AngularMomentum>,
    /// 推薬の合計
    pub propellant: Qty<Mass>,
    /// ΔV 換算の合計（区分 `unloading` に足す値）
    pub delta_v: Qty<Velocity>,
}

/// アンロードの寄与（足せたか、足せない理由、区間の警告）。
pub(super) struct Contribution {
    pub unloading: Option<DvUnloading>,
    pub note: Option<UnloadingNote>,
    pub checks: Vec<Check>,
}

/// 区間 1 つの結果。RCS が無ければ `None`。
fn leg(l: &UnloadingLeg) -> Result<Option<(DvUnloadingLeg, Option<Check>)>, AnalysisError> {
    let uncounted = |direction| DvUnloadingLeg {
        phase: l.phase.clone(),
        duration: l.duration,
        counted: false,
        dumped_momentum: None,
        worst_direction: direction,
        propellant: None,
        delta_v: None,
    };
    let Some(duration) = l.duration else {
        return Ok(Some((uncounted(None), None)));
    };
    let Some(rcs) = RwUnloading::run(&l.rw_unloading)?.rcs_unloading else {
        return Ok(None);
    };
    let days = duration.value() / SECONDS_PER_DAY;
    let (Some(mp), Some(dv)) = (rcs.per_day.propellant, rcs.delta_v_per_day) else {
        let label = rcs.per_day.label;
        let who = l
            .phase
            .as_ref()
            .map_or_else(|| "The mission".to_owned(), |p| format!("Phase {p}"));
        let check = Check::new(
            "unloading_direction",
            "Directions RCS can unload",
            Verdict::Warning,
            format!(
                "{who}: no thruster can produce torque to dump momentum along {label}, so its unloading is not included"
            ),
        );
        return Ok(Some((uncounted(Some(label)), Some(check))));
    };
    Ok(Some((
        DvUnloadingLeg {
            counted: true,
            dumped_momentum: Some(Qty::from_raw(rcs.momentum_per_day.value() * days)),
            worst_direction: Some(rcs.per_day.label),
            propellant: Some(Qty::from_raw(mp.value() * days)),
            delta_v: Some(Qty::from_raw(dv.value() * days)),
            ..uncounted(None)
        },
        None,
    )))
}

/// アンロードの推薬を、噴射するスラスタが取るタンクごとに分ける（タンクの部品の ID → 推薬 [kg]。マージン・残る分は含めない）。
/// 区間の推薬（足した区間と同じ: 期間があり、捨てる向きにスラスタがある区間）を、同時に同じ時間だけ噴射するスラスタの推薬の流量
/// （`sat_astro::rocket::mass_flow_kg_s`）の比で分ける。`tanks` に載っていないスラスタの分はどのタンクにも入れない（タンクを書かなければ
/// RCS の推薬はどのタンクにも数えない）。
pub(super) fn propellant_by_tank(
    u: &UnloadingInput,
    tanks: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, f64>, AnalysisError> {
    let mut out = BTreeMap::new();
    for l in &u.legs {
        let (Some(duration), Some(input)) = (l.duration, &l.rw_unloading.rcs) else {
            continue;
        };
        let Some(rcs) = RwUnloading::run(&l.rw_unloading)?.rcs_unloading else {
            continue;
        };
        let Some(per_day) = rcs.per_day.propellant else {
            continue;
        };
        let leg_kg = per_day.value() * duration.value() / SECONDS_PER_DAY;
        let flows: Vec<(&str, f64)> = input
            .thrusters
            .iter()
            .filter(|t| rcs.per_day.thrusters.contains(&t.id))
            .map(|t| {
                let flow = mass_flow_kg_s(t.thrust.value(), exhaust_velocity(t.isp.value()));
                (t.id.as_str(), flow)
            })
            .collect();
        let total: f64 = flows.iter().map(|(_, f)| f).sum();
        if total <= 0.0 {
            continue;
        }
        for (id, flow) in flows {
            if let Some(tank) = tanks.get(id) {
                *out.entry(tank.clone()).or_insert(0.0) += leg_kg * flow / total;
            }
        }
    }
    Ok(out)
}

/// 区間ごとに求めて足す。
pub(super) fn contribution(u: &UnloadingInput) -> Result<Contribution, AnalysisError> {
    let mut legs = Vec::new();
    let mut checks = Vec::new();
    for l in &u.legs {
        let Some((leg, check)) = leg(l)? else {
            return Ok(Contribution {
                unloading: None,
                note: Some(UnloadingNote::NoRcs),
                checks: Vec::new(),
            });
        };
        legs.push(leg);
        checks.extend(check);
    }
    let sum = |f: fn(&DvUnloadingLeg) -> Option<f64>| -> f64 {
        legs.iter().filter(|l| l.counted).filter_map(f).sum()
    };
    if !legs.iter().any(|l| l.counted) {
        let note = if checks.is_empty() {
            UnloadingNote::NoMissionDuration
        } else {
            UnloadingNote::NotUnloadable
        };
        return Ok(Contribution {
            unloading: None,
            note: Some(note),
            checks,
        });
    }
    let total = DvUnloading {
        covered_duration: Qty::from_raw(sum(|l| l.duration.map(|d| d.value()))),
        dumped_momentum: Qty::from_raw(sum(|l| l.dumped_momentum.map(|v| v.value()))),
        propellant: Qty::from_raw(sum(|l| l.propellant.map(|v| v.value()))),
        delta_v: Qty::from_raw(sum(|l| l.delta_v.map(|v| v.value()))),
        legs,
    };
    Ok(Contribution {
        unloading: Some(total),
        note: None,
        checks,
    })
}
