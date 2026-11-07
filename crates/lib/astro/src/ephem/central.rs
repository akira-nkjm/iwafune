//! 軌道の中心天体（地球・月・太陽・惑星）。μ・半径・J2・位置を 1 か所で引く。
//!
//! 軌道の位置・速度は中心天体からの相対ベクトルで、**向きは J2000 平均赤道（ECI）のまま**（原点だけ動かす）。

use super::{Body, position_eci_m};
pub use crate::body::CentralBody;
#[cfg(test)]
use crate::constants::*;
use crate::math::Vec3;
use crate::time::Epoch;
impl CentralBody {
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
    fn moon_velocity_is_about_one_km_per_second() {
        let v = CentralBody::Moon
            .velocity_eci_m_s(Epoch { j2000_s: 8.6e8 })
            .norm();
        assert!(v > 960.0 && v < 1090.0, "{v}");
        let ratio = MU_EARTH / CentralBody::Moon.gm();
        assert!((ratio - 81.3).abs() < 0.1);
    }
}
