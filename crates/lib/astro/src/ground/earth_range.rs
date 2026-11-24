//! 地球から、他の天体（月・惑星・太陽）を回る衛星までの距離の範囲と、太陽–地球–探査機の角（SEP）の最小。
//! 距離と角は地心から（`timeline::geometry` の標本。中心天体の地心位置は [`crate::ephem`]、衛星は中心天体からの相対位置）。
//! 地上局までではなく地心までとみなす（地球の半径 6378 km の差。月で 1.7 %、火星以遠で 0.01 % 以下）。

use crate::AstroError;
use crate::orbit::Trajectory;
use crate::time::Epoch;
use crate::timeline::geometry::{
    Metric, extreme_indices, geometry_at, refined_extremes, time_below_s,
};
use crate::timeline::sample_times;

/// 距離の最小・最大と、その時刻（標本の刻みで見つけた値）。あわせて SEP の最小と、SEP がしきい値を下回る時間。
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
    /// 太陽–地球–探査機の角（SEP）の最小 [rad]（標本の前後を詰めた値。合に最も近い）
    pub sep_min_rad: f64,
    /// SEP が最小になった時刻
    pub sep_min_epoch: Epoch,
    /// SEP がしきい値を下回っていた時間 [s]（標本の間は直線で結ぶ。`time_below_s`）
    pub sep_below_s: f64,
}

/// 距離の範囲だけがほしいとき（SEP の下回り時間 [`EarthRange::sep_below_s`] を使わない）の入口。
/// [`earth_range_with_sep`] のしきい値を 0 にしたもの（SEP は 0 未満にならないので下回り時間は 0）。
pub fn earth_range(
    orbit: &Trajectory,
    start: Epoch,
    span_s: f64,
    step_s: f64,
) -> Result<EarthRange, AstroError> {
    earth_range_with_sep(orbit, start, span_s, step_s, 0.0)
}

/// 地心から衛星までの距離の最小・最大と SEP の最小を、`start` から `span_s` 秒の間 `step_s` 秒刻みで求める。
/// `sep_threshold_rad` は SEP の下回り時間（[`EarthRange::sep_below_s`]）を数えるしきい値 [rad]。
///
/// 軌道 `orbit` の中心天体が地球でも同じ式で求まる（地心距離そのもの）。刻みの間の極値は拾わないので、
/// 刻みは中心天体を回る軌道の周期より十分細かくする（衛星の周回による変動は中心天体の軌道半径以下）。
/// 距離と角は `geometry_at`（`timeline::geometry`。**距離・角度を出すのはそこだけ**）、距離の最小・最大の選び方は
/// [`extreme_indices`]（同値は最初の標本）、SEP の最小は `refined_extremes`。
pub fn earth_range_with_sep(
    orbit: &Trajectory,
    start: Epoch,
    span_s: f64,
    step_s: f64,
    sep_threshold_rad: f64,
) -> Result<EarthRange, AstroError> {
    let times = sample_times(span_s, step_s)?;
    let samples = geometry_at(orbit, start, &times)?;
    let d: Vec<f64> = samples.iter().map(|s| s.earth_distance_m).collect();
    let sep: Vec<f64> = samples.iter().map(|s| s.sep_rad).collect();
    let (imin, imax) = extreme_indices(&d)
        .ok_or_else(|| AstroError::InvalidInput("No samples for the Earth distance".into()))?;
    let sep_min = refined_extremes(orbit, start, &samples, Metric::Sep)?
        .ok_or_else(|| AstroError::InvalidInput("No samples for the SEP".into()))?
        .min;
    Ok(EarthRange {
        min_m: d[imin],
        min_epoch: start.plus_seconds(times[imin]),
        max_m: d[imax],
        max_epoch: start.plus_seconds(times[imax]),
        sep_min_rad: sep_min.value,
        sep_min_epoch: start.plus_seconds(sep_min.t_s),
        sep_below_s: time_below_s(&times, &sep, sep_threshold_rad),
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

    /// 外合（太陽の向こう側）から始まる 1.5 AU の円軌道: 元期の SEP は 0 で、最小もそこ（元期）。
    /// 太陽から見た経度差は、地球（0.9856°/日）と探査機（周期 1.5^1.5 年 = 1.837 年で 0.5365°/日）の差 0.449°/日で開く。
    /// 小さい角では SEP ≈ 経度差 × R/(R + r_E)（R = 1.5 AU、r_E ≈ 1 AU）≈ 0.6 × 経度差なので、5° を下回る時間は
    /// 5° / 0.6 / 0.449 ≈ 18.5 日（元期から片側だけ）。小角の近似なので 10〜30 日の幅で確かめる。
    #[test]
    fn superior_conjunction_start_has_zero_sep_min_and_a_short_time_below_the_threshold() {
        use crate::constants::AU;
        use crate::ephem::{Body, eci_to_ecliptic_j2000, position_eci_m};
        let s = position_eci_m(Body::Sun, epoch());
        let e = eci_to_ecliptic_j2000(-s);
        let lambda = e.y.atan2(e.x) + std::f64::consts::PI;
        let eps = crate::ephem::ecliptic_normal_eci().z.acos();
        let elements = OrbitElements {
            semi_major_axis: Qty::from_raw(1.5 * AU),
            eccentricity: 0.0,
            inclination: Qty::from_raw(eps),
            raan: Qty::from_raw(0.0),
            arg_perigee: Qty::from_raw(0.0),
            mean_anomaly: Qty::from_raw(lambda),
        };
        let mut orbit = OrbitSpec::new(epoch(), elements, Propagator::Kepler);
        orbit.central = CentralBody::Sun;
        let threshold = 5.0_f64.to_radians();
        let r = earth_range_with_sep(
            &Trajectory::from(orbit),
            epoch(),
            100.0 * SECONDS_PER_DAY,
            0.25 * SECONDS_PER_DAY,
            threshold,
        )
        .expect("range");
        assert!(r.sep_min_rad < 1e-4, "{}", r.sep_min_rad);
        assert!(r.sep_min_epoch.seconds_since(epoch()).abs() < 1.0);
        let days = r.sep_below_s / SECONDS_PER_DAY;
        assert!((10.0..30.0).contains(&days), "{days}");
    }
}
