//! ミッション幾何: フェーズごとの太陽距離・地球距離・太陽–地球–探査機の角（SEP）・太陽–探査機–地球の角（SPE）・光の伝搬時間。
//! 距離と角度の計算・極値の詰め方は `sat_astro::timeline::geometry`（ここは区間の決め方・サンプルの取り方・出力の形だけ）。
//!
//! 入力は区間（leg）の並び。1 区間 = 1 つのフェーズ（フェーズが無い設計は設計の軌道 1 つ）とそのフェーズで効いている軌道で、
//! 区間の期間は次の順に決める（どれを使ったかは出力の `span_basis`）: 軌道が 1 回限りの期間（[`Cycle::OneOff`]）を持てばそれ、
//! なければフェーズの `duration`、それも無ければ軌道の 1 周（[`Cycle::Periodic`]）。区間は元期から始まる。
//! サンプルは区間ごとに [`SAMPLES_PER_LEG`] 個に等分する（時系列の周回数ごとの刻みと違い、数日〜数年の区間でもサンプル数が一定）。
//! 極値はサンプルの前後を黄金分割で詰める（区間の中で単峰とみなすので、周回による速い変動は拾わない。幾何の量の変動は中心天体の軌道半径以下）。
//!
//! 後続の解析（回線・電源の最遠の条件）は、`legs[].earth_distance.max`（最遠の地球距離とその日付）と
//! `legs[].sun_distance.max`（太陽距離が最大の日付）を、フェーズの ID（`legs[].phase`）で引いて読む。
//! 判定・注記は付けない。

use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Channel, ChannelKind, ConditionTopic,
    Series, ensure_positive,
};
use sat_astro::ephem::{CentralBody, EphemerisSource, source_at};
use sat_astro::orbit::{Cycle, Trajectory};
use sat_astro::time::Epoch;
use sat_astro::timeline::geometry::{
    Extremum, GeometrySample, Metric, geometry_at, refined_extremes,
};
use sat_astro::timeline::sample_times;
use sat_units::typed::{Angle, Dimension, Length, Qty, Time};
use serde::{Deserialize, Serialize};

/// 区間ごとのサンプル数（等分）。
pub const SAMPLES_PER_LEG: usize = 720;

/// 区間の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GeometryLegInput {
    /// フェーズの ID（フェーズが無い設計の設計の軌道は `None`）
    pub phase: Option<String>,
    /// そのフェーズで効いている軌道
    pub orbit: Trajectory,
    /// フェーズの期間（書かれていれば）
    pub duration: Option<Qty<Time>>,
}

/// 幾何の入力。区間はフェーズの並びの順。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GeometryInput {
    pub legs: Vec<GeometryLegInput>,
}

/// 区間の期間をどこから決めたか。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum GeometrySpanBasis {
    /// 軌道の 1 回限りの期間（飛行時間など）
    OneOff,
    /// フェーズの `duration`
    PhaseDuration,
    /// 軌道の 1 周
    OneCycle,
}

/// 日付つきの値。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Dated<T> {
    pub epoch: Epoch,
    pub value: T,
}

/// 区間の中の最小と最大（それぞれの日付つき）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DatedRange<T> {
    pub min: Dated<T>,
    pub max: Dated<T>,
}

/// 区間 1 つの結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GeometryLeg {
    /// フェーズの ID（設計の軌道は `None`）
    pub phase: Option<String>,
    /// 軌道の中心天体
    pub central: CentralBody,
    /// 区間の始め（軌道の元期）
    pub start: Epoch,
    pub span: Qty<Time>,
    /// 期間の決め方
    pub span_basis: GeometrySpanBasis,
    /// サンプルの刻み（期間 / サンプル数）
    pub sample_step: Qty<Time>,
    /// 太陽の中心までの距離の範囲
    pub sun_distance: DatedRange<Qty<Length>>,
    /// 地球の中心までの距離の範囲（地球を回る軌道では軌道半径）
    pub earth_distance: DatedRange<Qty<Length>>,
    /// 太陽–地球–探査機の角の最小（合に最も近い日）
    pub sep_min: Dated<Qty<Angle>>,
    /// 地球までの片道の光の伝搬時間の最大
    pub light_time_max: Dated<Qty<Time>>,
    /// 天体の位置の出所（区間の始めと終わりの両方が DE440 の範囲内なら DE440、そうでなければ解析式）
    pub ephemeris: EphemerisSource,
    /// 区間の始めからの経過に対する、太陽距離・地球距離・SEP・SPE
    pub series: Series,
}

/// 幾何の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GeometryOutput {
    pub legs: Vec<GeometryLeg>,
}

/// ミッション幾何の解析。
pub struct Geometry;

impl Analysis for Geometry {
    const ID: &'static str = "orbit.geometry";
    const TITLE: &'static str = "ミッション幾何（太陽・地球との距離と角）";
    const TITLE_EN: &'static str = "Mission geometry";
    const AREA: AnalysisArea = AnalysisArea::Condition(ConditionTopic::Orbit);
    const KIND: AnalysisKind = AnalysisKind::TimeSeries;
    const DESCRIPTION: &'static str =
        "フェーズごとの太陽距離・地球距離・SEP・SPE・光の伝搬時間（最小・最大と日付、時系列）";
    const DESCRIPTION_EN: &'static str = "Sun and Earth distance, Sun-Earth-probe and Sun-probe-Earth angles and light time per phase: extremes with dates, and series";
    type Input = GeometryInput;
    type Output = GeometryOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        if input.legs.is_empty() {
            return Err(AnalysisError::InvalidInput(
                "No orbit legs to evaluate".into(),
            ));
        }
        let legs = input.legs.iter().map(leg).collect::<Result<_, _>>()?;
        Ok(GeometryOutput { legs })
    }
}

/// 区間の期間 [s] とその決め方: 1 回限りの期間 → フェーズの期間 → 軌道の 1 周。
pub fn leg_span(cycle: Cycle, duration: Option<Qty<Time>>) -> (f64, GeometrySpanBasis) {
    match (cycle, duration) {
        (Cycle::OneOff { span_s }, _) => (span_s, GeometrySpanBasis::OneOff),
        (Cycle::Periodic { .. }, Some(d)) => (d.value(), GeometrySpanBasis::PhaseDuration),
        (Cycle::Periodic { period_s }, None) => (period_s, GeometrySpanBasis::OneCycle),
    }
}

/// 極値を日付つきの量にする（量の種類は受け取る側の型で決まる）。
fn dated_at<D: Dimension>(start: Epoch, e: Extremum) -> Dated<Qty<D>> {
    Dated {
        epoch: start.plus_seconds(e.t_s),
        value: Qty::from_raw(e.value),
    }
}

/// サンプルの時系列（太陽距離・地球距離・SEP・SPE）。
fn series(times: Vec<f64>, samples: &[GeometrySample]) -> Series {
    Series {
        t: times,
        channels: vec![
            Channel::of::<Length>(
                "sun_distance",
                ChannelKind::SunDistance,
                samples.iter().map(|s| s.sun_distance_m).collect(),
            ),
            Channel::of::<Length>(
                "earth_distance",
                ChannelKind::EarthDistance,
                samples.iter().map(|s| s.earth_distance_m).collect(),
            ),
            Channel::of::<Angle>(
                "sep",
                ChannelKind::SunEarthProbeAngle,
                samples.iter().map(|s| s.sep_rad).collect(),
            ),
            Channel::of::<Angle>(
                "spe",
                ChannelKind::SunProbeEarthAngle,
                samples.iter().map(|s| s.spe_rad).collect(),
            ),
        ],
    }
}

fn leg(input: &GeometryLegInput) -> Result<GeometryLeg, AnalysisError> {
    let orbit = &input.orbit;
    let (span, span_basis) = leg_span(orbit.cycle(), input.duration);
    ensure_positive("span", span)?;
    let step = span / SAMPLES_PER_LEG as f64;
    let times = sample_times(span, step)?;
    let start = orbit.epoch();
    let samples = geometry_at(orbit, start, &times)?;
    let extremes = |metric| {
        refined_extremes(orbit, start, &samples, metric)?
            .ok_or_else(|| AnalysisError::InvalidInput("No samples".into()))
    };
    let (sun, earth) = (
        extremes(Metric::SunDistance)?,
        extremes(Metric::EarthDistance)?,
    );
    let ephemeris = match (source_at(start), source_at(start.plus_seconds(span))) {
        (EphemerisSource::De440, EphemerisSource::De440) => EphemerisSource::De440,
        _ => EphemerisSource::Analytic,
    };
    let series = series(times, &samples);
    Ok(GeometryLeg {
        phase: input.phase.clone(),
        central: orbit.central(),
        start,
        span: Qty::from_raw(span),
        span_basis,
        sample_step: Qty::from_raw(step),
        sun_distance: DatedRange {
            min: dated_at(start, sun.min),
            max: dated_at(start, sun.max),
        },
        earth_distance: DatedRange {
            min: dated_at(start, earth.min),
            max: dated_at(start, earth.max),
        },
        sep_min: dated_at(start, extremes(Metric::Sep)?.min),
        light_time_max: dated_at(start, extremes(Metric::LightTime)?.max),
        ephemeris,
        series,
    })
}

#[cfg(test)]
mod tests;
