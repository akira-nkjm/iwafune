//! ΔV の内訳表（`propulsion.dv_budget`）。
//!
//! - マヌーバの公称の ΔV（有限燃焼は損失込み。`propulsion.maneuvers` と同じ値）を区分ごとに足す。
//! - RCS でのアンロードの ΔV: フェーズ（期間のあるもの。無ければミッション期間）ごとの `aocs.rw_unloading` の 1 日の ΔV 換算 × 日数を、
//!   区分 `unloading` に足す（[`super::unloading`]）。書かれた `unloading` のマヌーバがあれば、その ΔV に**足す**（置き換えない。
//!   同じ区分の行の中で、書かれたものと計算値の両方が合計に入る）。足せなければ理由（[`UnloadingNote`]）を出す。
//! - 日付つきの惑星間遷移（`transfer`）があれば、その結果と判定（[`super::dated`]）を添える。
//! - マージン `ΔV_マージン込み = ΔV_合計 × (1 + マージン)`。マージンの既定値は [`super::DEFAULT_DV_MARGIN`]。

use super::dated::{self, DatedTransferResult};
use super::input::{DvCategory, PropulsionInput, UnloadingNote};
use super::maneuvers::nominal;
use super::unloading::{self, DvUnloading};
use crate::spec::{Analysis, AnalysisArea, AnalysisError, AnalysisKind, Check, System};
use sat_units::typed::{Fraction, Qty, Velocity};
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
    /// 日付つきの惑星間遷移の結果（`transfer` を書いたマヌーバ。実行の順）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<DatedTransferResult>>"))]
    pub dated_transfers: Vec<DatedTransferResult>,
    /// 日付つきの遷移の判定（赤緯・出発の日の時刻）と、アンロードを足せなかった区間の警告
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<Check>>"))]
    pub checks: Vec<Check>,
}

/// 区分ごとの ΔV の内訳・合計・マージン。
pub struct DvBudget;

impl Analysis for DvBudget {
    const ID: &'static str = "propulsion.dv_budget";
    const USES: &'static [&'static str] = &[crate::aocs::RwUnloading::ID];
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
        let (unload, note, unloading_checks) = match &input.unloading {
            Some(u) => {
                let c = unloading::contribution(u)?;
                (c.unloading, c.note, c.checks)
            }
            None => (None, input.unloading_note, Vec::new()),
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
            dated_transfers: seq.legs.iter().filter_map(dated::result).collect(),
            checks: (0..seq.legs.len())
                .flat_map(|i| dated::checks(&seq, input, i))
                .chain(unloading_checks)
                .collect(),
        })
    }
}
