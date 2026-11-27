//! 太陽を回る軌道の、太陽からの距離の最小・最大（ミッション期間の中。近日点・遠日点）。
//!
//! 解析ケースを「太陽距離が最小・最大の時刻」で作るために使う（`fune_service::edit::sun_distance`）。
//! カタログの解析ではなく、期間のサンプル（`spec::orbit_step_s` の刻み）で極値の近くを見つけ、その前後を黄金分割で詰める。
//! 距離と極値の詰め方は `sat_astro::timeline::geometry`（太陽距離 `body_distances_m`・`extremes_by`）で、ここは入力の検査と出力の型だけ。
//! 期間が半周より短ければ極値は期間の端になる（端もサンプルに含める）。

use crate::spec::{AnalysisError, orbit_step_s};
use sat_astro::ephem::CentralBody;
use sat_astro::orbit::{OrbitSpec, Trajectory};
use sat_astro::time::Epoch;
use sat_astro::timeline::geometry::{body_distances_m, breakpoints_from, extremes_by_split};
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
/// 中心天体が太陽でない軌道と、期間が正でない場合は `InvalidInput`（太陽を回る軌道の近日点・遠日点を求める入口。
/// 中心天体を問わない最小・最大は [`sun_distance_extremes_over`]）。
pub fn sun_distance_extremes(
    input: &SunDistanceInput,
) -> Result<SunDistanceExtremes, AnalysisError> {
    let SunDistanceInput { orbit, duration } = input;
    if orbit.central != CentralBody::Sun {
        return Err(AnalysisError::InvalidInput(
            "The Sun distance extremes are defined for an orbit around the Sun (the central body is not the Sun)".into(),
        ));
    }
    sun_distance_extremes_over(&Trajectory::from(orbit.clone()), duration.value())
}

/// サンプルの数の上限（中心天体の周りを速く回る軌道で、長い期間の刻みが細かくなりすぎないようにする）。
const MAX_SAMPLES: f64 = 50_000.0;

/// 軌道（どの出どころ・どの中心天体でもよい）の、元期から `span_s` 秒までの太陽距離の最小・最大と日付。
/// サンプルの刻みは軌道の繰り返しの 200 分の 1（`spec::orbit_step_s`）か、期間の [`MAX_SAMPLES`] 分の 1 の大きい方で、
/// 極値の近くは黄金分割で詰める（`sat_astro::timeline::geometry::extremes_by_split`。弧の連鎖の境をまたがない）。**距離の計算はミッション幾何の 1 か所**
/// （`body_distances_m`）で、ここはサンプルの取り方と型だけ。期間が正でなければ `InvalidInput`。
pub fn sun_distance_extremes_over(
    orbit: &Trajectory,
    span_s: f64,
) -> Result<SunDistanceExtremes, AnalysisError> {
    if !(span_s > 0.0 && span_s.is_finite()) {
        return Err(AnalysisError::InvalidInput(
            "The mission duration must be positive".into(),
        ));
    }
    let step = orbit_step_s(orbit.cycle()).max(span_s / MAX_SAMPLES);
    let times = sample_times(span_s, step)?;
    let start = orbit.epoch();
    let sun = |times: &[f64]| body_distances_m(orbit, start, times, CentralBody::Sun);
    let values = sun(&times)?;
    let breaks = breakpoints_from(orbit, start);
    let x = extremes_by_split(&times, &values, &breaks, |t| Ok(sun(&[t])?[0]))?
        .ok_or_else(|| AnalysisError::InvalidInput("No samples".into()))?;
    let point = |e: sat_astro::timeline::geometry::Extremum| SunDistancePoint {
        epoch: start.plus_seconds(e.t_s),
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

    /// 太陽中心の 1 AU の円（円速度）の状態ベクトルの軌道: どの出どころでも同じ入口で、太陽距離は期間を通して 1 AU の近く。
    /// 許容は 1e-2 AU（惑星の摂動の変位の上限。`fune-projection` の状態軌道のテストと同じ見積もり）。
    #[test]
    fn a_state_orbit_on_a_one_au_circle_stays_at_one_au() {
        use sat_astro::math::Vec3;
        use sat_astro::orbit::{OrbitState, StateSpec};
        let v = (CentralBody::Sun.gm() / AU).sqrt();
        let state = OrbitState {
            r_eci: Vec3::new(AU, 0.0, 0.0),
            v_eci: Vec3::new(0.0, v, 0.0),
        };
        let span = 200.0 * 86_400.0;
        let epoch = Epoch::parse("2030-01-01T00:00:00Z").expect("epoch");
        let t = Trajectory::State(StateSpec::with_defaults(
            epoch,
            CentralBody::Sun,
            state,
            span,
            None,
        ));
        let x = sun_distance_extremes_over(&t, span).unwrap();
        for d in [x.min.distance.value(), x.max.distance.value()] {
            assert!((d / AU - 1.0).abs() < 1e-2, "{} AU", d / AU);
        }
    }

    /// 中心天体が太陽でない軌道（火星を回る衛星）でも、太陽距離の最大は火星の日心距離の最大（遠日点 1.666 AU 付近）に衛星の軌道半径を足した値以下。
    #[test]
    fn extremes_over_work_about_other_bodies() {
        let mut o = ellipse(0.0);
        o.central = CentralBody::Mars;
        o.elements.semi_major_axis = Qty::from_raw(4.0e6);
        o.elements.eccentricity = 0.0;
        let span = 700.0 * 86_400.0;
        let x = sun_distance_extremes_over(&Trajectory::from(o), span).unwrap();
        // 火星の日心距離は 1.381〜1.666 AU（軌道の公称値）、衛星の軌道半径は 4000 km = 2.7e-5 AU
        assert!((1.38..1.67).contains(&(x.min.distance.value() / AU)));
        assert!((1.38..1.67).contains(&(x.max.distance.value() / AU)));
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
