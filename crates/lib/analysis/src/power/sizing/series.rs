//! 時系列の逆算: 電力収支（`power.energy_balance`）と同じ運用で、1 周の収支 ≥ 0 になる最小の面積を二分法で求め、
//! その面積での最深の放電から最小の容量を閉じた形で求める。
//!
//! - 時系列は最長蝕の標本の日付と β（`orbit.eclipse_season` の `longest`）で回す。
//! - 姿勢則ごとの発電は [`ScaledBalance`] で 1 回だけ作り、面積の比（× 経年・固有の劣化）を倍率にかける。
//! - 運用はモードごとのケース（最長継続時間があれば基本のモードの上で 1 回）と、全モードを逆算するときはモードの並び。
//! - 容量は全体の必要面積（運用のうち最大）で、[`battery::required_capacity`] から求める。

use super::calculate::ModeLoads;
use super::{SizingInput, SizingMode, SizingUsed};
use crate::orbit::EclipseSeasonPoint;
use crate::power::battery::{self, EnergyBalanceBattery};
use crate::power::energy_balance::{
    EnergyBalanceCase, EnergyBalanceEvaluation, EnergyBalanceInput, EnergyBalanceLoad, LoadGrowth,
    ScaledBalance, ScaledJudgement,
};
use crate::power::mode_load::ModeLoadOutput;
use crate::power::solar::SolarLife;
use crate::spec::AnalysisError;
use sat_astro::time::Epoch;
use sat_astro::timeline::TimelineSpec;
use sat_units::typed::{Angle, Area, Energy, Fraction, Qty};
use serde::{Deserialize, Serialize};

/// 二分法を止める面積の相対誤差（区間の幅 / 上端）。
pub const AREA_TOLERANCE: f64 = 1e-4;
/// 面積の上端を探すとき、初めの見積もり（SMAD の面積）を 2 倍にする回数の上限。
const MAX_DOUBLINGS: u32 = 40;
/// モードの並びの運用の ID（`power.energy_balance` の並びの結果と同じ）。
pub const SEQUENCE_TARGET: &str = "sequence";

/// 時系列の逆算の 1 つの運用（モードか、モードの並び）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SizingSeriesTarget {
    /// モードの ID（モードの並びは `sequence`）
    pub id: String,
    /// 電力収支の判定と同じ運用（最長継続時間で評価したか・並びか）
    pub evaluation: EnergyBalanceEvaluation,
    /// 1 周の収支 ≥ 0 になる最小の面積
    pub required_area: Qty<Area>,
    /// `required_area` での 1 周あたりのエネルギー余裕（電池の効率込み）
    pub energy_margin_per_orbit: Qty<Energy>,
    /// 全体の必要面積（[`SizingTimeSeries::required_area`]）で DoD が上限に収まる最小の容量
    pub required_capacity: Qty<Energy>,
    /// 二分法で収支を評価した回数（上端を探す分を含む）
    pub evaluations: u32,
}

/// 時系列の逆算の結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SizingTimeSeries {
    /// 時系列を回した日付（最長蝕の標本）
    pub epoch: Epoch,
    /// 時系列を回した β（最長蝕の標本）
    pub beta: Qty<Angle>,
    /// 発電にかけた劣化（`used.life_factor × used.inherent_degradation`）
    pub degradation: Fraction,
    /// 二分法を止めた面積の相対誤差
    pub area_tolerance: Fraction,
    /// 必要面積（運用のうち最大）
    pub required_area: Qty<Area>,
    /// 必要面積を決めた運用
    pub area_target: String,
    /// 必要容量（運用のうち最大）
    pub required_capacity: Qty<Energy>,
    /// 必要容量を決めた運用
    pub capacity_target: String,
    /// 運用ごとの値（モードの順、並びがあれば最後）
    pub targets: Vec<SizingSeriesTarget>,
}

/// 時系列の逆算に要る、入力の外の値。
pub(super) struct SeriesBasis<'a> {
    pub battery: &'a EnergyBalanceBattery,
    pub used: &'a SizingUsed,
    pub design_area: f64,
    pub longest: &'a EclipseSeasonPoint,
    pub loads: &'a [ModeLoads],
    /// 全モードの負荷の内訳（`power.mode_load` の出力）
    pub mode_loads: &'a ModeLoadOutput,
    /// SMAD の逆算（面積の初めの見積もりに使う）
    pub smad: &'a [SizingMode],
}

/// 時系列の逆算。判定できない（回り切った周回が無いなど）ときは理由を `Err` で返す（解析は SMAD で判定する）。
pub(super) fn evaluate(
    input: &SizingInput,
    basis: &SeriesBasis,
) -> Result<Result<SizingTimeSeries, String>, AnalysisError> {
    let eb = balance_input(input, basis)?;
    let scaled = ScaledBalance::new(&eb)?;
    let degradation = basis.used.life_factor * basis.used.inherent_degradation;
    let per_area = degradation / basis.design_area;
    let guess = basis
        .smad
        .iter()
        .map(|m| m.required_area.value())
        .fold(0.0, f64::max);
    let mut found = Vec::new();
    for (i, l) in basis.loads.iter().enumerate() {
        let start = basis.smad.get(i).map_or(guess, |m| m.required_area.value());
        match minimum_area(|a| scaled.case(i, a * per_area), start)? {
            Ok(f) => found.push((l.id.clone(), f)),
            Err(reason) => return Ok(Err(format!("Mode {}: {reason}", l.id))),
        }
    }
    if !eb.sequence.is_empty() {
        match minimum_area(|a| sequence(&scaled, a * per_area), guess)? {
            Ok(f) => found.push((SEQUENCE_TARGET.to_owned(), f)),
            Err(reason) => return Ok(Err(format!("Mode sequence: {reason}"))),
        }
    }
    capacities(&scaled, basis, found, per_area, degradation)
}

/// 全体の必要面積で各運用の容量を求め、結果をまとめる。
fn capacities(
    scaled: &ScaledBalance,
    basis: &SeriesBasis,
    found: Vec<(String, Found)>,
    per_area: f64,
    degradation: f64,
) -> Result<Result<SizingTimeSeries, String>, AnalysisError> {
    let Some((area_target, area)) = governing(found.iter().map(|(id, f)| (id, f.area))) else {
        return Ok(Err("No operation to size against".into()));
    };
    let (soc, dod) = (basis.battery.start_soc(), basis.used.dod_max.value());
    let mut targets = Vec::with_capacity(found.len());
    for (i, (id, f)) in found.into_iter().enumerate() {
        let judged = if i >= scaled.case_count() {
            sequence(scaled, area * per_area)?
        } else {
            scaled.case(i, area * per_area)?
        };
        let Some(capacity) = battery::required_capacity(&judged.delta, soc, dod) else {
            return Ok(Err(format!(
                "{id}: the initial SOC alone reaches the allowed DoD, so the capacity cannot be determined"
            )));
        };
        targets.push(SizingSeriesTarget {
            id,
            evaluation: f.evaluation,
            required_area: Qty::from_raw(f.area),
            energy_margin_per_orbit: Qty::from_raw(f.margin),
            required_capacity: Qty::from_raw(capacity),
            evaluations: f.evaluations,
        });
    }
    let capacity = governing(targets.iter().map(|t| (&t.id, t.required_capacity.value())));
    let (capacity_target, capacity) = capacity.unwrap_or((area_target.clone(), 0.0));
    Ok(Ok(SizingTimeSeries {
        epoch: basis.longest.epoch,
        beta: basis.longest.beta,
        degradation: Fraction::new(degradation),
        area_tolerance: Fraction::new(AREA_TOLERANCE),
        required_area: Qty::from_raw(area),
        area_target,
        required_capacity: Qty::from_raw(capacity),
        capacity_target,
        targets,
    }))
}

/// 値が最大の運用（同じ値なら先のもの）。空なら None。
fn governing<'a>(values: impl Iterator<Item = (&'a String, f64)>) -> Option<(String, f64)> {
    values
        .fold(None, |best: Option<(&String, f64)>, (id, v)| match best {
            Some((_, b)) if v <= b => best,
            _ => Some((id, v)),
        })
        .map(|(id, v)| (id.clone(), v))
}

fn sequence(scaled: &ScaledBalance, scale: f64) -> Result<ScaledJudgement, AnalysisError> {
    scaled
        .sequence(scale)?
        .ok_or_else(|| AnalysisError::InvalidInput("No mode sequence".into()))
}

/// 二分法で見つけた最小の面積。
struct Found {
    area: f64,
    margin: f64,
    evaluation: EnergyBalanceEvaluation,
    evaluations: u32,
}

/// 1 周の収支 ≥ 0 になる最小の面積 [m²]。`judge(a)` は面積 `a` での判定、`start` は初めの見積もり。
/// 収支は面積に対して増えるので、上端を 2 倍ずつ探してから二分法で詰める。
fn minimum_area(
    judge: impl Fn(f64) -> Result<ScaledJudgement, AnalysisError>,
    start: f64,
) -> Result<Result<Found, String>, AnalysisError> {
    const NO_ORBIT: &str =
        "No complete orbit in the analysis span, so the per-orbit balance cannot be judged";
    let mut evaluations = 1;
    let zero = judge(0.0)?;
    let Some(m0) = zero.margin else {
        return Ok(Err(NO_ORBIT.into()));
    };
    if m0 >= 0.0 {
        return Ok(Ok(Found {
            area: 0.0,
            margin: m0,
            evaluation: zero.evaluation,
            evaluations,
        }));
    }
    let (mut lo, mut hi) = (0.0, if start > 0.0 { start } else { 1.0 });
    let mut best = loop {
        let j = judge(hi)?;
        evaluations += 1;
        match j.margin {
            None => return Ok(Err(NO_ORBIT.into())),
            Some(m) if m >= 0.0 => break (m, j.evaluation),
            Some(_) if evaluations > MAX_DOUBLINGS => {
                return Ok(Err(
                    "The per-orbit balance does not reach 0 even with more area".into(),
                ));
            }
            Some(_) => (lo, hi) = (hi, hi * 2.0),
        }
    };
    while hi - lo > AREA_TOLERANCE * hi {
        let mid = 0.5 * (lo + hi);
        let j = judge(mid)?;
        evaluations += 1;
        match j.margin {
            Some(m) if m >= 0.0 => (hi, best) = (mid, (m, j.evaluation)),
            _ => lo = mid,
        }
    }
    Ok(Ok(Found {
        area: hi,
        margin: best.0,
        evaluation: best.1,
        evaluations,
    }))
}

/// 時系列の逆算に渡す電力収支の入力。電池は満充電から始め、空にならない大きさにする
/// （容量は増分から閉じた形で求め、最長継続時間の最悪の開始は満充電からの下がり幅で選ぶため）。
fn balance_input(
    input: &SizingInput,
    basis: &SeriesBasis,
) -> Result<EnergyBalanceInput, AnalysisError> {
    let timeline = worst_season(&input.timeline, basis.longest);
    let peak_load = basis
        .mode_loads
        .modes
        .iter()
        .map(|m| {
            m.total_with_growth
                .value()
                .abs()
                .max(m.total_eclipse_with_growth.value().abs())
        })
        .fold(0.0, f64::max);
    let span = timeline.sampling.span.value();
    let mut probe = basis.battery.clone();
    probe.capacity = Qty::from_raw(2.0 * peak_load * span / probe.discharge_eff() + 1.0);
    probe.initial_soc = Some(1.0);
    let cases = basis
        .loads
        .iter()
        .map(|l| EnergyBalanceCase {
            id: l.id.clone(),
            label: String::new(),
            load: EnergyBalanceLoad::Mode { mode: l.id.clone() },
            attitude: input.mode_laws.get(&l.id).cloned(),
            max_duration: input.max_durations.get(&l.id).copied(),
            timing: input.mode_timings.get(&l.id).copied().unwrap_or_default(),
        })
        .collect();
    Ok(EnergyBalanceInput {
        timeline,
        array: input.array.clone(),
        life: Some(SolarLife::Eol),
        batteries: vec![probe],
        loads: input.loads.clone(),
        load_growth: Some(LoadGrowth::Included),
        cases,
        sequence: if input.mode.is_none() {
            input.sequence.clone()
        } else {
            Vec::new()
        },
        sequence_timing: input.sequence_timing.clone(),
        // 並びを使わないときも、基本のモードは電力収支と同じ（設定、無ければ並びの最初）
        base_mode: input
            .base_mode
            .clone()
            .or_else(|| input.sequence.first().map(|s| s.mode.clone())),
        mode_attitudes: input.mode_laws.clone(),
    })
}

/// 時系列の日付と β を最長蝕の標本に替える（姿勢則・期間・刻みはそのまま）。
pub fn worst_season(timeline: &TimelineSpec, longest: &EclipseSeasonPoint) -> TimelineSpec {
    let mut t = timeline.clone();
    // 季節変化（β と日付）は要素で書かれた軌道だけの機能（`power.sizing` の入力は `OrbitSpec` から作る）。
    if let Some(orbit) = t.sampling.orbit.elements_mut() {
        orbit.epoch = longest.epoch;
    }
    t.sampling.beta = Some(longest.beta);
    t
}
