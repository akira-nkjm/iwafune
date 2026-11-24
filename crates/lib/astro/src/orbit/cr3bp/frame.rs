//! 正規化した回転系と慣性系の変換（脈動座標系）。天体の暦は呼ばない純粋なベクトルの関数。
//!
//! **規約（脈動座標系、独立変数は真近点離角）**: 副天体の主天体まわりの位置 `r_s`・速度 `v_s`（慣性系、主天体中心）から、
//! 瞬間の距離 `d = |r_s|`、角速度 `ω = |r_s × v_s| / d²`、基底 `e_x = r_s/d`、`e_z = (r_s × v_s)/|r_s × v_s|`、`e_y = e_z × e_x` を決める。
//! 重心の位置は主天体から `μ·r_s`（速度 `μ·v_s`）。重心からの位置 `ρ` の正規化した位置は `X = Rᵀρ / d`、
//! 速度は `V = (Rᵀ(ρ̇ − ω×ρ)/d − (ḋ/d)·X) / ω`（`ḋ = r_s·v_s/d`。すなわち `dX/dτ`、`dτ = ω dt`）。
//! 副天体の軌道が円なら `ḋ = 0`、`ω = n` で、通常の CR3BP の正規化に一致する。
//! 慣性系の状態は **主天体中心**（`OrbitState` の `r_eci`・`v_eci`、単位は m と m/s）。

use super::Cr3bp;
use super::dynamics::State6;
use crate::math::{Mat3, Vec3};
use crate::state::OrbitState;

/// 瞬間の脈動座標系。
struct Pulsating {
    d: f64,
    d_dot: f64,
    omega: f64,
    /// 列が `e_x, e_y, e_z`（回転系 → 慣性系）
    rot: Mat3,
    omega_vec: Vec3,
    bary: OrbitState,
}

impl Pulsating {
    fn new(mu: f64, sec: &OrbitState) -> Option<Self> {
        let (r, v) = (sec.r_eci, sec.v_eci);
        let d = r.norm();
        let h = r.cross(v);
        let hn = h.norm();
        if !(d > 0.0 && hn > 0.0) {
            return None;
        }
        let (ex, ez) = (r / d, h / hn);
        let ey = ez.cross(ex);
        let omega = hn / (d * d);
        Some(Self {
            d,
            d_dot: r.dot(v) / d,
            omega,
            rot: Mat3::from_columns(ex, ey, ez),
            omega_vec: ez * omega,
            bary: OrbitState {
                r_eci: r * mu,
                v_eci: v * mu,
            },
        })
    }
}

impl Cr3bp {
    /// 回転系の正規化した状態を、主天体中心の慣性系の状態 [m, m/s] にする。`secondary` は副天体の主天体まわりの状態。
    /// 副天体の位置が 0、または角運動量が 0（直線運動）なら `None`。
    pub fn rotating_to_inertial(&self, secondary: &OrbitState, x: &State6) -> Option<OrbitState> {
        let f = Pulsating::new(self.mu(), secondary)?;
        let rho = f.rot.mul_vec(Vec3::new(x[0], x[1], x[2])) * f.d;
        let v_rot = f.rot.mul_vec(Vec3::new(x[3], x[4], x[5]));
        let rho_dot = f.omega_vec.cross(rho) + v_rot * (f.d * f.omega) + rho * (f.d_dot / f.d);
        Some(OrbitState {
            r_eci: f.bary.r_eci + rho,
            v_eci: f.bary.v_eci + rho_dot,
        })
    }

    /// 主天体中心の慣性系の状態を、回転系の正規化した状態にする（[`Self::rotating_to_inertial`] の逆）。
    pub fn inertial_to_rotating(&self, secondary: &OrbitState, s: &OrbitState) -> Option<State6> {
        let f = Pulsating::new(self.mu(), secondary)?;
        let rt = f.rot.transpose();
        let rho = s.r_eci - f.bary.r_eci;
        let rho_dot = s.v_eci - f.bary.v_eci;
        let pos = rt.mul_vec(rho) / f.d;
        let vel =
            (rt.mul_vec(rho_dot - f.omega_vec.cross(rho)) / f.d - pos * (f.d_dot / f.d)) / f.omega;
        Some([pos.x, pos.y, pos.z, vel.x, vel.y, vel.z])
    }
}
