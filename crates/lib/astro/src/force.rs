//! 軌道の力モデル（高次重力場・第三体・太陽輻射圧・大気抵抗・固体潮汐・相対論・推力）。**力モデルはここだけ。**
//! 重力場の係数は `data/egm96_to70.txt`（70 次まで）を埋め込む。
//!
//! 項ごとの加速度は [`acceleration_breakdown`] が並べ、[`acceleration_eci`] はその和をとる（計算の道筋は 1 つ）。
//! 推力（[`thrust`]）は状態（質量・積んだ ΔV）と噴射の有無に依るので、推力を入れた伝搬（`crate::orbit::powered`）が
//! この和に足す。

mod gravity;
mod solid_tide;
mod terms;
pub mod thrust;

pub use gravity::EGM96_MAX_DEGREE;
pub use solid_tide::solid_tide_eci;
pub use terms::{
    drag_force, earth_drag_eci, earth_relative_wind_eci, earth_srp_eci, j2_perturbation,
    point_mass, relativity_eci, srp_pressure, third_body_eci, zonal_j2,
};

use crate::AstroError;
use crate::atmosphere::AtmosphereModel;
use crate::constants::MU_EARTH;
use crate::ephem::orientation::pole_eci;
use crate::ephem::{Body, CentralBody, position_eci_m};
use crate::math::Vec3;
use crate::shadow::{ShadowModel, central_shadow_factor};
use crate::state::OrbitState;
use crate::time::Epoch;
use serde::{Deserialize, Serialize};

/// 数値伝搬で使う力の組み合わせ。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ForceConfig {
    pub gravity: GravityModel,
    /// 第三体として加える天体
    #[serde(default)]
    pub third_bodies: Vec<Body>,
    #[serde(default)]
    pub srp: Option<SrpParams>,
    #[serde(default)]
    pub drag: Option<DragParams>,
    /// 固体地球潮汐（2 次、IERS 2010）
    #[serde(default)]
    pub solid_tide: bool,
    /// 一般相対論の補正（Schwarzschild 項）
    #[serde(default)]
    pub relativity: bool,
}

impl ForceConfig {
    /// 重力場だけの設定（第三体・SRP・抗力・潮汐・相対論なし）。
    pub fn gravity_only(gravity: GravityModel) -> Self {
        Self {
            gravity,
            third_bodies: Vec::new(),
            srp: None,
            drag: None,
            solid_tide: false,
            relativity: false,
        }
    }
}

/// 力の項（[`acceleration_breakdown`] の内訳の名前）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "term", content = "body", rename_all = "snake_case")]
pub enum ForceTerm {
    /// 中心天体の点質量
    CentralBody,
    /// 重力場の非球形の項（J2 または球面調和の 1 次以上）
    NonSpherical,
    /// 第三体
    ThirdBody(Body),
    /// 地球（中心天体が地球以外のときの第三体）
    EarthThirdBody,
    /// 太陽輻射圧
    SolarRadiationPressure,
    /// 大気抵抗
    Drag,
    /// 固体潮汐
    SolidTide,
    /// 一般相対論
    Relativity,
}

/// 地球の重力場のモデル。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "model", rename_all = "snake_case")]
pub enum GravityModel {
    PointMass,
    J2,
    /// EGM96 の球面調和関数（次数・位数は 70 まで）
    Harmonics {
        degree: u8,
        order: u8,
    },
}

/// 太陽輻射圧（キャノンボール）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SrpParams {
    /// 反射係数 Cr
    pub cr: f64,
    /// 面積質量比 [m²/kg]
    pub area_to_mass: f64,
}

/// 大気抵抗（キャノンボール。大気モデルは [`crate::atmosphere::AtmosphereModel`] で選ぶ）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DragParams {
    /// 抵抗係数 Cd
    pub cd: f64,
    /// 面積質量比 [m²/kg]
    pub area_to_mass: f64,
    /// 大気モデル（既定は指数大気）
    #[serde(default)]
    pub atmosphere: AtmosphereModel,
}

/// ECI での加速度 [m/s²]（[`acceleration_breakdown`] の和）。
///
/// 重力場の係数表が使えない（埋め込みデータの破損）ときは点質量＋J2 で代える。[`validate`] で事前に検出できる。
pub fn acceleration_eci(config: &ForceConfig, epoch: Epoch, state: &OrbitState) -> Vec3 {
    acceleration_breakdown(config, epoch, state)
        .iter()
        .fold(Vec3::ZERO, |acc, (_, a)| acc + *a)
}

/// 項ごとの加速度（ECI）[m/s²]。設定で有効な項だけを、決まった順に並べる（地球が中心）。
pub fn acceleration_breakdown(
    config: &ForceConfig,
    epoch: Epoch,
    state: &OrbitState,
) -> Vec<(ForceTerm, Vec3)> {
    acceleration_breakdown_about(CentralBody::Earth, config, epoch, state)
}

/// 中心天体 `central` のまわりの軌道の加速度（`state` は中心天体からの相対、向きは ECI）[m/s²]。
pub fn acceleration_eci_about(
    central: CentralBody,
    config: &ForceConfig,
    epoch: Epoch,
    state: &OrbitState,
) -> Vec3 {
    acceleration_breakdown_about(central, config, epoch, state)
        .iter()
        .fold(Vec3::ZERO, |acc, (_, a)| acc + *a)
}

/// 中心天体 `central` のまわりの項ごとの加速度。地球以外では重力場は点質量＋J2（自転軸まわり）で、
/// 地球を第三体として自動で加える。抗力・固体潮汐・EGM96 は地球だけ（[`validate_about`] が弾く）。
pub fn acceleration_breakdown_about(
    central: CentralBody,
    config: &ForceConfig,
    epoch: Epoch,
    state: &OrbitState,
) -> Vec<(ForceTerm, Vec3)> {
    let r = state.r_eci;
    let earth = central == CentralBody::Earth;
    let mut out = Vec::with_capacity(5 + config.third_bodies.len());
    let (central_acc, rest) = if earth {
        terms::gravity_split(config.gravity, epoch, r)
    } else {
        let (mu, re, j2) = (central.gm(), central.radius_m(), central.j2());
        let ns = match config.gravity {
            GravityModel::PointMass => Vec3::ZERO,
            _ => zonal_j2(mu, re, j2, pole_eci(central, epoch), r),
        };
        (point_mass(mu, r), ns)
    };
    out.push((ForceTerm::CentralBody, central_acc));
    if config.gravity != GravityModel::PointMass {
        out.push((ForceTerm::NonSpherical, rest));
    }
    let origin = central.position_eci_m(epoch);
    let r_sun = (!config.third_bodies.is_empty() || config.srp.is_some())
        .then(|| position_eci_m(Body::Sun, epoch) - origin);
    if let Some(r_sun) = r_sun {
        let others = config
            .third_bodies
            .iter()
            .filter(|b| Some(**b) != central.as_body());
        for body in others {
            let r_b = if *body == Body::Sun {
                r_sun
            } else {
                position_eci_m(*body, epoch) - origin
            };
            out.push((
                ForceTerm::ThirdBody(*body),
                third_body_eci(body.gm(), r_b, r),
            ));
        }
        if let Some(srp) = &config.srp {
            let nu = central_shadow_factor(central, r, r_sun, ShadowModel::Conical);
            let a = terms::srp_with_shadow(srp, r, r_sun, nu);
            out.push((ForceTerm::SolarRadiationPressure, a));
        }
    }
    if !earth {
        out.push((
            ForceTerm::EarthThirdBody,
            third_body_eci(MU_EARTH, -origin, r),
        ));
    }
    if let Some(drag) = config.drag.as_ref().filter(|_| earth) {
        out.push((ForceTerm::Drag, earth_drag_eci(drag, epoch, state)));
    }
    if config.solid_tide && earth {
        out.push((ForceTerm::SolidTide, solid_tide_eci(epoch, r)));
    }
    if config.relativity {
        out.push((ForceTerm::Relativity, relativity_eci(central.gm(), state)));
    }
    out
}

/// 力の設定を検査する（次数・位数の範囲、係数の符号、重力場の表）。地球が中心。
pub fn validate(config: &ForceConfig) -> Result<(), AstroError> {
    validate_about(CentralBody::Earth, config)
}

/// 中心天体 `central` での力の設定を検査する。EGM96・抗力・固体潮汐は地球だけ。
pub fn validate_about(central: CentralBody, config: &ForceConfig) -> Result<(), AstroError> {
    if central != CentralBody::Earth {
        let earth_only = matches!(config.gravity, GravityModel::Harmonics { .. })
            || config.drag.is_some()
            || config.solid_tide;
        if earth_only {
            return Err(AstroError::InvalidInput(format!(
                "EGM96, drag and solid tides are only available when Earth is the central body (central: {central:?})"
            )));
        }
    }
    if let GravityModel::Harmonics { degree, order } = config.gravity {
        if degree > EGM96_MAX_DEGREE || order > degree {
            return Err(AstroError::InvalidInput(format!(
                "Gravity field degree and order must satisfy 0 <= order <= degree <= {EGM96_MAX_DEGREE} ({degree}x{order})"
            )));
        }
        if gravity::egm96().is_none() {
            return Err(AstroError::InvalidInput(
                "Cannot read the EGM96 coefficient table".into(),
            ));
        }
    }
    let ok = |x: f64| x.is_finite() && x >= 0.0;
    if let Some(s) = &config.srp
        && !(ok(s.cr) && ok(s.area_to_mass))
    {
        return Err(AstroError::InvalidInput(
            "SRP Cr and area-to-mass ratio must be 0 or more".into(),
        ));
    }
    if let Some(d) = &config.drag
        && !(ok(d.cd) && ok(d.area_to_mass) && atmosphere_ok(&d.atmosphere))
    {
        return Err(AstroError::InvalidInput(
            "Drag Cd and area-to-mass ratio must be 0 or more".into(),
        ));
    }
    Ok(())
}

/// NRLMSISE-00 の宇宙天気の値が有限で負でないか。
fn atmosphere_ok(model: &AtmosphereModel) -> bool {
    match *model {
        AtmosphereModel::Exponential => true,
        AtmosphereModel::Nrlmsise00 { f107, f107a, ap } => {
            [f107, f107a, ap].iter().all(|v| v.is_finite() && *v >= 0.0)
        }
    }
}

#[cfg(test)]
mod tests;
