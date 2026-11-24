//! 太陽を回る軌道の、太陽からの距離の最小・最大（ミッション期間の中。近日点・遠日点）。
//!
//! 解析ケースを「太陽距離が最小・最大の時刻」で作るために使う（`fune_service::edit::sun_distance`）。
//! カタログの解析ではなく、期間の標本（`spec::orbit_step_s` の刻み）で極値の近くを見つけ、その前後を黄金分割で詰める。
//! 距離と極値の詰め方は `sat_astro::timeline::geometry`（太陽距離 `body_distances_m`・`extremes_by`）で、ここは入力の検査と出力の型だけ。
//! 期間が半周より短ければ極値は期間の端になる（端も標本に含める）。

use crate::spec::{AnalysisError, orbit_step_s};
use sat_astro::ephem::CentralBody;
use sat_astro::orbit::{OrbitSpec, Trajectory};
use sat_astro::time::Epoch;
use sat_astro::timeline::geometry::{body_distances_m, extremes_by};
use sat_astro::timeline::sample_times;
use sat_units::typed::{Length, Qty, Time};

/// 太陽距離の極値を求める入力（軌道とミッション期間。`orbit.eclipse_season` の入力から β と刻みを除いたもの）。
#[derive(Debug, Clone)]
pub struct SunDistanceInput {
    pub orbit: OrbitSpec,
    /// 元期からの期間（ミッション期間）
    pub duration: Qty<Time>,
}

/// 太陽距離の極値の 1 点。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SunDistancePoint {
    pub epoch: Epoch,
    /// 太陽の中心からの距離
    pub distance: Qty<Length>,
}

/// 期間の中の太陽距離の最小と最大。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SunDistanceExtremes {
    pub min: SunDistancePoint,
    pub max: SunDistancePoint,
}

/// 太陽中心の軌道の、元期から期間までの太陽距離の最小・最大。
/// 中心天体が太陽でない軌道と、期間が正でない場合は `InvalidInput`。
pub fn sun_distance_extremes(
    input: &SunDistanceInput,
) -> Result<SunDistanceExtremes, AnalysisError> {
    let SunDistanceInput { orbit, duration } = input;
    if orbit.central != CentralBody::Sun {
        return Err(AnalysisError::InvalidInput(
            "The Sun distance extremes are defined for an orbit around the Sun (the central body is not the Sun)".into(),
        ));
    }
    let span = duration.value();
    if !(span > 0.0 && span.is_finite()) {
        return Err(AnalysisError::InvalidInput(
            "The mission duration must be positive".into(),
        ));
    }
    let times = sample_times(span, orbit_step_s(orbit.cycle()))?;
    let trajectory = Trajectory::from(orbit.clone());
    let sun = |times: &[f64]| body_distances_m(&trajectory, orbit.epoch, times, CentralBody::Sun);
    let values = sun(&times)?;
    let x = extremes_by(&times, &values, |t| Ok(sun(&[t])?[0]))?
        .ok_or_else(|| AnalysisError::InvalidInput("No samples".into()))?;
    let point = |e: sat_astro::timeline::geometry::Extremum| SunDistancePoint {
        epoch: orbit.epoch.plus_seconds(e.t_s),
        distance: Qty::from_raw(e.value),
    };
    Ok(SunDistanceExtremes {
        min: point(x.min),
        max: point(x.max),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sat_astro::constants::AU;
    use sat_astro::orbit::{OrbitElements, Propagator};

    /// 近日点 0.7 AU・遠日点 1.5 AU の日心楕円（平均近点角 `m0` [rad] から始める）。
    fn ellipse(m0: f64) -> OrbitSpec {
        let (rp, ra) = (0.7 * AU, 1.5 * AU);
        let (a, e) = ((rp + ra) / 2.0, (ra - rp) / (ra + rp));
        let elements = OrbitElements::with_defaults(
            Qty::from_raw(a),
            Qty::from_raw(0.1),
            Some(e),
            None,
            None,
            Some(Qty::from_raw(m0)),
        );
        let mut o = OrbitSpec::new(
            Epoch::parse("2030-01-01T00:00:00Z").expect("epoch"),
            elements,
            Propagator::Kepler,
        );
        o.central = CentralBody::Sun;
        o
    }

    fn extremes(o: &OrbitSpec, span_s: f64) -> Result<SunDistanceExtremes, AnalysisError> {
        sun_distance_extremes(&SunDistanceInput {
            orbit: o.clone(),
            duration: Qty::from_raw(span_s),
        })
    }

    /// 手計算: 近日点 = a(1−e) = 0.7 AU、遠日点 = a(1+e) = 1.5 AU。元期が近日点なら最小は t=0、
    /// 最大は半周（T/2）後。周期は ケプラーの第 3 法則 T = 2π√(a³/GM)。
    #[test]
    fn perihelion_and_aphelion_of_a_full_orbit() {
        let o = ellipse(0.0);
        let period = o.period_s();
        let x = extremes(&o, 1.2 * period).unwrap();
        assert!((x.min.distance.value() / (0.7 * AU) - 1.0).abs() < 1e-9);
        assert!((x.max.distance.value() / (1.5 * AU) - 1.0).abs() < 1e-9);
        assert!(x.min.epoch.seconds_since(o.epoch).abs() < 1.0);
        // 遠日点の時刻は半周後（黄金分割の精度は距離の 2 次なので、時刻は数百秒の幅で合えばよい）
        let t_max = x.max.epoch.seconds_since(o.epoch);
        assert!((t_max - period / 2.0).abs() < 600.0, "{t_max}");
    }

    /// 期間が半周より短ければ、遠日点には届かず最大は期間の端（元期が近日点で距離は単調増加）。
    #[test]
    fn short_span_puts_the_maximum_at_the_end() {
        let o = ellipse(0.0);
        let span = o.period_s() * 0.25;
        let x = extremes(&o, span).unwrap();
        assert!((x.max.epoch.seconds_since(o.epoch) - span).abs() < 1e-6);
        assert!(x.max.distance.value() < 1.5 * AU);
        assert!(x.max.distance.value() > x.min.distance.value());
    }

    #[test]
    fn rejects_non_solar_orbits_and_bad_spans() {
        let mut o = ellipse(0.0);
        assert!(extremes(&o, 0.0).is_err());
        o.central = CentralBody::Earth;
        assert!(extremes(&o, 1.0e7).is_err());
    }
}
