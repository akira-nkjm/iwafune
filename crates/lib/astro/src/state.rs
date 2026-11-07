//! 軌道位置・速度の共有型。
use crate::math::Vec3;
use serde::{Deserialize, Serialize};
/// ECI での位置・速度。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OrbitState {
    /// 位置 [m]
    pub r_eci: Vec3,
    /// 速度 [m/s]
    pub v_eci: Vec3,
}

impl OrbitState {
    /// 軌道面の法線（角運動量の方向）の単位ベクトル。
    pub fn orbit_normal(&self) -> Vec3 {
        self.r_eci.cross(self.v_eci).normalized().unwrap_or(Vec3::Z)
    }
}
