//! モードの並びで回す電力収支: 並び（1 周期分）を繰り返して解析期間を覆い、標本ごとにその時刻のモードの
//! 負荷（日照中・日陰中、蝕の上書きは `mode_load` の評価のまま）と姿勢則の発電を使って電池を積分する。
//!
//! 時刻 `t` のモードは、解析の開始からの経過を周期で割った余りが入る区間（`[始め, 終わり)`）のモード。
//! 区間の境目は標本の刻みで丸める（刻みの間は線形補間）。

use super::case::{balance, duty_of, mode_loads};
use super::context::mode_generation;
use super::{Context, EnergyBalanceCaseResult, EnergyBalanceEvaluation, EnergyBalanceInput};
use crate::power::battery::BatteryTrace;
use crate::spec::{AnalysisError, ensure_positive};
use sat_units::typed::{Power, Qty, Time};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// モードの並びの 1 区間。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EnergyBalanceSegment {
    pub mode: String,
    pub duration: Qty<Time>,
}

/// モードの並びで回した電力収支。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EnergyBalanceSequence {
    /// 並びの 1 周期（区間の長さの合計）
    pub cycle: Qty<Time>,
    /// 並びの区間（入力の順）
    pub segments: Vec<EnergyBalanceSegment>,
    /// 並びに沿って負荷と姿勢則を切り替えた収支（`id` は `sequence`）。
    /// `load_sunlit`・`load_eclipse` は日照中・日陰中の標本の平均、判定は並びに沿って回したときのもの
    pub balance: EnergyBalanceCaseResult,
}

/// 並びの ID と表示名。
const SEQUENCE_ID: &str = "sequence";
const SEQUENCE_LABEL: &str = "モードの並び";
/// 並びの判定の括弧書き。
const SEQUENCE_BASIS: &str = "モードの並びに沿って回したとき";

/// モードの (日照中, 日陰中) の負荷と、そのモードの姿勢則での発電の列。
type ModeColumns = ((Qty<Power>, Qty<Power>), Vec<f64>);

/// 区間の長さを検査し、1 周期の長さを返す。
pub(super) fn validate(input: &EnergyBalanceInput) -> Result<f64, AnalysisError> {
    let mut cycle = 0.0;
    for s in &input.sequence {
        ensure_positive("モードの並びの区間の長さ", s.duration.value())?;
        cycle += s.duration.value();
    }
    Ok(cycle)
}

/// 並びで回す（並びが空なら None）。電池の積分（`trace`）も返す（時系列の逆算が刻みごとの増分を使う）。
pub(super) fn evaluate(
    ctx: &Context,
) -> Result<Option<(EnergyBalanceSequence, BatteryTrace)>, AnalysisError> {
    let input = ctx.input;
    if input.sequence.is_empty() {
        return Ok(None);
    }
    let cycle = validate(input)?;
    let mut checks = Vec::new();
    let mut modes: BTreeMap<&str, ModeColumns> = BTreeMap::new();
    for s in &input.sequence {
        if !modes.contains_key(s.mode.as_str()) {
            let loads = mode_loads(input, &s.mode, &mut checks)?;
            modes.insert(&s.mode, (loads, mode_generation(ctx, &s.mode)?));
        }
    }
    let columns: Vec<&ModeColumns> = input
        .sequence
        .iter()
        .filter_map(|s| modes.get(s.mode.as_str()))
        .collect();
    let (load, generation) = along_sequence(ctx, &columns, cycle);
    let means = (lit_mean(ctx, &load, true), lit_mean(ctx, &load, false));
    let mixed = ctx.with_generation(generation);
    let (mut result, trace) = balance(
        &mixed,
        (SEQUENCE_ID, SEQUENCE_LABEL),
        means,
        load,
        checks,
        SEQUENCE_BASIS,
    );
    result.evaluation = EnergyBalanceEvaluation::Sequence;
    result.duty = duty_of(input, input.sequence.iter().map(|s| s.mode.as_str()));
    let sequence = EnergyBalanceSequence {
        cycle: Qty::from_raw(cycle),
        segments: input.sequence.clone(),
        balance: result,
    };
    Ok(Some((sequence, trace)))
}

/// 標本ごとの負荷と発電の列。`columns[i]` は並びの `i` 番目の区間のモードの値。
fn along_sequence(ctx: &Context, columns: &[&ModeColumns], cycle: f64) -> (Vec<f64>, Vec<f64>) {
    let ends: Vec<f64> = ctx
        .input
        .sequence
        .iter()
        .scan(0.0, |end, s| {
            *end += s.duration.value();
            Some(*end)
        })
        .collect();
    let t = &ctx.profile.t;
    let t0 = t.first().copied().unwrap_or(0.0);
    let last = columns.len().saturating_sub(1);
    let (mut load, mut generation) = (Vec::with_capacity(t.len()), Vec::with_capacity(t.len()));
    for (k, &tk) in t.iter().enumerate() {
        let phase = (tk - t0).rem_euclid(cycle);
        let i = ends.iter().position(|&e| phase < e).unwrap_or(last);
        let Some(((sun, ecl), g)) = columns.get(i) else {
            continue;
        };
        load.push(if ctx.profile.lit[k] {
            sun.value()
        } else {
            ecl.value()
        });
        generation.push(g[k]);
    }
    (load, generation)
}

/// 日照中（`lit`）か日陰中の標本の負荷の平均（標本が無ければ 0）。
fn lit_mean(ctx: &Context, load: &[f64], lit: bool) -> Qty<Power> {
    let v: Vec<f64> = load
        .iter()
        .zip(&ctx.profile.lit)
        .filter(|(_, l)| **l == lit)
        .map(|(p, _)| *p)
        .collect();
    Qty::from_raw(if v.is_empty() {
        0.0
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    })
}
