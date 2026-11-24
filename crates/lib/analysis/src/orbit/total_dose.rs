//! 放射線の総線量: 利用者が与える環境の表（遮蔽の厚さ → 線量率）を、フェーズごとの期間で積み、部品ごとの総線量と許容線量との余裕を出す。
//! 放射線帯や遮蔽の輸送計算はしない（環境の表は利用者のもの）。球殻の等価アルミ厚さを仮定し、はじき出し損傷・単一事象の影響は扱わない。
//!
//! - 区間（leg）: 1 つのフェーズ（フェーズが無い設計は設計の軌道とミッション期間の 1 区間）。期間か環境の表が無い区間は
//!   **推測せず、出力の `legs[].status` に挙げて総線量から除く**（除いたことは `phases_left_out` の警告）。
//! - 線量率: 部品の遮蔽厚さで環境の表を引く。引き方（遮蔽の厚さについて線形、線量率は対数で結ぶ。表の外は外挿しない）は
//!   `sat_astro::math::interpolate` が持ち、使った規則は出力の `interpolation`。表の外の遮蔽の部品は **評価しない**（端の値を使わない）。
//! - 総線量 = 評価した区間の（線量率 × 期間）の和。余裕 = 許容線量 / (総線量 × 設計余裕 RDM) − 1（`spec::ratio_margin`）。
//!   許容線量が無い部品は線量だけを出し、判定は付けない。RDM の既定は [`DEFAULT_DESIGN_MARGIN`]（出力に書く）。

use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Check, ConditionTopic, Verdict,
    ensure_positive, ratio_margin, ratio_margin_verdict,
};
use sat_astro::math::{ValueScale, interpolate};
use sat_units::typed::{
    AbsorbedDose, AbsorbedDoseKind, AbsorbedDoseRate, Fraction, Length, Qty, Time,
};
use serde::{Deserialize, Serialize};

/// 設計余裕（RDM。許容線量を、予測した総線量の何倍以上に取るか）の既定。宇宙機の放射線設計で慣用の 2。
/// **既定値はここだけ**（出力の `design_margin` と `design_margin_source` に書く）。
pub const DEFAULT_DESIGN_MARGIN: f64 = 2.0;

/// 環境の表の点 1 つ（遮蔽の厚さ → 線量率）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DoseDepthPoint {
    /// 等価アルミの遮蔽厚さ
    pub shielding: Qty<Length>,
    /// 線量率（吸収線量 / 時間。正）
    pub rate: Qty<AbsorbedDoseRate>,
}

/// 区間の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DoseLegInput {
    /// フェーズの ID（フェーズが無い設計は `None`）
    pub phase: Option<String>,
    /// 期間（無ければ評価しない）
    pub duration: Option<Qty<Time>>,
    /// 環境の表（遮蔽の厚さが増える順。無ければ評価しない）
    pub environment: Option<Vec<DoseDepthPoint>>,
}

/// 部品の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DosePartInput {
    pub part: String,
    /// 部品の遮蔽厚さ（無ければ `default_shielding`）
    pub shielding: Option<Qty<Length>>,
    /// 許容する総線量（無ければ判定しない）
    #[serde(serialize_with = "AbsorbedDoseKind::serialize_option")]
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub tolerance: Option<Qty<AbsorbedDose>>,
}

/// 総線量の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TotalDoseInput {
    pub legs: Vec<DoseLegInput>,
    pub parts: Vec<DosePartInput>,
    /// 遮蔽厚さを書かない部品に使う厚さ（無ければ、その部品は評価しない）
    pub default_shielding: Option<Qty<Length>>,
    /// 設計余裕 RDM（無ければ [`DEFAULT_DESIGN_MARGIN`]）
    pub design_margin: Option<f64>,
}

/// 表の引き方。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DoseInterpolation {
    /// 遮蔽の厚さについて線形、線量率は対数（対数線形。表の外は外挿しない）
    LogLinear,
}

/// 設計余裕の出どころ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DesignMarginSource {
    /// 設定に書かれた値
    Given,
    /// この解析の既定（[`DEFAULT_DESIGN_MARGIN`]）
    Default,
}

/// 遮蔽厚さの出どころ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ShieldingSource {
    /// 部品に書かれた値
    Part,
    /// 設計の既定の遮蔽厚さ
    Default,
}

/// 区間の評価の状態。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DoseLegStatus {
    /// 評価した
    Evaluated,
    /// 期間が無いので除いた
    NoDuration,
    /// 環境の表が無いので除いた
    NoEnvironment,
}

/// 部品を評価できなかった理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DoseNotEvaluated {
    /// 遮蔽厚さが無く、既定も無い
    NoShielding,
    /// 遮蔽厚さが、ある区間の環境の表の範囲の外
    OutsideTable,
}

/// 区間 1 つの結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DoseLeg {
    pub phase: Option<String>,
    pub status: DoseLegStatus,
    pub duration: Option<Qty<Time>>,
    /// 基準の遮蔽厚さ（評価した部品のうち最も薄いもの）での線量率（表の外なら `None`）
    pub rate_at_reference: Option<Qty<AbsorbedDoseRate>>,
    /// 基準の遮蔽厚さでの区間の線量
    #[serde(serialize_with = "AbsorbedDoseKind::serialize_option")]
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub dose_at_reference: Option<Qty<AbsorbedDose>>,
}

/// 部品の区間ごとの線量。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DosePhase {
    pub phase: Option<String>,
    pub rate: Qty<AbsorbedDoseRate>,
    #[serde(serialize_with = "AbsorbedDoseKind::serialize")]
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub dose: Qty<AbsorbedDose>,
}

/// 部品 1 つの結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DosePart {
    pub part: String,
    pub shielding: Option<Qty<Length>>,
    pub shielding_source: Option<ShieldingSource>,
    /// 評価した区間ごとの線量（評価できなければ空）
    pub phases: Vec<DosePhase>,
    /// 総線量（評価できなければ `None`）
    #[serde(serialize_with = "AbsorbedDoseKind::serialize_option")]
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub total: Option<Qty<AbsorbedDose>>,
    #[serde(serialize_with = "AbsorbedDoseKind::serialize_option")]
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub tolerance: Option<Qty<AbsorbedDose>>,
    /// 余裕 = 許容線量 / (総線量 × RDM) − 1（許容線量が無い・総線量が 0 なら `None`）
    pub margin: Option<Fraction>,
    /// 判定（許容線量があり、評価できたときだけ）
    pub verdict: Option<Verdict>,
    pub not_evaluated: Option<DoseNotEvaluated>,
}

/// 余裕が最も小さい部品。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DoseWorst {
    pub part: String,
    pub margin: Fraction,
}

/// 総線量の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TotalDoseOutput {
    pub interpolation: DoseInterpolation,
    pub design_margin: f64,
    pub design_margin_source: DesignMarginSource,
    pub reference_shielding: Option<Qty<Length>>,
    pub legs: Vec<DoseLeg>,
    pub parts: Vec<DosePart>,
    pub worst: Option<DoseWorst>,
    pub checks: Vec<Check>,
}

/// 放射線の総線量の解析。
pub struct TotalDose;

impl Analysis for TotalDose {
    const ID: &'static str = "orbit.total_dose";
    const TITLE: &'static str = "放射線の総線量（利用者の環境の表）";
    const TITLE_EN: &'static str = "Total ionising dose";
    const AREA: AnalysisArea = AnalysisArea::Condition(ConditionTopic::Orbit);
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str = "利用者が与えた環境の表（遮蔽の厚さ → 線量率）をフェーズの期間で積み、部品ごとの総線量と許容線量との余裕を出す";
    const DESCRIPTION_EN: &'static str = "Total dose per part from a user-supplied environment table (dose rate versus shielding) over the phase durations, with the margin to its tolerance";
    type Input = TotalDoseInput;
    type Output = TotalDoseOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        run(input)
    }
}

/// 環境の表の検査（1 点以上・遮蔽が正で増える順・線量率が正で有限）。
fn validate(table: &[DoseDepthPoint]) -> Result<(), AnalysisError> {
    let bad = |why: &str| Err(AnalysisError::InvalidInput(format!("Dose table: {why}")));
    if table.is_empty() {
        return bad("it has no point");
    }
    for p in table {
        ensure_positive("shielding thickness", p.shielding.value())?;
        ensure_positive("dose rate", p.rate.value())?;
    }
    if table
        .windows(2)
        .any(|w| w[1].shielding.value() <= w[0].shielding.value())
    {
        return bad("the shielding thickness must be strictly increasing");
    }
    Ok(())
}

/// 遮蔽厚さ `t`（m）での線量率 [Gy/s]。表の外は `None`。**表を引くのはここだけ。**
fn rate_at(table: &[DoseDepthPoint], t: f64) -> Option<f64> {
    let xs: Vec<f64> = table.iter().map(|p| p.shielding.value()).collect();
    let ys: Vec<f64> = table.iter().map(|p| p.rate.value()).collect();
    interpolate(&xs, &ys, t, ValueScale::Log)
}

/// 評価する区間（期間と環境の表がそろったもの）。
struct Used<'a> {
    index: usize,
    duration: f64,
    table: &'a [DoseDepthPoint],
}

fn run(input: &TotalDoseInput) -> Result<TotalDoseOutput, AnalysisError> {
    if input.parts.is_empty() {
        return Err(AnalysisError::InvalidInput("No parts to evaluate".into()));
    }
    let (rdm, rdm_source) = match input.design_margin {
        Some(m) => (m, DesignMarginSource::Given),
        None => (DEFAULT_DESIGN_MARGIN, DesignMarginSource::Default),
    };
    ensure_positive("design margin", rdm)?;
    let mut used = Vec::new();
    for (index, leg) in input.legs.iter().enumerate() {
        if let Some(table) = &leg.environment {
            validate(table)?;
        }
        if let (Some(d), Some(table)) = (leg.duration, &leg.environment) {
            ensure_positive("phase duration", d.value())?;
            used.push(Used {
                index,
                duration: d.value(),
                table,
            });
        }
    }
    if used.is_empty() {
        return Err(AnalysisError::InvalidInput(
            "No phase has both a duration and an environment table".into(),
        ));
    }
    let parts: Vec<DosePart> = input
        .parts
        .iter()
        .map(|p| part_result(p, input, &used, input.legs.as_slice(), rdm))
        .collect();
    let reference = parts
        .iter()
        .filter(|p| p.not_evaluated.is_none())
        .filter_map(|p| p.shielding)
        .min_by(|a, b| a.value().total_cmp(&b.value()));
    let legs = legs_result(&input.legs, &used, reference);
    let worst = parts
        .iter()
        .filter_map(|p| p.margin.map(|m| (p, m)))
        .min_by(|a, b| a.1.value().total_cmp(&b.1.value()))
        .map(|(p, margin)| DoseWorst {
            part: p.part.clone(),
            margin,
        });
    let checks = checks::checks(&parts, &legs, rdm);
    Ok(TotalDoseOutput {
        interpolation: DoseInterpolation::LogLinear,
        design_margin: rdm,
        design_margin_source: rdm_source,
        reference_shielding: reference,
        legs,
        parts,
        worst,
        checks,
    })
}

fn part_result(
    p: &DosePartInput,
    input: &TotalDoseInput,
    used: &[Used<'_>],
    legs: &[DoseLegInput],
    rdm: f64,
) -> DosePart {
    let (shielding, source) = match (p.shielding, input.default_shielding) {
        (Some(s), _) => (Some(s), Some(ShieldingSource::Part)),
        (None, Some(s)) => (Some(s), Some(ShieldingSource::Default)),
        (None, None) => (None, None),
    };
    let mut out = DosePart {
        part: p.part.clone(),
        shielding,
        shielding_source: source,
        phases: Vec::new(),
        total: None,
        tolerance: p.tolerance,
        margin: None,
        verdict: None,
        not_evaluated: None,
    };
    let Some(t) = shielding.map(|s| s.value()) else {
        out.not_evaluated = Some(DoseNotEvaluated::NoShielding);
        return out;
    };
    let mut phases = Vec::new();
    for u in used {
        let Some(rate) = rate_at(u.table, t) else {
            out.not_evaluated = Some(DoseNotEvaluated::OutsideTable);
            return out;
        };
        phases.push(DosePhase {
            phase: legs[u.index].phase.clone(),
            rate: Qty::from_raw(rate),
            dose: Qty::from_raw(rate * u.duration),
        });
    }
    let total: f64 = phases.iter().map(|x| x.dose.value()).sum();
    out.phases = phases;
    out.total = Some(Qty::from_raw(total));
    if let Some(tol) = p.tolerance {
        out.margin = ratio_margin(tol.value(), total * rdm);
        out.verdict = Some(ratio_margin_verdict(out.margin, Fraction::new(0.0)));
    }
    out
}

fn legs_result(
    legs: &[DoseLegInput],
    used: &[Used<'_>],
    reference: Option<Qty<Length>>,
) -> Vec<DoseLeg> {
    legs.iter()
        .enumerate()
        .map(|(i, leg)| {
            let hit = used.iter().find(|u| u.index == i);
            let status = match (hit, leg.duration, &leg.environment) {
                (Some(_), _, _) => DoseLegStatus::Evaluated,
                (None, None, _) => DoseLegStatus::NoDuration,
                (None, Some(_), _) => DoseLegStatus::NoEnvironment,
            };
            let rate = hit
                .zip(reference)
                .and_then(|(u, r)| rate_at(u.table, r.value()).map(|x| (x, u.duration)));
            DoseLeg {
                phase: leg.phase.clone(),
                status,
                duration: leg.duration,
                rate_at_reference: rate.map(|(x, _)| Qty::from_raw(x)),
                dose_at_reference: rate.map(|(x, d)| Qty::from_raw(x * d)),
            }
        })
        .collect()
}

mod checks;

#[cfg(test)]
mod tests;
