//! 太陽を回る軌道の、太陽からの距離の最小・最大（ミッション期間の中。近日点・遠日点）。
//!
//! 解析ケースを「太陽距離が最小・最大の時刻」で作るために使う（`fune_service::edit::sun_distance`）。
//! カタログの解析ではなく、期間の標本（`spec::orbit_step_s` の刻み）で極値の近くを見つけ、その前後を黄金分割で詰める。
//! 期間が半周より短ければ極値は期間の端になる（端も標本に含める）。

use crate::spec::{AnalysisError, orbit_step_s};
use sat_astro::ephem::CentralBody;
use sat_astro::orbit::OrbitSpec;
use sat_astro::time::Epoch;
use sat_astro::timeline::sample_times;
use sat_units::typed::{Length, Qty, Time};

/// 黄金分割の反復回数（区間は 0.618^N 倍になる。数十 ns のオーダーまで詰まる）。
const REFINE_ITERATIONS: usize = 60;

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
    let dist = |t: f64| -> Result<f64, AnalysisError> { Ok(orbit.state_at(t)?.r_eci.norm()) };
    let values: Vec<f64> = times.iter().map(|&t| dist(t)).collect::<Result<_, _>>()?;
    let extreme = |sign: f64| -> Result<SunDistancePoint, AnalysisError> {
        let i = (0..values.len())
            .min_by(|&a, &b| (sign * values[a]).total_cmp(&(sign * values[b])))
            .unwrap_or(0);
        let lo = times[i.saturating_sub(1)];
        let hi = times[(i + 1).min(times.len() - 1)];
        let t = golden_min(|t| dist(t).map(|d| sign * d), lo, hi)?;
        // 端の標本が極値なら端のまま（黄金分割は内側へ寄るので、端の値が良ければ端を採る）
        let t = [t, times[i]]
            .into_iter()
            .map(|t| dist(t).map(|d| (t, d)))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .min_by(|a, b| (sign * a.1).total_cmp(&(sign * b.1)))
            .map_or(times[i], |p| p.0);
        Ok(SunDistancePoint {
            epoch: orbit.epoch.plus_seconds(t),
            distance: Qty::from_raw(dist(t)?),
        })
    };
    Ok(SunDistanceExtremes {
        min: extreme(1.0)?,
        max: extreme(-1.0)?,
    })
}

/// 区間 `[lo, hi]` で `f` が最小になる点（黄金分割。区間の中で単峰とみなす）。
fn golden_min(
    f: impl Fn(f64) -> Result<f64, AnalysisError>,
    mut lo: f64,
    mut hi: f64,
) -> Result<f64, AnalysisError> {
    let g = (5f64.sqrt() - 1.0) / 2.0;
    let (mut x1, mut x2) = (hi - g * (hi - lo), lo + g * (hi - lo));
    let (mut f1, mut f2) = (f(x1)?, f(x2)?);
    for _ in 0..REFINE_ITERATIONS {
        if f1 < f2 {
            hi = x2;
            (x2, f2) = (x1, f1);
            x1 = hi - g * (hi - lo);
            f1 = f(x1)?;
        } else {
            lo = x1;
            (x1, f1) = (x2, f2);
            x2 = lo + g * (hi - lo);
            f2 = f(x2)?;
        }
    }
    Ok(0.5 * (lo + hi))
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
