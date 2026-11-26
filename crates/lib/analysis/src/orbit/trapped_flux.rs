//! 放射線帯の捕捉粒子（AP-8 の陽子・AE-8 の電子）の、軌道平均の積分フラックスとミッション期間のフルエンス。
//!
//! **方法**: 設計の軌道を 1 日分（`spec::daily_sampling`。地球の自転で地上軌跡が 1 周する長さ）の刻みで伝搬し（`OrbitSampling::states`）、
//! 各点の位置を ECEF にして、地図ごとに決まった古い地磁気モデルの元期の場（[`TrappedModel::field`]。年変化は足さない）で
//! McIlwain の L と B/B₀ を求める（`sat_astro::radiation::belt_position`）。標準のエネルギーの格子（[`proton_energies_mev`]・
//! [`electron_energies_mev`]）ごとに地図から積分フラックス `> E`（全方向）を引き、時間平均（台形則）・最大値・
//! フラックスが 0 でない時間の割合を求める。**フルエンス = 時間平均のフラックス × ミッション期間**（軌道は 1 日の標本が代表すると仮定する。
//! 軌道の経年変化は見ない）。陽子・電子それぞれ太陽活動の極小・極大の 4 つの地図で結果を並べる。
//!
//! **制限**: 静的なモデル（極小・極大の 2 つの元期。太陽活動の周期内の変化・磁気嵐・太陽粒子は含まない）。L が求まらない
//! （磁力線が閉じない・L > 16）点は地図の外としてフラックス 0 に数え、`samples_without_l` に出す。地球の周りの軌道だけ。
//! 地図は配布データ（`fune data`）で、無ければ解析は入力不足（`missing`）になる。

use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, ConditionTopic, ensure_positive,
    time_fraction,
};
use sat_astro::earth::eci_to_ecef;
use sat_astro::ephem::CentralBody;
use sat_astro::geomag::{FieldSnapshot, NoL};
use sat_astro::radiation::{
    BeltConvention, BeltField, BeltPosition, RadiationTables, SolarCondition, Species, TrappedMap,
    TrappedModel, belt_position_with,
};
use sat_astro::timeline::{OrbitPoint, OrbitSampling, time_mean};
use sat_units::typed::{Fraction, ParticleFluence, ParticleFlux, Qty, Time};
use serde::{Deserialize, Serialize};

/// 陽子の標準のエネルギーの格子 [MeV]（AP-8 は 0.1〜400 MeV）。
pub fn proton_energies_mev() -> &'static [f64] {
    &[
        0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 30.0, 50.0, 100.0, 200.0, 400.0,
    ]
}

/// 電子の標準のエネルギーの格子 [MeV]（AE-8 は 0.04〜7 MeV）。
pub fn electron_energies_mev() -> &'static [f64] {
    &[0.04, 0.1, 0.25, 0.5, 1.0, 1.5, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]
}

/// 解析の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TrappedFluxInput {
    /// 軌道の標本の取り方（1 日分。`spec::daily_sampling`）
    pub sampling: OrbitSampling,
    /// ミッション期間（フルエンスの積算時間。`orbit.total_dose` と同じ）
    pub mission_duration: Qty<Time>,
}

/// エネルギー 1 つの結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TrappedFluxPoint {
    /// エネルギーのしきい値 [MeV]（積分フラックス `> E` の E。粒子のエネルギーの慣用の単位）
    pub energy_mev: f64,
    /// 軌道平均の積分全方向フラックス（`> E`）
    pub mean_flux: Qty<ParticleFlux>,
    /// 標本の中の最大のフラックス
    pub peak_flux: Qty<ParticleFlux>,
    /// ミッション期間のフルエンス（平均のフラックス × ミッション期間）
    pub fluence: Qty<ParticleFluence>,
    /// フラックスが 0 でない時間の割合（この E 以上の粒子のいる領域にいる時間）
    pub time_fraction: Fraction,
}

/// 地図 1 つ（粒子 × 太陽活動）の結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TrappedFluxCase {
    pub species: Species,
    pub solar: SolarCondition,
    /// 地図の名前（`ap8min` など）
    pub model: String,
    /// 組にした地磁気モデルの名前（元期の場。年変化なし）
    pub field: String,
    /// 地図の元期（年）
    pub epoch_year: i32,
    /// エネルギーごとの結果（昇順）
    pub spectrum: Vec<TrappedFluxPoint>,
    /// L が求まらず地図の外とした標本の数
    pub samples_without_l: usize,
    /// L の最小・最大（L が求まった標本。無ければ `None`）
    pub l_min: Option<f64>,
    pub l_max: Option<f64>,
}

/// 使った標本の取り方。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TrappedFluxUsed {
    pub span: Qty<Time>,
    pub step: Qty<Time>,
    pub samples: usize,
    pub mission_duration: Qty<Time>,
}

/// 解析の出力。判定は付けない（遮蔽・線量への換算は別の解析）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TrappedFluxOutput {
    /// 陽子の極小・極大、電子の極小・極大の順
    pub cases: Vec<TrappedFluxCase>,
    pub used: TrappedFluxUsed,
}

/// 捕捉粒子の軌道平均フラックスとミッション期間のフルエンス。
pub struct TrappedFlux;

impl Analysis for TrappedFlux {
    const ID: &'static str = "orbit.trapped_flux";
    const TITLE: &'static str = "捕捉粒子のフラックスとフルエンス";
    const TITLE_EN: &'static str = "Trapped particle flux and fluence";
    const AREA: AnalysisArea = AnalysisArea::Condition(ConditionTopic::Orbit);
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str = "軌道上の AP-8（陽子）・AE-8（電子）の積分フラックスの軌道平均・最大値と、ミッション期間のフルエンス（太陽活動の極小・極大。静的なモデル）";
    const DESCRIPTION_EN: &'static str = "Orbit-averaged and peak integral flux of trapped protons (AP-8) and electrons (AE-8) and the mission fluence, for solar minimum and maximum (static models)";
    type Input = TrappedFluxInput;
    type Output = TrappedFluxOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        compute(input, &RadiationTables::installed())
    }
}

/// 引く地図と場を引数に取る本体（試験は合成の地図を渡す）。
pub fn compute(
    input: &TrappedFluxInput,
    tables: &RadiationTables<'_>,
) -> Result<TrappedFluxOutput, AnalysisError> {
    compute_with(input, tables, BeltConvention::default())
}

/// [`compute`] に (L, B/B₀) の定義を選ばせる版（定義どうしの比較用。解析は既定の文献の定義）。
pub fn compute_with(
    input: &TrappedFluxInput,
    tables: &RadiationTables<'_>,
    convention: BeltConvention,
) -> Result<TrappedFluxOutput, AnalysisError> {
    ensure_positive("mission_duration", input.mission_duration.value())?;
    let missing = tables.missing();
    if !missing.is_empty() {
        return Err(AnalysisError::InvalidInput(format!(
            "The AP-8/AE-8 model tables are not installed (missing: {})",
            missing.join(", ")
        )));
    }
    let orbit = input.sampling.resolved_orbit()?;
    if orbit.central() != CentralBody::Earth {
        return Err(AnalysisError::InvalidInput(
            "The trapped-radiation models describe the Earth's belts; the orbit must be about the Earth".into(),
        ));
    }
    let points = input.sampling.states()?;
    let t: Vec<f64> = points.iter().map(|p| p.t_s).collect();
    let mut cases = Vec::with_capacity(4);
    for field_id in BeltField::ALL {
        let Some(field) = tables.field(field_id) else {
            continue;
        };
        let positions = positions_in(&field.at(field.epoch()), &points, convention);
        for model in TrappedModel::ALL
            .into_iter()
            .filter(|m| m.field() == field_id)
        {
            let Some(map) = tables.map(model) else {
                continue;
            };
            let case = case_for(
                model,
                map,
                field.name(),
                &positions,
                &t,
                input.mission_duration,
            );
            cases.push(case);
        }
    }
    cases.sort_by_key(|c| (c.species, c.solar));
    Ok(TrappedFluxOutput {
        cases,
        used: TrappedFluxUsed {
            span: input.sampling.span,
            step: input.sampling.step,
            samples: points.len(),
            mission_duration: input.mission_duration,
        },
    })
}

/// 並列に使うスレッドの上限（L の計算は標本ごとに独立で、場の評価が重い。結果は順序も値も直列と同じ）。
const MAX_THREADS: usize = 8;

/// 標本ごとの (L, B/B₀)。求まらない点は理由つきで `Err`（地図の外）。標本は塊に分けて並列に求め、元の順に並べる。
fn positions_in(
    field: &FieldSnapshot,
    points: &[OrbitPoint],
    convention: BeltConvention,
) -> Vec<Result<BeltPosition, NoL>> {
    let at =
        |p: &OrbitPoint| belt_position_with(field, eci_to_ecef(p.state.r_eci, p.epoch), convention);
    let threads = std::thread::available_parallelism()
        .map_or(1, std::num::NonZero::get)
        .min(MAX_THREADS);
    let chunk = points.len().div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = points
            .chunks(chunk)
            .map(|part| scope.spawn(move || part.iter().map(at).collect::<Vec<_>>()))
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_default())
            .collect()
    })
}

/// 1 つの地図のエネルギーごとの結果。
fn case_for(
    model: TrappedModel,
    map: &TrappedMap,
    field_name: &str,
    positions: &[Result<BeltPosition, NoL>],
    t: &[f64],
    duration: Qty<Time>,
) -> TrappedFluxCase {
    let grid = match model.species() {
        Species::Proton => proton_energies_mev(),
        Species::Electron => electron_energies_mev(),
    };
    let spectrum = grid
        .iter()
        .map(|&e| {
            let flux: Vec<f64> = positions
                .iter()
                .map(|p| {
                    p.as_ref()
                        .map_or(0.0, |p| map.integral_flux(p.l, p.b_over_b0, e))
                })
                .collect();
            point_for(e, &flux, t, duration)
        })
        .collect();
    let ls: Vec<f64> = positions.iter().flatten().map(|p| p.l).collect();
    TrappedFluxCase {
        species: model.species(),
        solar: model.solar(),
        model: model.id().to_owned(),
        field: field_name.to_owned(),
        epoch_year: map.header().epoch_year,
        spectrum,
        samples_without_l: positions.len() - ls.len(),
        l_min: ls.iter().copied().reduce(f64::min),
        l_max: ls.iter().copied().reduce(f64::max),
    }
}

/// フラックスの時系列（地図の単位 cm⁻² s⁻¹）から 1 つのエネルギーの結果（SI の量にして返す）。
fn point_for(
    energy_mev: f64,
    flux_cm2s: &[f64],
    t: &[f64],
    duration: Qty<Time>,
) -> TrappedFluxPoint {
    /// 1 cm⁻² = 1e4 m⁻²（地図の単位 cm⁻² s⁻¹ を SI に直す）。
    const PER_CM2_TO_PER_M2: f64 = 1.0e4;
    let mean_cm2s = time_mean(t, flux_cm2s);
    let mean = if mean_cm2s.is_finite() {
        mean_cm2s * PER_CM2_TO_PER_M2
    } else {
        0.0
    };
    let peak = flux_cm2s.iter().copied().fold(0.0, f64::max) * PER_CM2_TO_PER_M2;
    let on: Vec<bool> = flux_cm2s.iter().map(|&f| f > 0.0).collect();
    TrappedFluxPoint {
        energy_mev,
        mean_flux: Qty::from_raw(mean),
        peak_flux: Qty::from_raw(peak),
        fluence: Qty::from_raw(mean * duration.value()),
        time_fraction: time_fraction(t, &on, &[]),
    }
}

#[cfg(test)]
mod tests;
