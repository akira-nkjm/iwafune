//! モード別消費電力（`power.mode_load`）。
//!
//! 部品ごとの「状態 → 消費電力」の表と、モードごとの状態の割り当て（[`crate::modes`]）から、
//! モードごとの平均消費電力を部品別に集計する。割り当ての無い部品や、使う状態の電力が無い部品は
//! **0 W とみなさず**、合計から外して警告に出す。成長余裕は部品ごとの割合（0.2 = +20 %）。
//! 電力収支（`power.energy_balance`）・逆算（`power.sizing`）もモードの負荷を [`evaluate_mode`] で求める。

use crate::modes::{self, ModeDef, StateValues, UnlistedParts};
use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Check, System, ensure_non_negative,
};
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
    /// 部品に `growth_margin` が無いときの成長余裕（0.2 = +20 %）。省略時は [`DEFAULT_GROWTH_MARGIN`]
    #[serde(default)]
    pub default_growth_margin: Option<f64>,
}

/// 部品にも入力にも成長余裕が書かれていないときの成長余裕。余裕を見込まない（0 %）。
/// 適用した値は出力の `parts[].growth_margin` に載る。
pub const DEFAULT_GROWTH_MARGIN: f64 = 0.0;

impl ModeLoadInput {
    /// 部品に成長余裕が無いときの成長余裕（省略時は [`DEFAULT_GROWTH_MARGIN`]）。
    pub fn default_growth_margin(&self) -> f64 {
        self.default_growth_margin.unwrap_or(DEFAULT_GROWTH_MARGIN)
    }
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
    /// 冗長の方式。
    #[serde(default)]
    pub redundancy: crate::modes::Redundancy,
    /// 種別の状態の語彙（warm の予備状態を選ぶ）。
    #[serde(default)]
    pub states: Vec<String>,
    /// 1 台の状態ごとの消費電力（例: `{"by_state": {"idle": 0, "on": 1.2}}`）
    pub power: StateValues<Power>,
    /// 成長余裕（0.2 = +20 %）。省略時は `default_growth_margin`
    #[serde(default)]
    pub growth_margin: Option<f64>,
}

impl ModeLoadPart {
    /// 共通のモード評価に渡す員数と語彙。
    pub fn population(&self) -> crate::modes::Population<'_> {
        crate::modes::Population {
            count: self.count,
            redundancy: self.redundancy,
            states: &self.states,
        }
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
    /// 蝕時の消費電力（成長余裕なし）。
    pub total_eclipse: Qty<Power>,
    /// 蝕時の消費電力（成長余裕込み）。
    pub total_eclipse_with_growth: Qty<Power>,
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
    /// 蝕時の平均消費電力（不足なら None）。
    pub mean_eclipse: Option<Qty<Power>>,
    /// 蝕時の成長余裕込みの平均消費電力。
    pub mean_eclipse_with_growth: Option<Qty<Power>>,
    /// 適用した成長余裕（0.2 = +20 %）
    pub growth_margin: Fraction,
    /// 予備機の常時負荷（予備があるとき、states の末尾）。
    pub spare_power: Option<Qty<Power>>,
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
    const AREA: AnalysisArea = AnalysisArea::System(System::Power);
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
    ensure_non_negative("成長余裕の既定値", input.default_growth_margin())?;
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
    let mut sun = evaluate_sunlit(input, mode)?;
    if mode.eclipse.is_empty() {
        sun.total_eclipse = sun.total;
        sun.total_eclipse_with_growth = sun.total_with_growth;
        for part in &mut sun.parts {
            part.mean_eclipse = part.mean;
            part.mean_eclipse_with_growth = part.mean_with_growth;
        }
    } else {
        let eclipse = evaluate_sunlit(input, &mode.in_eclipse())?;
        sun.total_eclipse = eclipse.total;
        sun.total_eclipse_with_growth = eclipse.total_with_growth;
        for part in &mut sun.parts {
            let value = eclipse.parts.iter().find(|p| p.part == part.part);
            part.mean_eclipse = value.and_then(|p| p.mean);
            part.mean_eclipse_with_growth = value.and_then(|p| p.mean_with_growth);
        }
        for check in eclipse.checks {
            if !sun
                .checks
                .iter()
                .any(|c| c.key == check.key && c.message == check.message)
            {
                sun.checks.push(check);
            }
        }
    }
    Ok(sun)
}

fn evaluate_sunlit(input: &ModeLoadInput, mode: &ModeDef) -> Result<ModeLoadMode, AnalysisError> {
    let ev = evaluate_parts(input, mode)?;
    let mut out = ModeLoadMode {
        id: mode.id.clone(),
        label: mode.label.clone(),
        total: Qty::ZERO,
        total_eclipse: Qty::ZERO,
        total_eclipse_with_growth: Qty::ZERO,
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
        let growth = part.growth_margin.unwrap_or(input.default_growth_margin());
        let result = part_result(part, assignment, mean.map(Qty::<Power>::from_raw), growth)?;
        accumulate(&mut out, part, &result, growth);
        out.parts.push(result);
    }
    Ok(out)
}

fn evaluate_parts(
    input: &ModeLoadInput,
    mode: &ModeDef,
) -> Result<modes::ModeEvaluation, AnalysisError> {
    modes::evaluate(
        mode,
        input.parts.iter().map(|p| p.id.as_str()),
        UnlistedParts::Warn,
        |id| {
            input
                .parts
                .iter()
                .find(|p| p.id == id)
                .map(ModeLoadPart::population)
                .unwrap_or(modes::Population {
                    count: 0,
                    redundancy: modes::Redundancy::None,
                    states: &[],
                })
        },
        |part, state| {
            input
                .parts
                .iter()
                .find(|p| p.id == part)
                .and_then(|p| p.power.get(state))
                .map(Qty::value)
        },
    )
}

fn part_result(
    part: &ModeLoadPart,
    assignment: &crate::modes::StateAssignment,
    mean: Option<Qty<Power>>,
    growth: f64,
) -> Result<ModeLoadPartResult, AnalysisError> {
    let states: Vec<ModeLoadStateShare> = modes::counted_states(assignment, part.population())?
        .into_iter()
        .map(|(state, fraction, count)| ModeLoadStateShare {
            power: part.power.get(&state).map(|p| p * f64::from(count)),
            state,
            fraction: Fraction::new(fraction),
        })
        .collect();
    Ok(ModeLoadPartResult {
        part: part.id.clone(),
        label: part.label.clone(),
        group: part.group.clone(),
        states: states.clone(),
        mean,
        mean_with_growth: mean.map(|m| m * (1.0 + growth)),
        mean_eclipse: None,
        mean_eclipse_with_growth: None,
        growth_margin: Fraction::new(growth),
        spare_power: if assignment.active.is_some_and(|n| n < part.count) {
            states.last().and_then(|s: &ModeLoadStateShare| s.power)
        } else {
            None
        },
    })
}

/// 値が揃った部品の結果をモードの合計・グループ小計・最大に足し込む。
fn accumulate(out: &mut ModeLoadMode, part: &ModeLoadPart, r: &ModeLoadPartResult, growth: f64) {
    let (Some(mean), Some(mean_g)) = (r.mean, r.mean_with_growth) else {
        return;
    };
    out.total = out.total + mean;
    out.total_with_growth = out.total_with_growth + mean_g;
    let spare = r.spare_power;
    let peak = r
        .states
        .iter()
        .take(r.states.len() - usize::from(spare.is_some()))
        .filter(|s| s.fraction.value() > 0.0)
        .filter_map(|s| s.power)
        .fold(Qty::<Power>::ZERO, |a, p| if p > a { p } else { a });
    out.peak_with_growth =
        out.peak_with_growth + (peak + spare.unwrap_or(Qty::ZERO)) * (1.0 + growth);
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
