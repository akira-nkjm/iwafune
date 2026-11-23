//! 地球から、他の天体（月・惑星・太陽）を回る衛星までの距離の範囲。
//! 距離は地心から衛星まで `|r_中心天体(t) + r_衛星(t)|`（中心天体の地心位置は [`crate::ephem`]、衛星は中心天体からの相対位置）。
//! 地上局までではなく地心までとみなす（地球の半径 6378 km の差。月で 1.7 %、火星以遠で 0.01 % 以下）。

use crate::AstroError;
use crate::orbit::Trajectory;
use crate::time::Epoch;
use crate::timeline::sample_times;

/// 距離の最小・最大と、その時刻（標本の刻みで見つけた値）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EarthRange {
    /// 最小の距離 [m]
    pub min_m: f64,
    /// 最小の距離になった時刻
    pub min_epoch: Epoch,
    /// 最大の距離 [m]
    pub max_m: f64,
    /// 最大の距離になった時刻
    pub max_epoch: Epoch,
}

/// 地心から衛星までの距離の最小・最大を、`start` から `span_s` 秒の間 `step_s` 秒刻みで求める。
///
/// 軌道 `orbit` の中心天体が地球でも同じ式で求まる（地心距離そのもの）。刻みの間の極値は拾わないので、
/// 刻みは中心天体を回る軌道の周期より十分細かくする（衛星の周回による変動は中心天体の軌道半径以下）。
pub fn earth_range(
    orbit: &Trajectory,
    start: Epoch,
    span_s: f64,
    step_s: f64,
) -> Result<EarthRange, AstroError> {
    let times = sample_times(span_s, step_s)?;
    let offset = start.seconds_since(orbit.epoch());
    let abs: Vec<f64> = times.iter().map(|t| t + offset).collect();
    let states = orbit.states_at(&abs)?;
    let mut min = (f64::INFINITY, start);
    let mut max = (f64::NEG_INFINITY, start);
    for (&t, state) in times.iter().zip(&states) {
        let epoch = start.plus_seconds(t);
        let d = (orbit.central().position_eci_m(epoch) + state.r_eci).norm();
        if d < min.0 {
            min = (d, epoch);
        }
        if d > max.0 {
            max = (d, epoch);
        }
    }
    Ok(EarthRange {
        min_m: min.0,
        min_epoch: min.1,
        max_m: max.0,
        max_epoch: max.1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{AU, SECONDS_PER_DAY};
    use crate::ephem::CentralBody;
    use crate::orbit::{OrbitElements, OrbitSpec, Propagator};
    use sat_units::typed::Qty;

    fn epoch() -> Epoch {
        Epoch::parse("2027-04-01T00:00:00Z").expect("epoch")
    }

    /// 日心の円軌道（半径 1.5237 AU・黄道面）: 地球の日心距離は 0.9833〜1.0167 AU なので、
    /// 最小は a − r_E ∈ [0.5070, 0.5404] AU、最大は a + r_E ∈ [2.5070, 2.5404] AU（手計算）。
    #[test]
    fn heliocentric_circle_ranges_from_a_minus_one_to_a_plus_one_au() {
        let a = 1.5237 * AU;
        let elements = OrbitElements {
            semi_major_axis: Qty::from_raw(a),
            eccentricity: 0.0,
            // 黄道傾斜角 ε₀ = 23.4393°、昇交点が春分点方向なら軌道面は黄道面
            inclination: Qty::from_raw(23.4393_f64.to_radians()),
            raan: Qty::from_raw(0.0),
            arg_perigee: Qty::from_raw(0.0),
            mean_anomaly: Qty::from_raw(0.0),
        };
        let mut orbit = OrbitSpec::new(epoch(), elements, Propagator::Kepler);
        orbit.central = CentralBody::Sun;
        let range = earth_range(
            &Trajectory::from(orbit.clone()),
            epoch(),
            5.0 * 365.25 * SECONDS_PER_DAY,
            6.0 * 3600.0,
        )
        .expect("range");
        let (min, max) = (range.min_m / AU, range.max_m / AU);
        assert!((0.5070..=0.5404).contains(&min), "{min}");
        assert!((2.5070..=2.5404).contains(&max), "{max}");
        assert!(range.min_epoch != range.max_epoch);
    }

    /// 月周回の円軌道（高度 100 km）: 距離は月の地心距離 ± 軌道半径（1837.4 km）に収まり、月の距離そのもの
    /// （約 356 000〜407 000 km）にも入る。
    #[test]
    fn lunar_orbiter_stays_within_moon_distance_plus_minus_orbit_radius() {
        let orbit = OrbitSpec::circular_about(
            CentralBody::Moon,
            epoch(),
            Qty::from_raw(100.0e3),
            Qty::from_raw(0.0),
            Qty::from_raw(0.0),
        );
        let span = 30.0 * SECONDS_PER_DAY;
        let r = earth_range(&Trajectory::from(orbit.clone()), epoch(), span, 600.0).expect("range");
        let a = orbit.elements.semi_major_axis.value();
        let centre = |e: Epoch| CentralBody::Moon.position_eci_m(e).norm();
        let (mut lo, mut hi) = (f64::INFINITY, 0.0_f64);
        for k in 0..=(span / 600.0) as usize {
            let d = centre(epoch().plus_seconds(k as f64 * 600.0));
            lo = lo.min(d);
            hi = hi.max(d);
        }
        assert!(r.min_m >= lo - a && r.min_m <= lo + a, "{} {lo}", r.min_m);
        assert!(r.max_m <= hi + a && r.max_m >= hi - a, "{} {hi}", r.max_m);
        assert!((3.5e8..4.1e8).contains(&r.min_m) && (3.5e8..4.1e8).contains(&r.max_m));
    }
}
