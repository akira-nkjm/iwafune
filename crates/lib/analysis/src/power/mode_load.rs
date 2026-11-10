//! モード別消費電力（`power.mode_load`）。
//!
//! 部品ごとの「状態 → 消費電力」の表と、モードごとの状態の割り当て（[`crate::modes`]）から、
//! モードごとの平均消費電力を部品別に集計する。割り当ての無い部品や、使う状態の電力が無い部品は
//! **0 W とみなさず**、合計から外して警告に出す。成長余裕は部品ごとの割合（0.2 = +20 %）。
//! 電力収支（`power.energy_balance`）もモードの負荷を [`evaluate_mode`] で求める。

use crate::modes::{self, ModeDef, StateValues, UnlistedParts};
use crate::spec::{Analysis, AnalysisError, AnalysisKind, Check, System, ensure_non_negative};
use sat_units::typed::{Fraction, Power, Qty};
use serde::{Deserialize, Serialize};

/// モード別消費電力の入力。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModeLoadInput {
    /// 電力を使う部品
    pub parts: Vec<ModeLoadPart>,
    /// モード（部品ごとの状態の割り当て）
    pub modes: Vec<ModeDef>,
    /// 部品に `growth_margin` が無いときの成長余裕（0.2 = +20 %）。省略時 0
    #[serde(default)]
    pub default_growth_margin: f64,
}

/// 電力を使う部品 1 つ。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModeLoadPart {
    /// 部品 ID（モードの割り当ての `part` と一致させる）
    pub id: String,
    #[serde(default)]
    pub label: String,
    /// 表示上のまとまり（例: `"AOCS"`）。集計の小計に使う
    #[serde(default)]
    pub group: String,
    /// 員数。消費電力は単体値を渡し、解析が掛ける（省略時 1）
    #[serde(default = "crate::spec::default_count")]
    pub count: u32,
    /// 1 台の状態ごとの消費電力（例: `{"by_state": {"off": 0, "on": 1.2}}`）
    pub power: StateValues<Power>,
    /// 成長余裕（0.2 = +20 %）。省略時は `default_growth_margin`
    #[serde(default)]
    pub growth_margin: Option<f64>,
}

impl ModeLoadPart {
    /// 状態 `state` での部品の消費電力（単体値 × 員数）。表に無ければ `None`。
    pub fn power_in(&self, state: &str) -> Option<Qty<Power>> {
        self.power.get(state).map(|p| p * f64::from(self.count))
    }
}

/// モード別消費電力の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModeLoadOutput {
    pub modes: Vec<ModeLoadMode>,
}

/// 1 モードの集計。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModeLoadMode {
    pub id: String,
    pub label: String,
    /// 平均消費電力の合計（成長余裕なし。値が揃った部品だけ）
    pub total: Qty<Power>,
    /// 平均消費電力の合計（成長余裕込み）
    pub total_with_growth: Qty<Power>,
    /// 瞬時の最大（各部品がこのモードで使う状態の最大電力を同時に取ったとき、成長余裕込み）
    pub peak_with_growth: Qty<Power>,
    /// グループ別の小計（グループ名の順）
    pub groups: Vec<ModeLoadGroup>,
    /// 部品別の内訳（入力の部品の順。割り当ての無い部品は含まない）
    pub parts: Vec<ModeLoadPartResult>,
    /// このモードに割り当ての無い部品の ID（合計に入れていない）
    pub unassigned: Vec<String>,
    /// 割り当てはあるが、使う状態の消費電力が無い部品（合計に入れていない）
    pub missing_power: Vec<ModeLoadMissing>,
    /// 警告（`unassigned`・`missing_value`・`unknown_part`。[`crate::modes::ModeEvaluation::checks`]）
    pub checks: Vec<Check>,
}

/// グループ別の小計。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModeLoadGroup {
    pub group: String,
    pub total: Qty<Power>,
    pub total_with_growth: Qty<Power>,
}

/// 部品 1 つの内訳。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModeLoadPartResult {
    pub part: String,
    pub label: String,
    pub group: String,
    /// 状態ごとの時間割合と消費電力
    pub states: Vec<ModeLoadStateShare>,
    /// 平均消費電力（使う状態の電力が欠けていれば `None`）
    pub mean: Option<Qty<Power>>,
    /// 成長余裕込みの平均消費電力
    pub mean_with_growth: Option<Qty<Power>>,
    /// 適用した成長余裕（0.2 = +20 %）
    pub growth_margin: Fraction,
}

/// 1 状態の時間割合と電力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModeLoadStateShare {
    pub state: String,
    /// 時間割合（0〜1）
    pub fraction: Fraction,
    /// その状態の部品の消費電力（単体値 × 員数。表に無ければ `None`）
    pub power: Option<Qty<Power>>,
}

/// 消費電力が足りない部品と状態。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModeLoadMissing {
    pub part: String,
    pub state: String,
}

/// モード別消費電力。
pub struct ModeLoad;

impl Analysis for ModeLoad {
    const ID: &'static str = "power.mode_load";
    const TITLE: &'static str = "モード別消費電力";
    const SYSTEM: System = System::Power;
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str =
        "モードごとの消費電力を部品別に集計する（デューティ比・成長余裕込み）";
    const TITLE_EN: &'static str = "Mode power consumption";
    const DESCRIPTION_EN: &'static str =
        "Sums power consumption per mode by part, with duty cycles and growth margins";
    type Input = ModeLoadInput;
    type Output = ModeLoadOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        validate(input)?;
        let modes = input
            .modes
            .iter()
            .map(|m| evaluate_mode(input, m))
            .collect::<Result<_, _>>()?;
        Ok(ModeLoadOutput { modes })
    }
}

/// 部品の ID の重複と成長余裕の値を調べる（電力収支も使う）。
pub(crate) fn validate(input: &ModeLoadInput) -> Result<(), AnalysisError> {
    ensure_non_negative("成長余裕の既定値", input.default_growth_margin)?;
    for (i, p) in input.parts.iter().enumerate() {
        if input.parts[..i].iter().any(|q| q.id == p.id) {
            return Err(AnalysisError::InvalidInput(format!(
                "部品 ID {} が重複しています",
                p.id
            )));
        }
        if let Some(g) = p.growth_margin {
            ensure_non_negative(&format!("部品 {} の成長余裕", p.id), g)?;
        }
    }
    Ok(())
}

/// 1 モードの消費電力を集計する（`power.energy_balance` もこれを使う）。
/// 平均と、未割り当て・値の不足の扱いは [`crate::modes::evaluate`]（警告して合計から除く）。
pub fn evaluate_mode(input: &ModeLoadInput, mode: &ModeDef) -> Result<ModeLoadMode, AnalysisError> {
    let ev = modes::evaluate(
        mode,
        input.parts.iter().map(|p| p.id.as_str()),
        UnlistedParts::Warn,
        |part, state| {
            input
                .parts
                .iter()
                .find(|p| p.id == part)
                .and_then(|p| p.power_in(state))
                .map(Qty::value)
        },
    )?;
    let mut out = ModeLoadMode {
        id: mode.id.clone(),
        label: mode.label.clone(),
        total: Qty::ZERO,
        total_with_growth: Qty::ZERO,
        peak_with_growth: Qty::ZERO,
        groups: Vec::new(),
        parts: Vec::new(),
        unassigned: ev.unassigned.clone(),
        missing_power: ev
            .missing
            .iter()
            .map(|(part, state)| ModeLoadMissing {
                part: part.clone(),
                state: state.clone(),
            })
            .collect(),
        checks: ev.checks("消費電力"),
    };
    for (part, (_, mean)) in input.parts.iter().zip(&ev.means) {
        let Some(assignment) = mode.assignment(&part.id) else {
            continue;
        };
        let growth = part.growth_margin.unwrap_or(input.default_growth_margin);
        let result = part_result(part, assignment, mean.map(Qty::<Power>::from_raw), growth)?;
        accumulate(&mut out, part, &result, growth);
        out.parts.push(result);
    }
    Ok(out)
}

fn part_result(
    part: &ModeLoadPart,
    assignment: &crate::modes::StateAssignment,
    mean: Option<Qty<Power>>,
    growth: f64,
) -> Result<ModeLoadPartResult, AnalysisError> {
    let states = assignment
        .fractions()?
        .into_iter()
        .map(|(state, fraction)| ModeLoadStateShare {
            power: part.power_in(&state),
            state,
            fraction: Fraction::new(fraction),
        })
        .collect();
    Ok(ModeLoadPartResult {
        part: part.id.clone(),
        label: part.label.clone(),
        group: part.group.clone(),
        states,
        mean,
        mean_with_growth: mean.map(|m| m * (1.0 + growth)),
        growth_margin: Fraction::new(growth),
    })
}

/// 値が揃った部品の結果をモードの合計・グループ小計・最大に足し込む。
fn accumulate(out: &mut ModeLoadMode, part: &ModeLoadPart, r: &ModeLoadPartResult, growth: f64) {
    let (Some(mean), Some(mean_g)) = (r.mean, r.mean_with_growth) else {
        return;
    };
    out.total = out.total + mean;
    out.total_with_growth = out.total_with_growth + mean_g;
    let peak = r
        .states
        .iter()
        .filter(|s| s.fraction.value() > 0.0)
        .filter_map(|s| s.power)
        .fold(Qty::<Power>::ZERO, |a, p| if p > a { p } else { a });
    out.peak_with_growth = out.peak_with_growth + peak * (1.0 + growth);
    let idx = match out.groups.binary_search_by(|g| g.group.cmp(&part.group)) {
        Ok(i) => i,
        Err(i) => {
            out.groups.insert(
                i,
                ModeLoadGroup {
                    group: part.group.clone(),
                    total: Qty::ZERO,
                    total_with_growth: Qty::ZERO,
                },
            );
            i
        }
    };
    out.groups[idx].total = out.groups[idx].total + mean;
    out.groups[idx].total_with_growth = out.groups[idx].total_with_growth + mean_g;
}

#[cfg(test)]
mod tests;
