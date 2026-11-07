//! 地磁気（傾斜双極子。双極子軸は地理北極から約 9.4° 傾く）。**地磁気モデルはここだけ。**
//!
//! `B = B₀ (R_E/r)³ [3 (m̂·r̂) r̂ − m̂]`。m̂ は双極子モーメントの向き（南半球側を指す。
//! 北磁極の向きの反対）、B₀ は赤道表面の強さ [`crate::constants::B0_EARTH`]。
//! 双極子軸は地球に固定（ECEF）で、時刻に応じて ECI に回す。

use crate::constants::{B0_EARTH, GEOMAG_NORTH_POLE_LAT_DEG, GEOMAG_NORTH_POLE_LON_DEG, R_EARTH};
use crate::earth::ecef_to_eci;
use crate::ephem::CentralBody;
use crate::math::Vec3;
use crate::time::Epoch;

/// 双極子モーメントの向き（ECEF の単位ベクトル。北磁極の反対向き）。
pub fn dipole_axis_ecef() -> Vec3 {
    let (lat, lon) = (
        GEOMAG_NORTH_POLE_LAT_DEG.to_radians(),
        GEOMAG_NORTH_POLE_LON_DEG.to_radians(),
    );
    -Vec3::from_spherical(lon, lat)
}

/// 位置 `r_eci_m` [m] での磁束密度（ECI）[T]。地心では 0。
pub fn dipole_field_eci_t(r_eci_m: Vec3, epoch: Epoch) -> Vec3 {
    let r = r_eci_m.norm();
    let Some(r_hat) = r_eci_m.normalized() else {
        return Vec3::ZERO;
    };
    let m_hat = ecef_to_eci(dipole_axis_ecef(), epoch);
    let k = B0_EARTH * (R_EARTH / r).powi(3);
    (r_hat * (3.0 * m_hat.dot(r_hat)) - m_hat) * k
}

/// 中心天体 `central` のまわりの磁束密度（ECI）[T]。地球だけが磁場を持つモデルなので、ほかの中心天体では 0。
/// `r_rel_m` は中心天体からの位置 [m]。
pub fn field_about_eci_t(central: CentralBody, r_rel_m: Vec3, epoch: Epoch) -> Vec3 {
    match central {
        CentralBody::Earth => dipole_field_eci_t(r_rel_m, epoch),
        _ => Vec3::ZERO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::earth::{ecef_to_eci, geodetic_to_ecef};

    fn epoch() -> Epoch {
        Epoch::parse("2027-04-01T00:00:00Z").expect("元期")
    }

    #[test]
    fn magnitude_on_magnetic_equator_and_pole() {
        let m = ecef_to_eci(dipole_axis_ecef(), epoch());
        let eq = m.any_orthogonal() * R_EARTH;
        let b_eq = dipole_field_eci_t(eq, epoch());
        assert!((b_eq.norm() - B0_EARTH).abs() < 1e-12);
        // 赤道では北（−m̂）向き
        assert!(b_eq.dot(-m) > 0.0);
        let pole = -m * R_EARTH;
        let b_pole = dipole_field_eci_t(pole, epoch());
        assert!((b_pole.norm() - 2.0 * B0_EARTH).abs() < 1e-12);
        // 北磁極では下向き（地心向き）
        assert!(b_pole.dot(pole) < 0.0);
    }

    #[test]
    fn leo_field_strength_range() {
        // 550 km の円軌道では 18〜50 μT 程度
        let r = R_EARTH + 550e3;
        for i in 0..36 {
            let lat = f64::from(i) * 10.0 - 175.0;
            let p = ecef_to_eci(geodetic_to_ecef(0.0, lat.to_radians(), 550e3), epoch());
            let b = dipole_field_eci_t(p.normalized().expect("非ゼロ") * r, epoch()).norm();
            assert!(b > 18e-6 && b < 52e-6, "{b}");
        }
        assert_eq!(dipole_field_eci_t(Vec3::ZERO, epoch()), Vec3::ZERO);
    }

    #[test]
    fn dipole_axis_is_tilted_about_9_degrees() {
        let tilt = dipole_axis_ecef().angle_to(-Vec3::Z).to_degrees();
        assert!((tilt - 9.35).abs() < 0.05, "{tilt}");
    }
}
