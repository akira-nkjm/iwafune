//! ΔV の内訳表（`propulsion.dv_budget`）。
//!
//! - マヌーバの公称の ΔV（有限燃焼は損失込み。`propulsion.maneuvers` と同じ値）を区分ごとに足す。
//! - RCS でのアンロードの ΔV: `aocs.rw_unloading` の 1 日の ΔV 換算 × ミッション期間（日）を区分 `unloading` に足す。
//!   RCS が無いなどで足せなければ理由（[`UnloadingNote`]）を出す。
//! - マージン `ΔV_マージン込み = ΔV_合計 × (1 + マージン)`。マージンの既定値は [`super::DEFAULT_DV_MARGIN`]。

use super::input::{DvCategory, PropulsionInput, UnloadingInput, UnloadingNote};
use super::maneuvers::nominal;
use crate::aocs::RwUnloading;
use crate::spec::{Analysis, AnalysisArea, AnalysisError, AnalysisKind, System};
use sat_astro::constants::SECONDS_PER_DAY;
use sat_units::typed::{Fraction, Qty, Time, Velocity};
use serde::{Deserialize, Serialize};

/// 内訳表の 1 行（区分）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DvBudgetRow {
    pub category: DvCategory,
    pub delta_v: Qty<Velocity>,
    /// 合計に対する割合（合計が 0 なら 0）
    pub share: Fraction,
    /// この区分のマヌーバ（実行の順）
    pub maneuvers: Vec<String>,
    /// RCS でのアンロードの ΔV を含むか
    pub includes_rcs_unloading: bool,
}

/// RCS でのアンロードの ΔV。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DvUnloading {
    /// 1 日の ΔV 換算（`aocs.rw_unloading`）
    pub delta_v_per_day: Qty<Velocity>,
    pub mission_duration: Qty<Time>,
    pub delta_v: Qty<Velocity>,
}

/// `propulsion.dv_budget` の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DvBudgetOutput {
    /// ΔV のある区分（区分の順）
    pub rows: Vec<DvBudgetRow>,
    pub unloading: Option<DvUnloading>,
    /// アンロードの ΔV を足さなかった理由
    pub unloading_note: Option<UnloadingNote>,
    pub total: Qty<Velocity>,
    pub margin: Fraction,
    pub margin_delta_v: Qty<Velocity>,
    pub total_with_margin: Qty<Velocity>,
}

/// RCS でのアンロードの ΔV（足せなければ理由）。
fn unloading(u: &UnloadingInput) -> Result<Result<DvUnloading, UnloadingNote>, AnalysisError> {
    let out = RwUnloading::run(&u.rw_unloading)?;
    let Some(rcs) = out.rcs_unloading else {
        return Ok(Err(UnloadingNote::NoRcs));
    };
    let Some(per_day) = rcs.delta_v_per_day else {
        return Ok(Err(UnloadingNote::NotUnloadable));
    };
    let days = u.mission_duration.value() / SECONDS_PER_DAY;
    Ok(Ok(DvUnloading {
        delta_v_per_day: per_day,
        mission_duration: u.mission_duration,
        delta_v: Qty::from_raw(per_day.value() * days),
    }))
}

/// 区分ごとの ΔV の内訳・合計・マージン。
pub struct DvBudget;

impl Analysis for DvBudget {
    const ID: &'static str = "propulsion.dv_budget";
    const TITLE: &'static str = "ΔV バジェット";
    const TITLE_EN: &'static str = "Delta-V budget";
    const AREA: AnalysisArea = AnalysisArea::System(System::Propulsion);
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str =
        "マヌーバの ΔV と RCS のアンロードの ΔV を区分ごとに足し、合計とマージンを出す";
    const DESCRIPTION_EN: &'static str =
        "Sums maneuver delta-V and RCS unloading delta-V by category, with the total and margin";

    type Input = PropulsionInput;
    type Output = DvBudgetOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        let margin = input.dv_margin()?;
        let seq = nominal(input)?;
        let (unload, note) = match &input.unloading {
            Some(u) => match unloading(u)? {
                Ok(d) => (Some(d), None),
                Err(n) => (None, Some(n)),
            },
            None => (None, input.unloading_note),
        };
        let rows: Vec<DvBudgetRow> = DvCategory::ALL
            .iter()
            .filter_map(|&c| {
                let legs: Vec<_> = seq
                    .legs
                    .iter()
                    .filter(|l| l.maneuver.category == c)
                    .collect();
                let rcs = (c == DvCategory::Unloading)
                    .then(|| unload.as_ref().map(|u| u.delta_v.value()))
                    .flatten();
                if legs.is_empty() && rcs.is_none() {
                    return None;
                }
                let dv = legs.iter().map(|l| l.delta_v_m_s).sum::<f64>() + rcs.unwrap_or(0.0);
                Some(DvBudgetRow {
                    category: c,
                    delta_v: Qty::from_raw(dv),
                    share: Fraction::new(0.0),
                    maneuvers: legs.iter().map(|l| l.maneuver.id.clone()).collect(),
                    includes_rcs_unloading: rcs.is_some(),
                })
            })
            .collect();
        let total: f64 = rows.iter().map(|r| r.delta_v.value()).sum();
        let rows = rows
            .into_iter()
            .map(|r| DvBudgetRow {
                share: Fraction::new(if total > 0.0 {
                    r.delta_v.value() / total
                } else {
                    0.0
                }),
                ..r
            })
            .collect();
        Ok(DvBudgetOutput {
            rows,
            unloading: unload,
            unloading_note: note,
            total: Qty::from_raw(total),
            margin: Fraction::new(margin),
            margin_delta_v: Qty::from_raw(total * margin),
            total_with_margin: Qty::from_raw(total * (1.0 + margin)),
        })
    }
}
