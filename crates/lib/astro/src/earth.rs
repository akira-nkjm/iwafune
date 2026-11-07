//! 地球の座標変換（GMST 回転のみ。章動・極運動は無視）と測地座標（WGS84）、歳差（IAU 1976）。
//! **座標変換はここだけ。**

use crate::constants::{ARCSEC_TO_RAD, FLATTENING_EARTH, R_EARTH};
use crate::math::{Mat3, Vec3};
use crate::time::Epoch;

/// 第一離心率の 2 乗 e²（WGS84）。
const E2: f64 = FLATTENING_EARTH * (2.0 - FLATTENING_EARTH);

/// z 軸まわりに座標系を `angle` だけ回したときのベクトル成分（= ベクトルを −angle 回す）。
fn frame_rotate_z(v: Vec3, angle: f64) -> Vec3 {
    Mat3::rotation_z(-angle).mul_vec(v)
}

/// ECI → ECEF。
pub fn eci_to_ecef(v_eci: Vec3, epoch: Epoch) -> Vec3 {
    frame_rotate_z(v_eci, epoch.gmst_rad())
}

/// ECEF → ECI。
pub fn ecef_to_eci(v_ecef: Vec3, epoch: Epoch) -> Vec3 {
    frame_rotate_z(v_ecef, -epoch.gmst_rad())
}

/// 測地緯度・経度 [rad]・楕円体高 [m] → ECEF [m]。
pub fn geodetic_to_ecef(lat_rad: f64, lon_rad: f64, height_m: f64) -> Vec3 {
    let (sl, cl) = lat_rad.sin_cos();
    let (so, co) = lon_rad.sin_cos();
    let n = R_EARTH / (1.0 - E2 * sl * sl).sqrt();
    Vec3::new(
        (n + height_m) * cl * co,
        (n + height_m) * cl * so,
        (n * (1.0 - E2) + height_m) * sl,
    )
}

/// ECEF [m] → (測地緯度 [rad], 経度 [rad], 楕円体高 [m])。
///
/// 緯度は `φ = atan2(z + e² N sin φ, p)` の反復（極でも赤道でも収束する）。
pub fn ecef_to_geodetic(r_ecef_m: Vec3) -> (f64, f64, f64) {
    let p = r_ecef_m.x.hypot(r_ecef_m.y);
    let lon = r_ecef_m.y.atan2(r_ecef_m.x);
    let z = r_ecef_m.z;
    let mut lat = z.atan2(p * (1.0 - E2));
    for _ in 0..10 {
        let s = lat.sin();
        let n = R_EARTH / (1.0 - E2 * s * s).sqrt();
        let next = (z + E2 * n * s).atan2(p);
        let done = (next - lat).abs() < 1e-14;
        lat = next;
        if done {
            break;
        }
    }
    let (s, c) = lat.sin_cos();
    let h = p * c + z * s - R_EARTH * (1.0 - E2 * s * s).sqrt();
    (lat, lon, h)
}

/// 地表からの高度 [m]（楕円体高）。
pub fn altitude_m(r_eci_m: Vec3, epoch: Epoch) -> f64 {
    ecef_to_geodetic(eci_to_ecef(r_eci_m, epoch)).2
}

/// ECI の位置 [m] → (測地緯度 [rad], 経度 [rad], 楕円体高 [m])。地上軌跡などに使う。
pub fn geodetic_from_eci(r_eci_m: Vec3, epoch: Epoch) -> (f64, f64, f64) {
    ecef_to_geodetic(eci_to_ecef(r_eci_m, epoch))
}

/// その時刻の平均黄道傾斜角 ε_A [rad]（IAU 1976: 23°26′21.448″ − 46.8150″T − …）。
pub fn mean_obliquity_rad(epoch: Epoch) -> f64 {
    let t = epoch.julian_centuries();
    let j2000 = 23.0 * 3600.0 + 26.0 * 60.0 + 21.448;
    (j2000 + t * (-46.8150 + t * (-0.00059 + t * 0.001813))) * ARCSEC_TO_RAD
}

/// 平均赤道 of date のベクトルを J2000 平均赤道へ回す（IAU 1976 歳差、ζ_A・z_A・θ_A）。
pub fn of_date_to_j2000(v: Vec3, epoch: Epoch) -> Vec3 {
    let t = epoch.julian_centuries();
    let zeta = t * (2306.2181 + t * (0.30188 + t * 0.017998)) * ARCSEC_TO_RAD;
    let z = t * (2306.2181 + t * (1.09468 + t * 0.018203)) * ARCSEC_TO_RAD;
    let theta = t * (2004.3109 + t * (-0.42665 + t * -0.041833)) * ARCSEC_TO_RAD;
    // ベクトルの回転 R_z(−ζ) → R_y(θ) → R_z(−z)。
    let r = Mat3::rotation_z(-z)
        .mul(&Mat3::rotation_y(theta))
        .mul(&Mat3::rotation_z(-zeta));
    r.mul_vec(v)
}

/// 黄道直交座標 → 赤道直交座標（x 軸まわりに +ε 回す）。
pub fn ecliptic_to_equatorial(v_ecl: Vec3, obliquity_rad: f64) -> Vec3 {
    Mat3::rotation_x(obliquity_rad).mul_vec(v_ecl)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eci_ecef_round_trip_and_gmst_rotation() {
        let e = Epoch::parse("2027-04-01T03:00:00Z").expect("元期");
        let v = Vec3::new(7.0e6, -1.2e6, 3.3e5);
        let back = ecef_to_eci(eci_to_ecef(v, e), e);
        assert!((back - v).norm() < 1e-6);
        // ECI の x 軸は ECEF では経度 −GMST の方向にある。
        let x = eci_to_ecef(Vec3::X, e);
        let lon = x.y.atan2(x.x);
        let expected = (-e.gmst_rad() + core::f64::consts::PI).rem_euclid(core::f64::consts::TAU)
            - core::f64::consts::PI;
        assert!((lon - expected).abs() < 1e-12);
    }

    #[test]
    fn geodetic_round_trip() {
        for &(lat, lon, h) in &[
            (35.0_f64, 139.0_f64, 40.0),
            (-89.9, -45.0, 1000.0),
            (90.0, 0.0, 0.0),
            (0.0, 180.0, 550e3),
            (-33.9, 18.4, 12.0),
        ] {
            let r = geodetic_to_ecef(lat.to_radians(), lon.to_radians(), h);
            let (la, lo, hh) = ecef_to_geodetic(r);
            assert!((la.to_degrees() - lat).abs() < 1e-9, "{lat}");
            if lat.abs() < 90.0 {
                let dlon = (lo.to_degrees() - lon + 540.0).rem_euclid(360.0) - 180.0;
                assert!(dlon.abs() < 1e-9, "{lon}");
            }
            assert!((hh - h).abs() < 1e-6, "{h} vs {hh}");
        }
    }

    #[test]
    fn polar_radius_is_wgs84() {
        let r = geodetic_to_ecef(core::f64::consts::FRAC_PI_2, 0.0, 0.0);
        assert!((r.z - 6_356_752.314_245).abs() < 1e-3, "{}", r.z);
    }

    #[test]
    fn precession_rate_is_about_50_arcsec_per_year() {
        let e = Epoch {
            j2000_s: 365.25 * 86_400.0,
        };
        let moved = of_date_to_j2000(Vec3::X, e);
        let ang = Vec3::X.angle_to(moved) / ARCSEC_TO_RAD;
        assert!((ang - 50.29).abs() < 1.0, "{ang}");
        assert!(
            (mean_obliquity_rad(Epoch { j2000_s: 0.0 }).to_degrees() - 23.439_291).abs() < 1e-5
        );
    }
}
