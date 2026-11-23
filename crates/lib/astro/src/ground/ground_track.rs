//! 地上軌跡（衛星直下点の測地緯度・経度と高度の時系列）。

use crate::AstroError;
use crate::constants::R_MOON;
use crate::earth::geodetic_from_eci;
use crate::ephem::CentralBody;
use crate::ephem::orientation::moon_fixed_frame;
use crate::math::Vec3;
use crate::orbit::Trajectory;
use crate::time::Epoch;
use crate::timeline::sample_times;
use serde::{Deserialize, Serialize};

/// 地上軌跡の 1 点。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GroundTrackPoint {
    /// `start` からの経過 [s]
    pub t_s: f64,
    /// 測地緯度 [rad]
    pub lat_rad: f64,
    /// 経度 [rad]（−π〜π、東経が正）
    pub lon_rad: f64,
    /// 楕円体高 [m]
    pub alt_m: f64,
}

/// `start` から `span_s` 秒の地上軌跡を `step_s` 秒刻みで求める（最後の点は `span_s` ちょうど）。
///
/// 中心天体の固定座標で求める。地球は測地座標（WGS84）、月は月固定の ME 系の球面座標（高度は
/// [`crate::constants::R_MOON`] から）。ほかの中心天体は自転のモデルが無いので `InvalidInput`。
pub fn ground_track(
    orbit: &Trajectory,
    start: Epoch,
    span_s: f64,
    step_s: f64,
) -> Result<Vec<GroundTrackPoint>, AstroError> {
    let times = sample_times(span_s, step_s)?;
    let offset = start.seconds_since(orbit.epoch());
    let abs: Vec<f64> = times.iter().map(|t| t + offset).collect();
    let fixed = match orbit.central() {
        CentralBody::Earth => geodetic_from_eci,
        CentralBody::Moon => selenographic_from_eci,
        other => {
            return Err(AstroError::InvalidInput(format!(
                "Ground track is not available for {other:?} (no rotation model)"
            )));
        }
    };
    let states = orbit.states_at(&abs)?;
    Ok(times
        .iter()
        .zip(states.iter())
        .map(|(&t_s, s)| {
            let (lat_rad, lon_rad, alt_m) = fixed(s.r_eci, start.plus_seconds(t_s));
            GroundTrackPoint {
                t_s,
                lat_rad,
                lon_rad,
                alt_m,
            }
        })
        .collect())
}

/// 月中心の ECI の位置 → 月固定（ME 系）の (緯度, 経度, 高度)。球として扱う。
fn selenographic_from_eci(r_eci_m: Vec3, epoch: Epoch) -> (f64, f64, f64) {
    let f = moon_fixed_frame(epoch).mul_vec(r_eci_m);
    let r = f.norm();
    let lat = if r > 0.0 {
        (f.z / r).clamp(-1.0, 1.0).asin()
    } else {
        0.0
    };
    (lat, f.y.atan2(f.x), r - R_MOON)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{MU_EARTH, OMEGA_EARTH, R_EARTH};
    use crate::orbit::OrbitSpec;
    use sat_units::typed::Qty;

    fn epoch() -> Epoch {
        Epoch::parse("2027-04-01T00:00:00Z").expect("epoch")
    }

    #[test]
    fn max_latitude_equals_inclination_and_altitude_is_constant() {
        let orbit = OrbitSpec::circular(
            epoch(),
            Qty::from_raw(550e3),
            Qty::from_raw(51.6_f64.to_radians()),
            Qty::from_raw(0.0),
        );
        let track = ground_track(
            &Trajectory::from(orbit.clone()),
            epoch(),
            orbit.period_s(),
            10.0,
        )
        .expect("track");
        let max_lat = track.iter().fold(0.0_f64, |m, p| m.max(p.lat_rad.abs()));
        // 測地緯度は地心緯度より少し大きい（51.6° で +0.2° 程度）
        assert!(
            (max_lat.to_degrees() - 51.8).abs() < 0.2,
            "{}",
            max_lat.to_degrees()
        );
        for p in &track {
            // 楕円体高は緯度で ±11 km 程度変わる（地球の扁平）
            assert!((p.alt_m - 550e3).abs() < 25e3, "{}", p.alt_m);
        }
    }

    /// 1 周あたりの経度の西進は (ω_E − dΩ/dt)·T ≈ 24°（550 km）。
    #[test]
    fn longitude_shift_per_orbit() {
        let mut orbit = OrbitSpec::circular(
            epoch(),
            Qty::from_raw(550e3),
            Qty::from_raw(0.0),
            Qty::from_raw(0.0),
        );
        orbit.propagator = crate::orbit::Propagator::Kepler;
        let period = orbit.period_s();
        let track = ground_track(
            &Trajectory::from(orbit.clone()),
            epoch(),
            period,
            period / 4.0,
        )
        .expect("track");
        let d = (track[4].lon_rad - track[0].lon_rad).to_degrees();
        let shift = (d + 540.0).rem_euclid(360.0) - 180.0;
        let expected = -(OMEGA_EARTH * period).to_degrees();
        assert!((shift - expected).abs() < 0.05, "{shift} vs {expected}");
        let n = (MU_EARTH / (R_EARTH + 550e3).powi(3)).sqrt();
        assert!((n - core::f64::consts::TAU / period).abs() < 1e-15);
    }
}
