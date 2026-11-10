//! 擾乱トルク（重力傾斜・空力・太陽輻射圧・残留磁気）。**RW アンロード頻度などの姿勢の解析はこれだけを使う。**
//!
//! 式は sat-astro の型（`Vec3`・`Mat3`・`TimelineSample`）と環境モデル（大気・地磁気・太陽光）で書く。
//! トルクはすべて機体座標系 [N·m]、重心まわり。
//!
//! | 種類 | 式 |
//! |---|---|
//! | 重力傾斜 | `τ = 3μ/|r|⁵ · r × (I r)`（`r` は中心天体 → 衛星、機体座標系。μ は中心天体のもの） |
//! | 空力 | `F = −½ ρ (Cd·A) |v| v`（`sat_astro::force::drag_force`）、`τ = r_cp × F`（`v` は `sat_astro::force::relative_wind_eci`、ρ は `atmosphere::density_about_kg_m3`。地球以外は 0） |
//! | 太陽輻射圧 | `F = −P (Cr·A) ŝ`、`P = force::srp_pressure(太陽距離) · ν`、`τ = r_cp × F` |
//! | 残留磁気 | `τ = m × B`（B は `geomag::field_about_eci_t`。地球以外は 0） |

use crate::spec::{AnalysisError, ensure_non_negative, ensure_positive};
use sat_astro::atmosphere::{AtmosphereModel, density_about_kg_m3};
use sat_astro::ephem::CentralBody;
use sat_astro::force::{drag_force, relative_wind_eci, srp_pressure};
use sat_astro::geomag::field_about_eci_t;
use sat_astro::math::{Mat3, Vec3};
use sat_astro::timeline::TimelineSample;
use sat_units::typed::{Area, Qty};
use serde::{Deserialize, Serialize};

/// 擾乱を求めるための機体の値。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DisturbanceSpacecraft {
    /// 重心まわりの慣性テンソル [kg·m²]（機体座標系）
    pub inertia: Mat3,
    /// 空力の面（Cd·A と圧力中心）。省略時は空力トルクを 0 とする
    #[serde(default)]
    pub aero: Option<DisturbanceSurface>,
    /// 太陽輻射圧の面（Cr·A と圧力中心）。省略時は SRP トルクを 0 とする
    #[serde(default)]
    pub srp: Option<DisturbanceSurface>,
    /// 残留磁気モーメント [A·m²]（機体座標系）
    #[serde(default)]
    pub residual_dipole: Vec3,
    /// 大気モデル（省略時は指数大気）
    #[serde(default)]
    pub atmosphere: AtmosphereModel,
    /// 大気密度に掛ける倍率（モデルの不確かさを見込む）。省略時 1
    #[serde(default)]
    pub density_scale: Option<f64>,
}

/// 力を受ける面（キャノンボール近似: 向きによらず一定の係数 × 面積）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DisturbanceSurface {
    /// 圧力中心 − 重心 [m]（機体座標系）
    pub cp_offset: Vec3,
    /// 係数 × 面積（空力は Cd·A、太陽輻射圧は Cr·A）
    pub coefficient_area: Qty<Area>,
}

impl DisturbanceSurface {
    /// 箱の最大の面を受圧面積とするキャノンボール近似。
    /// size は SI の箱の辺 [m]、coefficient は Cd または Cr。
    pub fn from_box(size: Vec3, coefficient: f64, cp_offset: Vec3) -> Self {
        let area = (size.x * size.y).max(size.y * size.z).max(size.x * size.z);
        Self {
            cp_offset,
            coefficient_area: Qty::from_raw(coefficient * area),
        }
    }
}

/// 4 種の擾乱トルク [N·m]（機体座標系）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DisturbanceTorques {
    pub gravity_gradient: Vec3,
    pub aero: Vec3,
    pub srp: Vec3,
    pub magnetic: Vec3,
}

/// 擾乱の種類の機械向けの名前と表示名（[`DisturbanceTorques::by_kind`] の順）。
pub const DISTURBANCE_KINDS: [(&str, &str); 4] = [
    ("gravity_gradient", "重力傾斜トルク"),
    ("aero", "空力トルク"),
    ("srp", "太陽輻射圧トルク"),
    ("magnetic", "残留磁気トルク"),
];

impl DisturbanceTorques {
    /// 合計 [N·m]。
    pub fn total(&self) -> Vec3 {
        self.gravity_gradient + self.aero + self.srp + self.magnetic
    }

    /// [`DISTURBANCE_KINDS`] の順に並べたもの。
    pub fn by_kind(&self) -> [Vec3; 4] {
        [self.gravity_gradient, self.aero, self.srp, self.magnetic]
    }
}

impl DisturbanceSpacecraft {
    /// 値が物理的にありうるかを調べる。
    pub fn validate(&self) -> Result<(), AnalysisError> {
        for i in 0..3 {
            ensure_positive("慣性テンソルの対角成分", self.inertia.m[i][i])?;
        }
        if !self.inertia.m.iter().flatten().all(|v| v.is_finite()) {
            return Err(AnalysisError::InvalidInput(
                "慣性テンソルが有限ではありません".to_owned(),
            ));
        }
        if let Some(s) = self.density_scale {
            ensure_non_negative("大気密度の倍率", s)?;
        }
        for (name, s) in [("空力の Cd·A", &self.aero), ("SRP の Cr·A", &self.srp)] {
            if let Some(s) = s {
                ensure_non_negative(name, s.coefficient_area.value())?;
            }
        }
        Ok(())
    }

    /// 時系列の 1 点での擾乱トルク。
    /// 時系列の 1 点での擾乱トルク。`central` は軌道の中心天体（`TimelineSpec.orbit.central`）。
    /// μ・磁場・大気は中心天体のもの（磁場と大気は地球だけにあり、ほかの中心天体では 0）。
    pub fn torques(&self, central: CentralBody, s: &TimelineSample) -> DisturbanceTorques {
        let r_body = s.to_body(s.r_eci);
        let b_eci = field_about_eci_t(central, s.r_eci, s.epoch);
        let mut out = DisturbanceTorques {
            gravity_gradient: gravity_gradient(&self.inertia, r_body, central.gm()),
            magnetic: magnetic(self.residual_dipole, s.to_body(b_eci)),
            ..Default::default()
        };
        if let Some(surface) = &self.aero {
            let rho = density_about_kg_m3(central, &self.atmosphere, s.r_eci, s.epoch)
                * self.density_scale.unwrap_or(1.0);
            out.aero = aero(surface, rho, s.to_body(relative_wind_eci(s.r_eci, s.v_eci)));
        }
        if let Some(surface) = &self.srp {
            let pressure = srp_pressure(s.sun_distance_m) * s.sunlit.clamp(0.0, 1.0);
            out.srp = srp(surface, pressure, s.sun_dir_body());
        }
        out
    }
}

/// 重力傾斜トルク [N·m]。`inertia` [kg·m²]、`r_body` は中心天体 → 衛星の位置 [m]（どちらも機体座標系）、
/// `mu` は中心天体の重力定数 [m³/s²]。
pub fn gravity_gradient(inertia: &Mat3, r_body: Vec3, mu: f64) -> Vec3 {
    let r = r_body.norm();
    if r <= 0.0 {
        return Vec3::ZERO;
    }
    r_body.cross(inertia.mul_vec(r_body)) * (3.0 * mu / r.powi(5))
}

/// 空力トルク [N·m]。`rho` は大気密度 [kg/m³]、`v_rel_body` は大気に対する機体の速度 [m/s]（機体座標系）。
/// 抗力は `sat_astro::force::drag_force`。
pub fn aero(surface: &DisturbanceSurface, rho: f64, v_rel_body: Vec3) -> Vec3 {
    let force = drag_force(rho, surface.coefficient_area.value(), v_rel_body);
    surface.cp_offset.cross(force)
}

/// 太陽輻射圧トルク [N·m]。`pressure` は日照率込みの輻射圧 [N/m²]、`sun_dir_body` は機体 → 太陽の方向。
pub fn srp(surface: &DisturbanceSurface, pressure: f64, sun_dir_body: Vec3) -> Vec3 {
    let Some(sun) = sun_dir_body.normalized() else {
        return Vec3::ZERO;
    };
    let force = sun * (-pressure * surface.coefficient_area.value());
    surface.cp_offset.cross(force)
}

/// 残留磁気トルク [N·m]。`dipole` [A·m²]、`b_body` は磁束密度 [T]（どちらも機体座標系）。
pub fn magnetic(dipole: Vec3, b_body: Vec3) -> Vec3 {
    dipole.cross(b_body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sat_astro::constants::{C_LIGHT, MU_EARTH};

    #[test]
    fn box_surface_by_hand() {
        // 2 × 3 × 4 m の箱: 最大面 12 m²、Cd = 2 → 24 m²。
        let s = DisturbanceSurface::from_box(Vec3::new(2.0, 3.0, 4.0), 2.0, Vec3::X);
        assert_eq!(s.coefficient_area.value(), 24.0);
        assert_eq!(s.cp_offset, Vec3::X);
    }

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol * b.abs().max(1e-30)
    }

    #[test]
    fn gravity_gradient_45_degrees_by_hand() {
        // 地心方向が y-z 面で 45°: |τ| = 3μ/(2r³)·|Iz − Iy|（x 軸まわり）
        let inertia = Mat3::diagonal(Vec3::new(0.1, 0.2, 0.3));
        let r = 7.0e6;
        let r_body = Vec3::new(0.0, 1.0, 1.0).normalized().unwrap() * r;
        let tau = gravity_gradient(&inertia, r_body, MU_EARTH);
        let expect = 3.0 * MU_EARTH / (2.0 * r.powi(3)) * 0.1;
        assert!(close(tau.x, expect, 1e-12), "{tau:?} vs {expect}");
        assert!(tau.y.abs() < 1e-20 && tau.z.abs() < 1e-20);
        // 主軸が地心方向にそろえば 0
        let aligned = gravity_gradient(&inertia, Vec3::Z * r, MU_EARTH);
        assert!(aligned.norm() < 1e-20);
    }

    #[test]
    fn aero_by_hand() {
        // ρ = 1e-12、Cd·A = 0.1 m²、v = 7.5 km/s（+x）、CP が +y に 1 cm
        // F = −½·1e-12·0.1·7500² = −2.8125e-6 N（x）、τ = r × F = (0, 0.01, 0) × (F, 0, 0) = (0, 0, −0.01 F)
        let s = DisturbanceSurface {
            cp_offset: Vec3::new(0.0, 0.01, 0.0),
            coefficient_area: Qty::from_raw(0.1),
        };
        let tau = aero(&s, 1e-12, Vec3::new(7500.0, 0.0, 0.0));
        assert!(close(tau.z, 2.8125e-8, 1e-12), "{tau:?}");
    }

    #[test]
    fn srp_and_magnetic_by_hand() {
        // P = 1361/c = 4.54e-6 N/m²、Cr·A = 0.1 m²、太陽 +x、CP が +z に 2 cm
        let s = DisturbanceSurface {
            cp_offset: Vec3::new(0.0, 0.0, 0.02),
            coefficient_area: Qty::from_raw(0.1),
        };
        let p = 1361.0 / C_LIGHT;
        let tau = srp(&s, p, Vec3::X);
        // F = (−P·0.1, 0, 0)、τ = (0,0,0.02) × F = (0, −0.02·P·0.1, 0)
        assert!(close(tau.y, -0.02 * p * 0.1, 1e-12));
        // m = 0.01 A·m²（+z）、B = 30 µT（+x）→ τ = m × B = (0, 3e-7, 0)
        let m = magnetic(Vec3::new(0.0, 0.0, 0.01), Vec3::new(3e-5, 0.0, 0.0));
        assert!(close(m.y, 3e-7, 1e-12));
    }
}
