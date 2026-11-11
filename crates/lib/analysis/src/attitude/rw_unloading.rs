//! RW アンロード頻度（`attitude.rw_unloading`）。
//!
//! 1. 軌道と姿勢則の時系列（`sat_astro::timeline`）の各点で擾乱トルクを求める（[`super::disturbance`]）。
//! 2. トルクを ECI に直して `sat_astro::ode::rk4_linear_step`（`sat_astro::ode::rk4_step`）で積分し、ホイールが吸収する角運動量 `H_eci(t)` を得る
//!    （刻みの間のトルクは両端の線形補間。機体の角速度による角運動量 I·ω は一定とみなして含めない）。
//! 3. 機体座標系に直した `h(t)` をホイールの組に配る（[`super::wheels`]、擬似逆行列）。
//! 4. 回り切った周回ごとに `H_eci` の増分（永年成分）と、そこからの直線のずれの最大（周期成分）を分ける。
//! 5. アンロード間隔 = (永年成分の向きで蓄えられる量の最小 − 周期成分) / 1 周あたりの永年成分 × 周期。

use super::disturbance::{DisturbanceKind, DisturbanceSpacecraft, DisturbanceTorques};
use super::wheels::{RwWheel, WheelSet, usable_momentum_fraction};
use crate::spec::{
    Analysis, AnalysisError, AnalysisKind, Check, Series, SpanFallback, System, orbit_ranges,
    ratio_margin, span_fallback,
};
use sat_astro::constants::SECONDS_PER_DAY;
use sat_astro::math::Vec3;
use sat_astro::timeline::{self, TimelineSample, TimelineSpec, time_mean};
use sat_units::typed::{AngularMomentum, Fraction, Qty, Time, Torque};
use serde::{Deserialize, Serialize};

/// トルクの余裕（[`crate::spec::ratio_margin`]）の警告の幅。0 なので、最大トルクが必要なトルク以上なら ok、足りなければ違反の二段階。
pub const TORQUE_MARGIN_WARNING_BAND: Fraction = Fraction::new(0.0);

/// 1 周あたりの永年成分が容量のこの割合以下なら、永年成分は無い（アンロード不要）とみなす。
const NEGLIGIBLE_SECULAR_RATIO: f64 = 1e-12;

/// 容量が 0 のときの比較の下限 [N·m·s]（0 割りを避ける）。
const MIN_CAPACITY_FOR_RATIO: f64 = 1e-30;

/// RW アンロード頻度の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RwUnloadingInput {
    pub timeline: TimelineSpec,
    pub spacecraft: DisturbanceSpacecraft,
    /// ホイール（台数・配置は任意。軸が 3 次元を張ること）
    pub wheels: Vec<RwWheel>,
    /// 最大角運動量のうち使ってよい割合（0〜1）。省略時 [`super::wheels::DEFAULT_USABLE_MOMENTUM_FRACTION`]
    #[serde(default)]
    pub usable_momentum_fraction: Option<Fraction>,
    /// アンロード間隔の要求下限（省略時は判定しない）
    #[serde(default)]
    pub required_unload_interval: Option<Qty<Time>>,
}

/// RW アンロード頻度の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RwUnloadingOutput {
    pub period: Qty<Time>,
    /// 回り切った周回の数（0 なら全期間から 1 周分に外挿）
    pub complete_orbits: usize,
    /// 1 周に満たないときの代わりの集計（回り切った周回があれば `None`）
    pub span_fallback: Option<SpanFallback>,
    /// 使った「最大角運動量のうち使ってよい割合」（入力が無ければ既定値）
    pub usable_momentum_fraction: Fraction,
    /// 種類別の擾乱トルク（大きさの最大・平均）
    pub disturbances: Vec<RwUnloadingDisturbance>,
    /// 合計の擾乱トルクの大きさの最大
    #[serde(serialize_with = "sat_units::typed::TorqueKind::serialize")]
    pub peak_disturbance: Qty<Torque>,
    /// 1 周あたりの永年成分 [N·m·s]（ECI。周回の平均）
    pub secular_per_orbit_eci: Vec3,
    /// 1 周あたりの永年成分の大きさ
    pub secular_per_orbit: Qty<AngularMomentum>,
    /// 周期成分の振幅（周回ごとの永年の直線からのずれの最大）
    pub cyclic_amplitude: Qty<AngularMomentum>,
    /// 期間中の機体の角運動量の大きさの最大
    pub peak_momentum: Qty<AngularMomentum>,
    /// 永年成分の向きに蓄えられる量（使える割合込み、姿勢が変わる中での最小）
    pub capacity: Qty<AngularMomentum>,
    /// アンロード間隔。永年成分が無ければ `None`、周期成分だけで容量を超えるなら 0
    pub unload_interval: Option<Qty<Time>>,
    /// 1 日あたりのアンロード回数（永年成分が無ければ 0、間隔が 0 なら `None`）
    pub unloads_per_day: Option<f64>,
    /// 解析期間中にどれかのホイールが使える上限を超えた最初の時刻
    pub first_saturation: Option<Qty<Time>>,
    /// トルクの余裕 = 最大トルク / 必要なトルク − 1 のホイールのうち最小（[`crate::spec::ratio_margin`]）。
    /// 擾乱が 0 なら `None`（余裕は無限大）
    pub min_torque_margin: Option<Fraction>,
    /// ホイール別（入力の順）
    pub wheels: Vec<RwUnloadingWheelResult>,
    /// 時系列: `torque:<種類>`・`torque:total`・`momentum:{x,y,z,norm}`（機体座標系）・`wheel:<id>`
    pub series: Series,
    pub checks: Vec<Check>,
}

/// 1 種類の擾乱トルク。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RwUnloadingDisturbance {
    /// 擾乱の種類（表示名は service が語彙で引く）
    pub key: DisturbanceKind,
    #[serde(serialize_with = "sat_units::typed::TorqueKind::serialize")]
    pub peak: Qty<Torque>,
    #[serde(serialize_with = "sat_units::typed::TorqueKind::serialize")]
    pub mean: Qty<Torque>,
}

/// 1 台のホイールの結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RwUnloadingWheelResult {
    pub id: String,
    /// 期間中の角運動量の大きさの最大
    pub peak_momentum: Qty<AngularMomentum>,
    /// 最大角運動量 × 使える割合 に対する割合
    pub momentum_utilization: Fraction,
    /// 擾乱を打ち消すのに要るトルクの最大
    #[serde(serialize_with = "sat_units::typed::TorqueKind::serialize")]
    pub peak_torque: Qty<Torque>,
    /// 最大トルクに対する割合
    pub torque_utilization: Fraction,
}

/// RW アンロード頻度。
pub struct RwUnloading;

impl Analysis for RwUnloading {
    const ID: &'static str = "attitude.rw_unloading";
    const TITLE: &'static str = "RW アンロード頻度";
    const SYSTEM: System = System::Attitude;
    const KIND: AnalysisKind = AnalysisKind::TimeSeries;
    const DESCRIPTION: &'static str = "擾乱トルクから蓄積角運動量とアンロード間隔を求める";
    const TITLE_EN: &'static str = "RW unloading frequency";
    const DESCRIPTION_EN: &'static str =
        "Accumulated momentum and unloading interval from disturbance torques";
    type Input = RwUnloadingInput;
    type Output = RwUnloadingOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        input.spacecraft.validate()?;
        let usable = usable_momentum_fraction(input.usable_momentum_fraction)?;
        let set = WheelSet::new(&input.wheels)?;
        let samples = timeline::build(&input.timeline)?;
        let trace = Trace::build(
            &input.spacecraft,
            input.timeline.orbit().central,
            &set,
            &samples,
        );
        Ok(summarize(input, usable, &set, &samples, &trace))
    }
}

fn summarize(
    input: &RwUnloadingInput,
    usable: Fraction,
    set: &WheelSet,
    samples: &[TimelineSample],
    tr: &Trace,
) -> RwUnloadingOutput {
    let period_s = input.timeline.orbit().period_s();
    let ranges = orbit_ranges(&tr.t, period_s);
    let (secular, cyclic) = secular_and_cyclic(&tr.t, &tr.h_eci, &ranges, period_s);
    let limits: Vec<f64> = input
        .wheels
        .iter()
        .map(|w| w.max_momentum.value() * usable.value())
        .collect();
    let capacity = capacity(set, samples, secular, &limits);
    let interval = unload_interval(secular.norm(), cyclic, capacity, period_s);
    let wheels = wheel_results(input, tr, &limits);
    let max_utilization = wheels
        .iter()
        .map(|w| w.torque_utilization.value())
        .fold(0.0, f64::max);
    let disturbances = disturbance_summary(&tr.t, &tr.torques);
    let mut out = RwUnloadingOutput {
        period: Qty::from_raw(period_s),
        complete_orbits: ranges.len(),
        span_fallback: span_fallback(ranges.len(), SpanFallback::SecularExtrapolated),
        usable_momentum_fraction: usable,
        peak_disturbance: Qty::from_raw(
            tr.torques
                .iter()
                .map(|d| d.total().norm())
                .fold(0.0, f64::max),
        ),
        disturbances,
        secular_per_orbit_eci: secular,
        secular_per_orbit: Qty::from_raw(secular.norm()),
        cyclic_amplitude: Qty::from_raw(cyclic),
        peak_momentum: Qty::from_raw(tr.h_body.iter().map(|h| h.norm()).fold(0.0, f64::max)),
        capacity: Qty::from_raw(capacity),
        unload_interval: interval.map(Qty::from_raw),
        unloads_per_day: match interval {
            None => Some(0.0),
            Some(i) if i > 0.0 => Some(SECONDS_PER_DAY / i),
            Some(_) => None,
        },
        first_saturation: first_saturation(tr, &limits).map(Qty::from_raw),
        // 最も余裕の無いホイール: 最大トルク / 必要なトルク の最小 = 1 / 利用率の最大
        min_torque_margin: ratio_margin(1.0, max_utilization),
        wheels,
        series: series(input, tr),
        checks: Vec::new(),
    };
    out.checks = verdicts(input, &out);
    out
}

/// 永年成分の向き（ECI）に蓄えられる量 [N·m·s] の、姿勢が変わる中での最小。
/// 永年成分が無い（向きが決まらない）ときは機体の ±x・±y・±z のうち最も小さいもの。
fn capacity(set: &WheelSet, samples: &[TimelineSample], secular_eci: Vec3, limits: &[f64]) -> f64 {
    let c = if secular_eci.normalized().is_some() {
        samples
            .iter()
            .map(|s| set.capacity_along(s.to_body(secular_eci), limits))
            .fold(f64::INFINITY, f64::min)
    } else {
        [Vec3::X, Vec3::Y, Vec3::Z]
            .iter()
            .map(|&d| set.capacity_along(d, limits))
            .fold(f64::INFINITY, f64::min)
    };
    if c.is_finite() { c } else { 0.0 }
}

/// アンロード間隔 [s]。永年成分がほぼ 0 なら `None`、周期成分だけで容量を超えるなら 0。
fn unload_interval(secular: f64, cyclic: f64, capacity: f64, period_s: f64) -> Option<f64> {
    let usable = capacity - cyclic;
    if usable <= 0.0 {
        return Some(0.0);
    }
    if secular <= NEGLIGIBLE_SECULAR_RATIO * capacity.max(MIN_CAPACITY_FOR_RATIO) {
        return None;
    }
    Some(usable / secular * period_s)
}

fn first_saturation(tr: &Trace, limits: &[f64]) -> Option<f64> {
    (0..tr.t.len())
        .find(|&k| {
            tr.wheel_h
                .iter()
                .zip(limits)
                .any(|(h, lim)| h[k].abs() > *lim)
        })
        .map(|k| tr.t[k])
}

fn wheel_results(
    input: &RwUnloadingInput,
    tr: &Trace,
    limits: &[f64],
) -> Vec<RwUnloadingWheelResult> {
    let peak = |v: &[f64]| v.iter().map(|x| x.abs()).fold(0.0, f64::max);
    input
        .wheels
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let (ph, pt) = (peak(&tr.wheel_h[i]), peak(&tr.wheel_tau[i]));
            RwUnloadingWheelResult {
                id: w.id.clone(),
                peak_momentum: Qty::from_raw(ph),
                momentum_utilization: Fraction::new(ph / limits[i]),
                peak_torque: Qty::from_raw(pt),
                torque_utilization: Fraction::new(pt / w.max_torque.value()),
            }
        })
        .collect()
}

/// 種類ごとの擾乱トルクの大きさの最大と時間平均（台形則、`sat_astro::timeline::time_mean`）。
fn disturbance_summary(t: &[f64], torques: &[DisturbanceTorques]) -> Vec<RwUnloadingDisturbance> {
    DisturbanceKind::ALL
        .iter()
        .enumerate()
        .map(|(i, &key)| {
            let mags: Vec<f64> = torques.iter().map(|d| d.by_kind()[i].norm()).collect();
            RwUnloadingDisturbance {
                key,
                peak: Qty::from_raw(mags.iter().copied().fold(0.0, f64::max)),
                mean: Qty::from_raw(nan_to_zero(time_mean(t, &mags))),
            }
        })
        .collect()
}

/// 空の時系列の平均（NaN）を 0 にする。
fn nan_to_zero(v: f64) -> f64 {
    if v.is_finite() { v } else { 0.0 }
}

mod report;
mod trace;
use report::{series, verdicts};
use trace::{Trace, secular_and_cyclic};

#[cfg(test)]
mod tests;
