//! スラスタ 1 本の値と、重心まわりのトルク。**スラスタのトルク `τ = (r − r_cm) × F d̂` はここだけで求める**
//! （RCS でのアンロード・推力誤差の擾乱・アンロードの推薬が共有する）。
//!
//! 推力の向き `d̂` は機体が押される向き（機体座標系）。推力の線が重心を通らないぶん（取付位置・向き・重心のずれ）が
//! トルクになる。

use crate::spec::{AnalysisError, ensure_positive};
use sat_astro::math::Vec3;
use sat_units::typed::{Force, Qty, Time};
use serde::{Deserialize, Serialize};

/// スラスタ 1 本。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Thruster {
    pub id: String,
    pub thrust: Qty<Force>,
    pub isp: Qty<Time>,
    /// 取付位置 [m]（機体座標系）
    pub position: Vec3,
    /// 推力の向き（機体座標系。機体が押される向き。長さは問わない）
    pub direction: Vec3,
}

/// 値を調べたスラスタ 1 本の、重心まわりのトルクと推力。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThrusterTorque {
    /// 重心まわりのトルク [N·m]（機体座標系）
    pub torque: Vec3,
    /// 推力のベクトル [N]（機体座標系）
    pub force: Vec3,
    /// 推力の大きさ [N]
    pub thrust: f64,
    /// 比推力 [s]
    pub isp: f64,
}

impl Thruster {
    /// 値を調べ、重心 `center_of_mass` [m] まわりのトルクと推力を返す。推力・比推力が正でないか、向きがゼロなら `InvalidInput`。
    pub fn torque_about(&self, center_of_mass: Vec3) -> Result<ThrusterTorque, AnalysisError> {
        let (f, isp) = (self.thrust.value(), self.isp.value());
        ensure_positive(&format!("スラスタ {} の推力", self.id), f)?;
        ensure_positive(&format!("スラスタ {} の比推力", self.id), isp)?;
        let d = self
            .direction
            .normalized()
            .filter(|_| self.position.norm().is_finite() && center_of_mass.norm().is_finite())
            .ok_or_else(|| {
                AnalysisError::InvalidInput(format!("スラスタ {} の位置・向きが不正です", self.id))
            })?;
        let force = d * f;
        Ok(ThrusterTorque {
            torque: (self.position - center_of_mass).cross(force),
            force,
            thrust: f,
            isp,
        })
    }
}

/// すべてのスラスタの重心まわりのトルクと推力（入力の順）。
pub fn thruster_torques(
    thrusters: &[Thruster],
    center_of_mass: Vec3,
) -> Result<Vec<ThrusterTorque>, AnalysisError> {
    thrusters
        .iter()
        .map(|t| t.torque_about(center_of_mass))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn torque_is_lever_arm_cross_force_by_hand() {
        // 推力 1 N（+z）、取付 (0.01, 0, −0.2) m、重心 (0, 0.002, 0) m
        // 腕 = (0.01, −0.002, −0.2)、τ = 腕 × (0, 0, 1) = (−0.002·1 − 0, 0 − 0.01·1, 0) = (−0.002, −0.01, 0)
        let t = Thruster {
            id: "T".to_owned(),
            thrust: Qty::from_raw(1.0),
            isp: Qty::from_raw(220.0),
            position: Vec3::new(0.01, 0.0, -0.2),
            direction: Vec3::new(0.0, 0.0, 2.0),
        };
        let r = t.torque_about(Vec3::new(0.0, 0.002, 0.0)).unwrap();
        assert!((r.torque - Vec3::new(-0.002, -0.01, 0.0)).norm() < 1e-15);
        assert!((r.force - Vec3::Z).norm() < 1e-15);
        assert_eq!((r.thrust, r.isp), (1.0, 220.0));
    }

    #[test]
    fn invalid_values_are_rejected() {
        let t = Thruster {
            id: "T".to_owned(),
            thrust: Qty::from_raw(1.0),
            isp: Qty::from_raw(220.0),
            position: Vec3::ZERO,
            direction: Vec3::ZERO,
        };
        assert!(t.torque_about(Vec3::ZERO).is_err());
        let t = Thruster {
            direction: Vec3::Z,
            thrust: Qty::from_raw(0.0),
            ..t
        };
        assert!(t.torque_about(Vec3::ZERO).is_err());
    }
}
