//! 回線バジェットの距離（距離のモデルの選択はここだけ）。
//! - 地球を回る衛星: 局から見た斜距離（`sat_astro::ground::slant_range_m`）。最大は最低仰角、最小は天頂（= 高度）。
//! - 月・惑星・太陽を回る衛星: 地球から衛星までの距離（`sat_astro::ground::earth_range`、地心で近似）の、期間中の最小と最大。
//!   期間はミッション期間（無ければ [`DEFAULT_RANGE_SPAN`]）、1 回限りの軌道（惑星間の巡航）はその期間（飛行時間）。

use super::LinkBudgetLink;
use crate::spec::{AnalysisError, orbit_step_s};
use sat_astro::constants::{C_LIGHT, SECONDS_PER_DAY};
use sat_astro::ephem::CentralBody;
use sat_astro::ground::{EarthRange, earth_range, slant_range_m};
use sat_astro::orbit::{Cycle, Trajectory};
use sat_astro::time::Epoch;
use sat_units::typed::{Length, Qty, Time};
use serde::{Deserialize, Serialize};

/// 距離を評価する期間の既定（ミッション期間が書かれていないとき）。**既定値はここだけ。**
/// 1 年（回帰年）: 月の公転（27.3 日）と地球の日心距離の 1 周期を含む。火星などの会合周期（約 2.1 年）には足りないので、
/// 惑星以遠はミッション期間（`input = "mission_duration"` の要求）を書く。
pub const DEFAULT_RANGE_SPAN: Qty<Time> = Qty::from_raw(365.25 * SECONDS_PER_DAY);

/// 距離の標本数の上限（長い期間の刻みが細かくなりすぎないようにする）。
const MAX_RANGE_SAMPLES: f64 = 50_000.0;

/// 距離の求め方。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum LinkDistanceModel {
    /// 地球を回る衛星の、局から見た斜距離（最大は最低仰角、最小は天頂）
    EarthOrbit,
    /// 月・惑星・太陽を回る衛星の、地球（地心）から衛星までの距離（期間中の最大と最小）
    EarthToBody,
}

/// 回線 1 本の距離。評価（回線余裕）は `max`、最悪の PFD は `min` で行う。
#[derive(Debug, Clone, Copy)]
pub(super) struct LinkRange {
    pub model: LinkDistanceModel,
    pub min_m: f64,
    pub max_m: f64,
    /// 最小・最大になった時刻（地球を回る衛星では決まらないので `None`）
    pub min_epoch: Option<Epoch>,
    pub max_epoch: Option<Epoch>,
}

impl LinkRange {
    /// 最大の距離を光が進む時間（片道）。
    pub fn light_time(&self) -> Qty<Time> {
        Qty::from_raw(self.max_m / C_LIGHT)
    }

    pub fn min(&self) -> Qty<Length> {
        Qty::from_raw(self.min_m)
    }

    pub fn max(&self) -> Qty<Length> {
        Qty::from_raw(self.max_m)
    }
}

/// 回線の中に中心天体が地球でないものがあれば、地球から衛星までの距離の範囲を求める（回線の数だけ繰り返さない）。
pub(super) fn deep_space_range(
    links: &[LinkBudgetLink],
    orbit: Option<&Trajectory>,
    duration: Option<Qty<Time>>,
) -> Result<Option<EarthRange>, AnalysisError> {
    if links.iter().all(|l| l.central == CentralBody::Earth) {
        return Ok(None);
    }
    let orbit = orbit.ok_or_else(|| {
        AnalysisError::InvalidInput(
            "The orbit is needed to compute the Earth distance of a spacecraft around another body"
                .to_owned(),
        )
    })?;
    // 1 回限りの軌道（惑星間の巡航など）は、その期間（飛行時間）の距離を見る。ミッション期間は軌道の外まで含むので使わない。
    let span = match orbit.cycle() {
        Cycle::OneOff { span_s } => span_s,
        Cycle::Periodic { .. } => duration.unwrap_or(DEFAULT_RANGE_SPAN).value(),
    };
    if !(span > 0.0 && span.is_finite()) {
        return Err(AnalysisError::InvalidInput(
            "The evaluation span of the distance must be positive".to_owned(),
        ));
    }
    let step = orbit_step_s(orbit.cycle()).max(span / MAX_RANGE_SAMPLES);
    Ok(Some(earth_range(
        orbit,
        orbit.epoch(),
        span,
        step.min(span),
    )?))
}

/// 回線の距離。地球を回る衛星は斜距離、ほかの天体を回る衛星は `deep` の範囲（[`deep_space_range`] が求めたもの）。
pub(super) fn link_range(
    link: &LinkBudgetLink,
    deep: Option<&EarthRange>,
) -> Result<LinkRange, AnalysisError> {
    match (link.central, deep) {
        (CentralBody::Earth, _) => {
            let (radius, altitude) = (link.central.radius_m(), link.altitude.value());
            Ok(LinkRange {
                model: LinkDistanceModel::EarthOrbit,
                min_m: slant_range_m(radius, altitude, std::f64::consts::FRAC_PI_2),
                max_m: slant_range_m(radius, altitude, link.elevation.value()),
                min_epoch: None,
                max_epoch: None,
            })
        }
        (_, Some(r)) => Ok(LinkRange {
            model: LinkDistanceModel::EarthToBody,
            min_m: r.min_m,
            max_m: r.max_m,
            min_epoch: Some(r.min_epoch),
            max_epoch: Some(r.max_epoch),
        }),
        (_, None) => Err(AnalysisError::InvalidInput(format!(
            "{}: the Earth distance range was not computed",
            link.name
        ))),
    }
}
