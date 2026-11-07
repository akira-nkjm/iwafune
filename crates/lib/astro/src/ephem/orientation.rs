//! 天体の自転軸と本初子午線（IAU WGCCRE 2015）。月は秤動の周期項まで入れる。
//! 惑星と太陽は J2000 の自転軸の向きだけを持つ。
//!
//! 月の座標系は Mean Earth / polar axis（ME）系。経度は東が正。

use super::CentralBody;
use crate::constants::SECONDS_PER_DAY;
use crate::math::{Mat3, Vec3};
use crate::time::Epoch;

/// 月の回転要素の引数 E1〜E13 [deg]（定数項, 日あたりの変化）。
const E_ARGUMENTS_DEG: [[f64; 2]; 13] = [
    [125.045, -0.052_992_1],
    [250.089, -0.105_984_2],
    [260.008, 13.012_000_9],
    [176.625, 13.340_715_4],
    [357.529, 0.985_600_3],
    [311.589, 26.405_708_4],
    [134.963, 13.064_993_0],
    [276.617, 0.328_714_6],
    [34.226, 1.748_487_7],
    [15.134, -0.158_976_3],
    [119.743, 0.003_609_6],
    [239.961, 0.164_357_3],
    [25.053, 12.959_008_8],
];
const ALPHA_TERMS: [(usize, f64); 7] = [
    (1, -3.8787),
    (2, -0.1204),
    (3, 0.0700),
    (4, -0.0172),
    (6, 0.0072),
    (10, -0.0052),
    (13, 0.0043),
];
const DELTA_TERMS: [(usize, f64); 8] = [
    (1, 1.5419),
    (2, 0.0239),
    (3, -0.0278),
    (4, 0.0068),
    (6, -0.0029),
    (7, 0.0009),
    (10, 0.0008),
    (13, -0.0009),
];
const W_TERMS: [(usize, f64); 13] = [
    (1, 3.5610),
    (2, 0.1208),
    (3, -0.0642),
    (4, 0.0158),
    (5, 0.0252),
    (6, -0.0066),
    (7, -0.0047),
    (8, -0.0046),
    (9, 0.0028),
    (10, 0.0052),
    (11, 0.0040),
    (12, 0.0019),
    (13, -0.0044),
];

/// 月の自転軸の赤経・赤緯と本初子午線の位置角 W [rad]。
pub fn lunar_orientation(epoch: Epoch) -> (f64, f64, f64) {
    let d = epoch.j2000_s / SECONDS_PER_DAY;
    let t = epoch.julian_centuries();
    let e: [f64; 13] =
        core::array::from_fn(|i| (E_ARGUMENTS_DEG[i][0] + E_ARGUMENTS_DEG[i][1] * d).to_radians());
    let sum_sin =
        |terms: &[(usize, f64)]| terms.iter().map(|(n, c)| c * e[n - 1].sin()).sum::<f64>();
    let sum_cos =
        |terms: &[(usize, f64)]| terms.iter().map(|(n, c)| c * e[n - 1].cos()).sum::<f64>();
    let alpha = 269.9949 + 0.0031 * t + sum_sin(&ALPHA_TERMS);
    let delta = 66.5392 + 0.0130 * t + sum_cos(&DELTA_TERMS);
    let w = 38.3213 + 13.176_358_15 * d - 1.4e-12 * d * d + sum_sin(&W_TERMS);
    (alpha.to_radians(), delta.to_radians(), w.to_radians())
}

/// 惑星・太陽の J2000 の自転軸 (α₀, δ₀) [deg]（WGCCRE 2015 の定数項）。
fn pole_ra_dec_deg(body: CentralBody) -> Option<(f64, f64)> {
    match body {
        CentralBody::Earth | CentralBody::Moon => None,
        CentralBody::Sun => Some((286.13, 63.87)),
        CentralBody::Mercury => Some((281.0103, 61.4155)),
        CentralBody::Venus => Some((272.76, 67.16)),
        CentralBody::Mars => Some((317.681, 52.887)),
        CentralBody::Jupiter => Some((268.056_595, 64.495_303)),
        CentralBody::Saturn => Some((40.589, 83.537)),
        CentralBody::Uranus => Some((257.311, -15.175)),
        CentralBody::Neptune => Some((299.36, 43.46)),
    }
}

/// 中心天体の自転軸（北極）の向き（ECI の単位ベクトル）。地球は ECI の z 軸（歳差・章動を無視）。
pub fn pole_eci(body: CentralBody, epoch: Epoch) -> Vec3 {
    match body {
        CentralBody::Earth => Vec3::Z,
        CentralBody::Moon => {
            let (ra, dec, _) = lunar_orientation(epoch);
            Vec3::from_spherical(ra, dec)
        }
        other => pole_ra_dec_deg(other)
            .map(|(ra, dec)| Vec3::from_spherical(ra.to_radians(), dec.to_radians()))
            .unwrap_or(Vec3::Z),
    }
}

/// ECI → 天体の赤道座標系（z = 自転軸、x = ICRF 赤道上の昇交点方向）の回転行列（行 = 新しい軸）。
pub fn equatorial_frame(body: CentralBody, epoch: Epoch) -> Mat3 {
    let z = pole_eci(body, epoch);
    let x = Vec3::Z.cross(z).normalized().unwrap_or(Vec3::X);
    Mat3::from_rows(x, z.cross(x), z)
}

/// ECI → 月固定座標系（ME 系）の回転行列（行 = 月固定の軸）。
pub fn moon_fixed_frame(epoch: Epoch) -> Mat3 {
    let (ra, dec, w) = lunar_orientation(epoch);
    let pole = Vec3::from_spherical(ra, dec);
    let node = Vec3::new(-ra.sin(), ra.cos(), 0.0);
    let (sw, cw) = w.sin_cos();
    let x = node * cw + pole.cross(node) * sw;
    Mat3::from_rows(x, pole.cross(x), pole)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ephem::{Body, position_eci_m};

    /// 潮汐固定: 月から見た地球（副地球点）は秤動の範囲（経度 ±8°・緯度 ±7°）に収まる。
    #[test]
    fn sub_earth_point_stays_within_libration() {
        for day in (0..400).step_by(7) {
            let e = Epoch {
                j2000_s: 9_000.0 * SECONDS_PER_DAY + f64::from(day) * SECONDS_PER_DAY,
            };
            let to_earth = -position_eci_m(Body::Moon, e);
            let f = moon_fixed_frame(e).mul_vec(to_earth);
            let lon = f.y.atan2(f.x).to_degrees();
            let lat = (f.z / f.norm()).asin().to_degrees();
            assert!(
                lon.abs() < 9.0 && lat.abs() < 8.0,
                "day {day}: {lon}, {lat}"
            );
        }
    }

    #[test]
    fn frames_are_rotations() {
        let e = Epoch { j2000_s: 8.6e8 };
        for m in [moon_fixed_frame(e), equatorial_frame(CentralBody::Mars, e)] {
            assert!((m.determinant() - 1.0).abs() < 1e-12);
        }
        assert_eq!(pole_eci(CentralBody::Earth, e), Vec3::Z);
        // 月の自転軸は黄道の極から約 1.5°（ECI の z 軸から約 24°）
        let tilt = pole_eci(CentralBody::Moon, e)
            .angle_to(Vec3::Z)
            .to_degrees();
        assert!((tilt - 23.5).abs() < 3.0, "{tilt}");
    }
}
