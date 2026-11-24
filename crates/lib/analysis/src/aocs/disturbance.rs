//! 擾乱トルク（重力傾斜・空力・太陽輻射圧・残留磁気・推力誤差）。**RW アンロード頻度などの姿勢の解析はこれだけを使う。**
//!
//! 式は sat-astro の型（`Vec3`・`Mat3`・`TimelineSample`）と環境モデル（大気・地磁気・太陽光）で書く。
//! トルクはすべて機体座標系 [N·m]、重心まわり。
//!
//! | 種類 | 式 |
//! |---|---|
//! | 重力傾斜 | `τ = 3μ/|r|⁵ · r × (I r)`（`r` は中心天体 → 衛星、機体座標系。μ は中心天体のもの） |
//! | 空力 | `F = −½ ρ (Cd·A) |v| v`（`sat_astro::force::drag_force`）、`τ = r_cp × F`（`v` は `sat_astro::force::earth_relative_wind_eci`、ρ は `atmosphere::density_about_kg_m3`。地球以外は 0。[`DisturbanceSpacecraft::environment_check`]） |
//! | 太陽輻射圧 | `F = −P (Cr·A) ŝ`、`P = force::srp_pressure(太陽距離) · ν`、`τ = r_cp × F` |
//!
//! 空力・太陽輻射圧の `A`（流れ・太陽の向きに沿った投影面積）と `r_cp`（重心から投影の図心への腕）は、書かれていればその値、
//! 無ければ部品の箱から時系列の点ごとに出す（[`surface`]）。
//! | 残留磁気 | `τ = m × B`（B は `geomag::field_about_eci_t`。地球以外は 0。同上） |
//! | 推力誤差 | `τ = Σ f_i (r_i − r_cm) × F_i d̂_i`（推力の線と重心のずれ。[`super::thrusters`]）。`f_i` はモードで噴射している時間の割合で、機体座標系で一定の平均トルクとして足す |

use super::thrusters::Thruster;
use crate::spec::{
    AnalysisError, ChannelKind, ensure_fraction, ensure_non_negative, ensure_positive,
};
use sat_astro::atmosphere::{AtmosphereModel, density_about_kg_m3};
use sat_astro::body::EnvironmentModel;
use sat_astro::ephem::CentralBody;
use sat_astro::force::{drag_force, earth_relative_wind_eci, srp_pressure};
use sat_astro::geomag::field_about_eci_t;
use sat_astro::math::{Mat3, Vec3};
use sat_astro::timeline::TimelineSample;
use sat_units::typed::Fraction;
use serde::{Deserialize, Serialize};
use surface::SurfaceModel;
pub use surface::{DisturbanceGeometry, DisturbanceSurface, SurfaceSample, SurfaceValueSource};

/// 擾乱を求めるための機体の値。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DisturbanceSpacecraft {
    /// 重心まわりの慣性テンソル [kg·m²]（機体座標系）
    pub inertia: Mat3,
    /// 空力の面（Cd・面積・圧力中心）。省略時は空力トルクを 0 とする
    #[serde(default)]
    pub aero: Option<DisturbanceSurface>,
    /// 太陽輻射圧の面（Cr・面積・圧力中心）。省略時は SRP トルクを 0 とする
    #[serde(default)]
    pub srp: Option<DisturbanceSurface>,
    /// 面積と圧力中心を出す部品の箱と重心（面の面積か圧力中心が書かれていないときに使う）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<DisturbanceGeometry>,
    /// 残留磁気モーメント [A·m²]（機体座標系）。省略時は [`DEFAULT_RESIDUAL_DIPOLE`]（0 = 磁気トルクを考えない）
    #[serde(default)]
    pub residual_dipole: Option<Vec3>,
    /// 大気モデル（省略時は指数大気）
    #[serde(default)]
    pub atmosphere: AtmosphereModel,
    /// 大気密度に掛ける倍率（モデルの不確かさを見込む）。省略時 1
    #[serde(default)]
    pub density_scale: Option<f64>,
    /// 推力誤差（噴射するスラスタ）。省略時は推力誤差のトルクを考えない（種類の一覧にも出さない）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thrust: Option<ThrustDisturbance>,
}

/// 推力誤差の擾乱: 噴射するスラスタと重心。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThrustDisturbance {
    /// 重心 [m]（機体座標系）
    pub center_of_mass: Vec3,
    /// 噴射するスラスタ（噴射の割合が 0 のものは入れなくてよい）
    pub firing: Vec<FiringThruster>,
}

/// 噴射するスラスタ 1 本と、噴射している時間の割合。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FiringThruster {
    pub thruster: Thruster,
    /// 噴射している時間の割合（0〜1。モードの時間割合）
    pub firing_fraction: Fraction,
}

impl ThrustDisturbance {
    /// 時間平均の推力誤差のトルク `Σ f_i τ_i` [N·m]（機体座標系）。スラスタの値が不正なら `InvalidInput`。
    pub fn torque(&self) -> Result<Vec3, AnalysisError> {
        let mut total = Vec3::ZERO;
        for f in &self.firing {
            let share = f.firing_fraction.value();
            ensure_fraction(&format!("Thruster {} burn fraction", f.thruster.id), share)?;
            total += f.thruster.torque_about(self.center_of_mass)?.torque * share;
        }
        Ok(total)
    }
}

/// 残留磁気モーメントが書かれていないときの値 [A·m²]。0（機器の残留磁気を見込まない）。
pub const DEFAULT_RESIDUAL_DIPOLE: Vec3 = Vec3::ZERO;

/// 圧力中心 − 重心が書かれておらず、部品の箱も無いときの値 [m]。0（圧力中心を重心に置く。空力・太陽輻射圧のトルクは 0）。
pub const DEFAULT_CP_OFFSET: Vec3 = Vec3::ZERO;

/// 種類ごとの擾乱トルク [N·m]（機体座標系）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DisturbanceTorques {
    pub gravity_gradient: Vec3,
    pub aero: Vec3,
    pub srp: Vec3,
    pub magnetic: Vec3,
    pub thrust: Vec3,
    /// この点の空力の面積と腕（空力トルクを求めた点だけ）
    pub aero_surface: Option<SurfaceSample>,
    /// この点の太陽輻射圧の面積と腕（圧力が 0 の食の点は求めないので `None`）
    pub srp_surface: Option<SurfaceSample>,
}

/// 擾乱の種類。**表示名は持たない**（service の語彙が日本語と英語で引く）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DisturbanceKind {
    GravityGradient,
    Aero,
    Srp,
    Magnetic,
    /// 推力誤差（推力の線と重心のずれ）
    Thrust,
}

impl DisturbanceKind {
    /// すべての種類（出力に並べる順）。
    pub const ALL: [Self; 5] = [
        Self::GravityGradient,
        Self::Aero,
        Self::Srp,
        Self::Magnetic,
        Self::Thrust,
    ];

    /// 機械向けの名前（時系列の key `torque:<名前>` に使う。JSON の綴りと同じ）。
    pub fn key(self) -> &'static str {
        match self {
            Self::GravityGradient => "gravity_gradient",
            Self::Aero => "aero",
            Self::Srp => "srp",
            Self::Magnetic => "magnetic",
            Self::Thrust => "thrust",
        }
    }

    /// この種類のトルクの時系列の種類。
    pub fn channel(self) -> ChannelKind {
        match self {
            Self::GravityGradient => ChannelKind::GravityGradientTorque,
            Self::Aero => ChannelKind::AeroTorque,
            Self::Srp => ChannelKind::SrpTorque,
            Self::Magnetic => ChannelKind::MagneticTorque,
            Self::Thrust => ChannelKind::ThrustTorque,
        }
    }
}

impl DisturbanceTorques {
    /// 合計 [N·m]。
    pub fn total(&self) -> Vec3 {
        self.gravity_gradient + self.aero + self.srp + self.magnetic + self.thrust
    }

    /// 種類 `kind` のトルク [N·m]。
    pub fn of(&self, kind: DisturbanceKind) -> Vec3 {
        match kind {
            DisturbanceKind::GravityGradient => self.gravity_gradient,
            DisturbanceKind::Aero => self.aero,
            DisturbanceKind::Srp => self.srp,
            DisturbanceKind::Magnetic => self.magnetic,
            DisturbanceKind::Thrust => self.thrust,
        }
    }
}

impl DisturbanceSpacecraft {
    /// 値が物理的にありうるかを調べる。
    pub fn validate(&self) -> Result<(), AnalysisError> {
        for (i, row) in self.inertia.m.iter().enumerate() {
            ensure_positive("Inertia tensor diagonal", row[i])?;
        }
        if !self.inertia.m.iter().flatten().all(|v| v.is_finite()) {
            return Err(AnalysisError::InvalidInput(
                "Inertia tensor is not finite".to_owned(),
            ));
        }
        ensure_non_negative("Atmospheric density scale", self.density_scale())?;
        let has_boxes = SurfaceModel::new(self.geometry.as_ref()).has_geometry();
        for (name, s) in [("Aerodynamic", &self.aero), ("SRP", &self.srp)] {
            let Some(s) = s else { continue };
            ensure_non_negative(&format!("{name} coefficient"), s.coefficient)?;
            match s.area {
                Some(a) => ensure_non_negative(&format!("{name} area"), a.value())?,
                None if !has_boxes => {
                    return Err(AnalysisError::InvalidInput(format!(
                        "{name} area is not written and no part has a size to take it from"
                    )));
                }
                None => {}
            }
        }
        if let Some(t) = &self.thrust {
            t.torque()?;
        }
        Ok(())
    }

    /// 出力に並べる擾乱の種類（[`DisturbanceKind::ALL`] の順）。推力誤差は `thrust` があるときだけ。
    pub fn kinds(&self) -> Vec<DisturbanceKind> {
        DisturbanceKind::ALL
            .into_iter()
            .filter(|k| *k != DisturbanceKind::Thrust || self.thrust.is_some())
            .collect()
    }

    /// 残留磁気モーメント [A·m²]（省略時は [`DEFAULT_RESIDUAL_DIPOLE`]）。
    pub fn residual_dipole(&self) -> Vec3 {
        self.residual_dipole.unwrap_or(DEFAULT_RESIDUAL_DIPOLE)
    }

    /// 大気密度に掛ける倍率（省略時 1 = モデルのまま）。
    pub fn density_scale(&self) -> f64 {
        self.density_scale.unwrap_or(1.0)
    }

    /// 値を調べて、時系列の点ごとにトルクを求める評価器にする（箱の幾何と、向きごとの投影の使い回しを持つ）。
    pub fn model(&self) -> Result<DisturbanceModel<'_>, AnalysisError> {
        self.validate()?;
        Ok(DisturbanceModel {
            sc: self,
            surface: SurfaceModel::new(self.geometry.as_ref()),
        })
    }
}

/// 擾乱トルクの評価器（[`DisturbanceSpacecraft::model`]）。
pub struct DisturbanceModel<'a> {
    sc: &'a DisturbanceSpacecraft,
    surface: SurfaceModel,
}

impl DisturbanceModel<'_> {
    /// 時系列の 1 点での擾乱トルク。`central` は軌道の中心天体（`TimelineSpec::orbit` の `central`）。
    /// μ・磁場・大気は中心天体のもの（磁場と大気のモデルは地球だけで、ほかの中心天体では空力・磁気トルクは 0。
    /// 0 が正しいのかモデルが無いのかは [`DisturbanceSpacecraft::environment_check`] が伝える）。
    pub fn torques(&self, central: CentralBody, s: &TimelineSample) -> DisturbanceTorques {
        let sc = self.sc;
        let r_body = s.to_body(s.r_eci);
        let b_eci = field_about_eci_t(central, s.r_eci, s.epoch);
        let mut out = DisturbanceTorques {
            gravity_gradient: gravity_gradient(&sc.inertia, r_body, central.gm()),
            magnetic: magnetic(sc.residual_dipole(), s.to_body(b_eci)),
            // 値は validate で調べてある（不正なら 0 にせず、解析の前にエラーになる）
            thrust: sc
                .thrust
                .as_ref()
                .and_then(|t| t.torque().ok())
                .unwrap_or(Vec3::ZERO),
            ..Default::default()
        };
        // 大気のモデルは地球だけ（ほかの天体は 0 のまま。理由は [`DisturbanceSpacecraft::environment_check`]）
        let aero_surface = sc
            .aero
            .as_ref()
            .filter(|_| central.atmosphere_model() == EnvironmentModel::Modelled);
        if let Some(surface) = aero_surface {
            let rho =
                density_about_kg_m3(central, &sc.atmosphere, s.r_eci, s.epoch) * sc.density_scale();
            let wind = s.to_body(earth_relative_wind_eci(s.r_eci, s.v_eci));
            // 力は流れの向きに沿う。投影は流れの向き（機体座標系）で見る
            let sample = self.surface.evaluate(surface, wind);
            out.aero = aero(
                surface.coefficient * sample.area,
                sample.cp_offset,
                rho,
                wind,
            );
            out.aero_surface = Some(sample);
        }
        if let Some(surface) = &sc.srp {
            let pressure = srp_pressure(s.sun_distance_m) * s.sunlit.clamp(0.0, 1.0);
            // 食の中は圧力が 0 でトルクも 0（投影は解かない）
            if pressure > 0.0 {
                let sun = s.sun_dir_body();
                let sample = self.surface.evaluate(surface, sun);
                out.srp = srp(
                    surface.coefficient * sample.area,
                    sample.cp_offset,
                    pressure,
                    sun,
                );
                out.srp_surface = Some(sample);
            }
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

/// 空力トルク [N·m]。`coefficient_area` は Cd·A [m²]、`cp_offset` は圧力中心 − 重心 [m]、`rho` は大気密度 [kg/m³]、`v_rel_body` は大気に対する機体の速度 [m/s]（機体座標系）。
/// 抗力は `sat_astro::force::drag_force`。
pub fn aero(coefficient_area: f64, cp_offset: Vec3, rho: f64, v_rel_body: Vec3) -> Vec3 {
    let force = drag_force(rho, coefficient_area, v_rel_body);
    cp_offset.cross(force)
}

/// 太陽輻射圧トルク [N·m]。`coefficient_area` は Cr·A [m²]、`cp_offset` は圧力中心 − 重心 [m]、`pressure` は日照率込みの輻射圧 [N/m²]、`sun_dir_body` は機体 → 太陽の方向。
pub fn srp(coefficient_area: f64, cp_offset: Vec3, pressure: f64, sun_dir_body: Vec3) -> Vec3 {
    let Some(sun) = sun_dir_body.normalized() else {
        return Vec3::ZERO;
    };
    let force = sun * (-pressure * coefficient_area);
    cp_offset.cross(force)
}

/// 残留磁気トルク [N·m]。`dipole` [A·m²]、`b_body` は磁束密度 [T]（どちらも機体座標系）。
pub fn magnetic(dipole: Vec3, b_body: Vec3) -> Vec3 {
    dipole.cross(b_body)
}

mod environment;
pub mod surface;

#[cfg(test)]
mod tests {
    use super::*;
    use sat_astro::constants::{C_LIGHT, MU_EARTH};
    use sat_units::typed::Qty;

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
        let tau = aero(
            0.1,
            Vec3::new(0.0, 0.01, 0.0),
            1e-12,
            Vec3::new(7500.0, 0.0, 0.0),
        );
        assert!(close(tau.z, 2.8125e-8, 1e-12), "{tau:?}");
    }

    #[test]
    fn srp_and_magnetic_by_hand() {
        // P = 1361/c = 4.54e-6 N/m²、Cr·A = 0.1 m²、太陽 +x、CP が +z に 2 cm
        let p = 1361.0 / C_LIGHT;
        let tau = srp(0.1, Vec3::new(0.0, 0.0, 0.02), p, Vec3::X);
        // F = (−P·0.1, 0, 0)、τ = (0,0,0.02) × F = (0, −0.02·P·0.1, 0)
        assert!(close(tau.y, -0.02 * p * 0.1, 1e-12));
        // m = 0.01 A·m²（+z）、B = 30 µT（+x）→ τ = m × B = (0, 3e-7, 0)
        let m = magnetic(Vec3::new(0.0, 0.0, 0.01), Vec3::new(3e-5, 0.0, 0.0));
        assert!(close(m.y, 3e-7, 1e-12));
    }

    #[test]
    fn thrust_misalignment_by_hand() {
        // 推力 1 N（+z）、推力の線が重心から x に 2 mm ずれる → τ = (0.002, 0, 0) × (0, 0, 1) = (0, −2 mN·m, 0)。
        // 噴射は時間の 25 % → 平均 (0, −0.5 mN·m, 0)
        let t = ThrustDisturbance {
            center_of_mass: Vec3::new(-0.002, 0.0, 0.1),
            firing: vec![FiringThruster {
                thruster: Thruster {
                    id: "T".to_owned(),
                    thrust: Qty::from_raw(1.0),
                    isp: Qty::from_raw(220.0),
                    position: Vec3::new(0.0, 0.0, -0.2),
                    direction: Vec3::Z,
                },
                firing_fraction: Fraction::new(0.25),
            }],
        };
        let tau = t.torque().unwrap();
        assert!((tau - Vec3::new(0.0, -5e-4, 0.0)).norm() < 1e-15, "{tau:?}");
        let bad = ThrustDisturbance {
            firing: vec![FiringThruster {
                firing_fraction: Fraction::new(1.5),
                ..t.firing[0].clone()
            }],
            ..t
        };
        assert!(bad.torque().is_err());
    }
}
