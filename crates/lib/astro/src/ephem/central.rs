//! 軌道の中心天体（地球・月・太陽・惑星）。μ・半径・J2・位置を 1 か所で引く。
//!
//! 軌道の位置・速度は中心天体からの相対ベクトルで、**向きは J2000 平均赤道（ECI）のまま**（原点だけ動かす）。

use super::{Body, planets, position_eci_m};
pub use crate::body::CentralBody;
use crate::constants::A_MOON;
#[cfg(test)]
use crate::constants::*;
use crate::math::Vec3;
use crate::time::Epoch;
impl CentralBody {
    /// 親の天体（[`CentralBody::parent`]）を回る軌道の長半径 [m]。月は [`A_MOON`]、地球と惑星は
    /// Standish の表の J2000 の長半径。太陽は `None`。パッチドコニックの遷移は円軌道とみなしてこれを使う。
    pub fn orbit_radius_m(self) -> Option<f64> {
        match self {
            CentralBody::Sun => None,
            CentralBody::Moon => Some(A_MOON),
            other => planets::semi_major_axis_m(other.as_body()),
        }
    }

    /// 地心から中心天体への位置（ECI）[m]。地球は 0。
    pub fn position_eci_m(self, epoch: Epoch) -> Vec3 {
        self.as_body()
            .map_or(Vec3::ZERO, |b| position_eci_m(b, epoch))
    }

    /// 地心に対する中心天体の速度（ECI）[m/s]。暦の中心差分（±60 s）で求める。
    pub fn velocity_eci_m_s(self, epoch: Epoch) -> Vec3 {
        if self == CentralBody::Earth {
            return Vec3::ZERO;
        }
        let h = 60.0;
        (self.position_eci_m(epoch.plus_seconds(h)) - self.position_eci_m(epoch.plus_seconds(-h)))
            / (2.0 * h)
    }

    /// 中心天体から見た天体 `body` の位置（ECI の向き）[m]。
    pub fn relative_position_m(self, body: Body, epoch: Epoch) -> Vec3 {
        position_eci_m(body, epoch) - self.position_eci_m(epoch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earth_uses_existing_constants() {
        let e = CentralBody::Earth;
        assert_eq!(e.gm(), MU_EARTH);
        assert_eq!(e.radius_m(), R_EARTH);
        assert_eq!(e.j2(), J2_EARTH);
        assert_eq!(e.position_eci_m(Epoch { j2000_s: 0.0 }), Vec3::ZERO);
        assert_eq!(CentralBody::default(), CentralBody::Earth);
    }

    #[test]
    fn orbit_radii_and_parents() {
        let au = |b: CentralBody| b.orbit_radius_m().expect("半径") / AU;
        assert!((au(CentralBody::Earth) - 1.0).abs() < 1e-5);
        assert!((au(CentralBody::Mars) - 1.5237).abs() < 1e-4);
        assert_eq!(CentralBody::Moon.orbit_radius_m(), Some(A_MOON));
        assert_eq!(CentralBody::Sun.orbit_radius_m(), None);
        assert_eq!(CentralBody::Moon.parent(), Some(CentralBody::Earth));
        assert_eq!(CentralBody::Mars.parent(), Some(CentralBody::Sun));
        assert_eq!(CentralBody::Moon.nominal_sun_distance_m(), Some(AU));
        assert_eq!(CentralBody::Mars.nominal_sun_distance_m(), None);
    }

    #[test]
    fn moon_velocity_is_about_one_km_per_second() {
        let v = CentralBody::Moon
            .velocity_eci_m_s(Epoch { j2000_s: 8.6e8 })
            .norm();
        assert!(v > 960.0 && v < 1090.0, "{v}");
        let ratio = MU_EARTH / CentralBody::Moon.gm();
        assert!((ratio - 81.3).abs() < 0.1);
    }
}
