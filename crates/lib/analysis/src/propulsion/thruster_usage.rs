//! スラスタの使い方と定格（`propulsion.thruster_usage`）。
//!
//! - マージン込みの設計値の列（`propulsion.propellant` と同じ）から、スラスタごとに総インパルス `Σ m_p v_e`（= 推力 × 噴射時間）、
//!   累積噴射時間、噴射回数（インパルス・有限燃焼は噴射の数、低推力は 1 マヌーバを 1 回の連続噴射として数える）を足す。
//! - 定格（総インパルス・累積噴射時間・噴射回数）があれば比の余裕 `定格 / 使う量 − 1` を判定する
//!   （警告の幅 [`super::input::RATING_MARGIN_WARNING`]）。

use super::input::{PropulsionInput, PropulsionThruster, RATING_MARGIN_WARNING, TransferMethod};
use super::sequence::{Options, Sequence, run};
use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Check, System, margin_verdict,
    quantity_text, ratio_margin,
};
use sat_units::typed::{Fraction, Impulse, Qty, Time};
use serde::{Deserialize, Serialize};

/// スラスタ 1 本の使い方。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThrusterUsageRow {
    pub id: String,
    /// このスラスタを使うマヌーバ
    pub maneuvers: Vec<String>,
    pub total_impulse: Qty<Impulse>,
    pub firing_time: Qty<Time>,
    pub firings: u32,
    pub rated_total_impulse: Option<Qty<Impulse>>,
    pub rated_firing_time: Option<Qty<Time>>,
    pub rated_firings: Option<u32>,
    /// 定格に対する余裕（定格が無いか使う量が 0 なら `None`）
    pub impulse_margin: Option<Fraction>,
    pub firing_time_margin: Option<Fraction>,
    pub firings_margin: Option<Fraction>,
    pub min_impulse_bit: Option<Qty<Impulse>>,
    pub checks: Vec<Check>,
}

/// `propulsion.thruster_usage` の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThrusterUsageOutput {
    pub thrusters: Vec<ThrusterUsageRow>,
    /// 使った ΔV のマージン
    pub dv_margin: Fraction,
}

/// 定格 `rated` と使う量 `used` の判定（定格が無ければ判定しない）。
fn rating(
    key: &str,
    label: &str,
    rated: Option<f64>,
    used: f64,
    text: impl Fn(f64) -> String,
    checks: &mut Vec<Check>,
) -> Option<Fraction> {
    let rated = rated?;
    let margin = ratio_margin(rated, used);
    let verdict = margin.map_or(crate::spec::Verdict::Ok, |m| {
        margin_verdict(m.value(), RATING_MARGIN_WARNING)
    });
    checks.push(Check::new(
        key,
        label,
        verdict,
        format!("Used {} / rated {}", text(used), text(rated)),
    ));
    margin
}

/// スラスタの総インパルス・累積噴射時間・噴射回数と定格の比較。
pub struct ThrusterUsage;

impl Analysis for ThrusterUsage {
    const ID: &'static str = "propulsion.thruster_usage";
    const TITLE: &'static str = "スラスタの使い方";
    const TITLE_EN: &'static str = "Thruster usage";
    const AREA: AnalysisArea = AnalysisArea::System(System::Propulsion);
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str =
        "スラスタごとの総インパルス・累積噴射時間・噴射回数を定格と比べる（ΔV はマージン込み）";
    const DESCRIPTION_EN: &'static str = "Compares each thruster's total impulse, cumulative firing time and firing count with its rating (delta-V with margin)";

    type Input = PropulsionInput;
    type Output = ThrusterUsageOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        let opts = Options::design(input)?;
        let seq = run(input, opts)?;
        let thrusters = input.thrusters.iter().map(|t| row(t, &seq)).collect();
        Ok(ThrusterUsageOutput {
            thrusters,
            dv_margin: Fraction::new(opts.dv_scale - 1.0),
        })
    }
}

/// スラスタ 1 本の使い方（マヌーバの列 `seq` から）。
fn row(t: &PropulsionThruster, seq: &Sequence<'_>) -> ThrusterUsageRow {
    let legs: Vec<_> = seq
        .legs
        .iter()
        .filter(|l| l.thruster.is_some_and(|x| x.id == t.id))
        .collect();
    let time: f64 = legs.iter().filter_map(|l| l.burn_time_s).sum();
    let impulse = t.thrust.value() * time;
    let firings: usize = legs
        .iter()
        .map(|l| match l.maneuver.method {
            TransferMethod::LowThrust => 1,
            _ => l.burns.len(),
        })
        .sum();
    let firings = u32::try_from(firings).unwrap_or(u32::MAX);
    let mut checks = Vec::new();
    let impulse_margin = rating(
        "total_impulse",
        "Total impulse",
        t.rated_total_impulse.map(Qty::value),
        impulse,
        quantity_text::<Impulse>,
        &mut checks,
    );
    let firing_time_margin = rating(
        "firing_time",
        "Cumulative burn time",
        t.rated_firing_time.map(Qty::value),
        time,
        quantity_text::<Time>,
        &mut checks,
    );
    let firings_margin = rating(
        "firings",
        "Number of burns",
        t.rated_firings.map(f64::from),
        f64::from(firings),
        |n| n.to_string(),
        &mut checks,
    );
    ThrusterUsageRow {
        id: t.id.clone(),
        maneuvers: legs.iter().map(|l| l.maneuver.id.clone()).collect(),
        total_impulse: Qty::from_raw(impulse),
        firing_time: Qty::from_raw(time),
        firings,
        rated_total_impulse: t.rated_total_impulse,
        rated_firing_time: t.rated_firing_time,
        rated_firings: t.rated_firings,
        impulse_margin,
        firing_time_margin,
        firings_margin,
        min_impulse_bit: t.min_impulse_bit,
        checks,
    }
}
