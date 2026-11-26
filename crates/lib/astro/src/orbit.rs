//! 軌道要素と伝播（Kepler / J2 永年項 / 力モデルの数値積分）、周期、太陽同期傾斜角、β 角。
//! **軌道伝播はここだけ。**
//!
//! 平均要素の J2 永年変化率は Vallado（*Fundamentals of Astrodynamics and Applications*, 4th ed.）eq. 9-41
//! に従う。数値伝播（[`Propagator::Numerical`]）は [`numerical`] にあり、[`crate::force`] の加速度を
//! [`crate::ode::Dop853`] で積分する（Cowell 法）。推力を入れた伝播（[`propagate_powered`]）は `powered` にあり、
//! 同じ積分器と力モデルに推力の項（[`crate::force::thrust`]）と質量を足す。平均の長半径（J2 の短周期項を引く）は `mean`。
//! 円制限三体問題（ラグランジュ点・ハロー軌道の補正・回転系と慣性系の変換）の数学は [`cr3bp`] にある。
//! 解析が軌道から読むもの（時刻での状態・中心天体・元期・繰り返しの長さ）の共通の入口は `trajectory`（[`Trajectory`]・[`Cycle`]）。
//! 軌道の出どころは、要素で書かれた軌道（[`OrbitSpec`]）と、状態ベクトルから多体の力で数値伝播する軌道（[`StateSpec`]。`state_spec`）と、
//! ラグランジュ点・ハロー軌道を暦の 2 天体の位置に載せた軌跡（[`SynodicSpec`]。`synodic`）。

mod analytic;
mod approach;
mod beta;
pub mod cr3bp;
mod local_time;
mod mean;
mod numerical;
mod powered;
mod state_spec;
mod synodic;
mod track;
mod trajectory;
pub use crate::kepler::{perifocal_to_eci, solve_kepler};
pub use crate::state::OrbitState;

pub use analytic::{j2_secular_rates, j2_secular_rates_about};
pub use approach::{ClosestApproach, closest_approach};
pub use beta::{beta_angle_rad, raan_for_beta};
pub use local_time::{
    ltan_from_raan, raan_from_ltan, raan_from_ltan_about, raan_from_ltdn, raan_from_ltdn_about,
};
pub use mean::{mean_semi_major_axis_m, orbit_averaged_elements, osculating_state_from_mean};
pub use numerical::{HELIOCENTRIC_INTEGRATOR, NUMERICAL_INTEGRATOR, propagate_states};
pub use powered::{PoweredArc, PoweredSample, PoweredTarget, ThrustArc, propagate_powered};
pub use state_spec::StateSpec;
pub use synodic::{HaloSpec, SynodicSpec};
pub use track::OrbitTrack;
pub(crate) use track::arc_index;
pub use trajectory::{Cycle, LIBRATION_SOURCE, Trajectory, measures_range_from_earth};

use crate::AstroError;
use crate::ephem::CentralBody;
use crate::math::Vec3;
use crate::time::Epoch;
use core::f64::consts::TAU;
use sat_units::typed::{Angle, Length, Qty};
use serde::{Deserialize, Serialize};

/// 解析の入力として渡す軌道の仕様。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OrbitSpec {
    pub epoch: Epoch,
    pub elements: OrbitElements,
    pub propagator: Propagator,
    /// 中心天体（既定は地球）。位置・速度は中心天体からの相対で、向きは ECI（J2000 平均赤道）。
    #[serde(default)]
    pub central: CentralBody,
}

/// 伝播方法の既定は二体 + J2 永年項。
impl Default for Propagator {
    fn default() -> Self {
        Self::J2
    }
}

/// 平均軌道要素（元期における値）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OrbitElements {
    pub semi_major_axis: Qty<Length>,
    pub eccentricity: f64,
    pub inclination: Qty<Angle>,
    pub raan: Qty<Angle>,
    pub arg_perigee: Qty<Angle>,
    pub mean_anomaly: Qty<Angle>,
}

/// 離心率を書かなかったときの値（円軌道）。
pub const DEFAULT_ECCENTRICITY: f64 = 0.0;

impl OrbitElements {
    /// 書かれていない要素を慣例の値で埋めて作る: 離心率は [`DEFAULT_ECCENTRICITY`]、
    /// RAAN・近点引数・平均近点角は 0（円軌道では近点引数と平均近点角の分け方は任意）。
    pub fn with_defaults(
        semi_major_axis: Qty<Length>,
        inclination: Qty<Angle>,
        eccentricity: Option<f64>,
        raan: Option<Qty<Angle>>,
        arg_perigee: Option<Qty<Angle>>,
        mean_anomaly: Option<Qty<Angle>>,
    ) -> Self {
        Self {
            semi_major_axis,
            eccentricity: eccentricity.unwrap_or(DEFAULT_ECCENTRICITY),
            inclination,
            raan: raan.unwrap_or(Qty::ZERO),
            arg_perigee: arg_perigee.unwrap_or(Qty::ZERO),
            mean_anomaly: mean_anomaly.unwrap_or(Qty::ZERO),
        }
    }
}

/// 伝播の方法。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum Propagator {
    /// 二体問題
    Kepler,
    /// 二体＋J2 の永年項（RAAN・近地点引数・平均近点角のドリフト）
    J2,
    /// 力モデルを DOP853（[`crate::ode::Dop853`]）で数値積分する。
    /// 要素は元期の**接触要素**として初期状態にする（[`Propagator::J2`] の平均要素とは意味が違い、
    /// 同じ数値でも長半径の差で周回あたり ~90 km 進行方向にずれる）。
    Numerical { forces: crate::force::ForceConfig },
}

/// 古典軌道要素（SI・rad）。内部計算と [`osculating_elements`] の結果に使う。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeplerElements {
    /// 長半径 [m]
    pub a_m: f64,
    pub e: f64,
    /// 傾斜角 [rad]
    pub i_rad: f64,
    /// 昇交点赤経 [rad]
    pub raan_rad: f64,
    /// 近地点引数 [rad]
    pub argp_rad: f64,
    /// 平均近点角 [rad]
    pub mean_anomaly_rad: f64,
}

impl OrbitSpec {
    /// 元期・要素・伝播の方法から作る（中心天体などの追加のフィールドは既定値）。
    /// フィールドが増えても呼び出し側を変えずに済むよう、構造体リテラルよりこれを使う。
    pub fn new(epoch: Epoch, elements: OrbitElements, propagator: Propagator) -> Self {
        Self {
            epoch,
            elements,
            propagator,
            central: CentralBody::Earth,
        }
    }
    /// 地球を回る円軌道の仕様を作る（高度・傾斜角・RAAN）。伝播は J2 永年項（[`Propagator::J2`]）にする。
    /// 近地点引数と平均近点角は 0（元期に昇交点にいる）。
    pub fn circular(
        epoch: Epoch,
        altitude: Qty<Length>,
        inclination: Qty<Angle>,
        raan: Qty<Angle>,
    ) -> Self {
        Self::circular_about(CentralBody::Earth, epoch, altitude, inclination, raan)
    }
    /// 中心天体 `central` を回る円軌道（高度は赤道半径から。傾斜角・RAAN は ECI の赤道基準）。
    pub fn circular_about(
        central: CentralBody,
        epoch: Epoch,
        altitude: Qty<Length>,
        inclination: Qty<Angle>,
        raan: Qty<Angle>,
    ) -> Self {
        Self {
            central,
            epoch,
            elements: OrbitElements {
                semi_major_axis: Qty::from_raw(semi_major_axis_from_altitude(
                    central,
                    altitude.value(),
                )),
                eccentricity: 0.0,
                inclination,
                raan,
                arg_perigee: Qty::from_raw(0.0),
                mean_anomaly: Qty::from_raw(0.0),
            },
            propagator: Propagator::J2,
        }
    }
    /// 元期から `t_s` 秒後の状態。数値伝播で多数の時刻が要るときは [`OrbitSpec::states_at`] を使う。
    pub fn state_at(&self, t_s: f64) -> Result<OrbitState, AstroError> {
        match &self.propagator {
            Propagator::Kepler | Propagator::J2 => Ok(analytic::Analytic::new(self)?.state_at(t_s)),
            Propagator::Numerical { .. } => self.states_at(&[t_s])?.pop().ok_or_else(|| {
                AstroError::InvalidInput("Numerical propagation returned no result".into())
            }),
        }
    }
    /// 昇順の時刻列 `times_s`（元期からの秒）での状態をまとめて求める（数値伝播は 1 回の積分で密出力を使う）。
    pub fn states_at(&self, times_s: &[f64]) -> Result<Vec<OrbitState>, AstroError> {
        match &self.propagator {
            Propagator::Kepler | Propagator::J2 => {
                let a = analytic::Analytic::new(self)?;
                Ok(times_s.iter().map(|&t| a.state_at(t)).collect())
            }
            Propagator::Numerical { forces } => {
                let x0 = elements_to_state(&self.kepler_elements()?, self.central.gm());
                numerical::states_at(self.central, forces, self.epoch, x0, times_s)
            }
        }
    }
    /// 軌道周期 [s]（Kepler 周期 2π√(a³/μ)、μ は中心天体）。
    pub fn period_s(&self) -> f64 {
        let a = self.elements.semi_major_axis.value();
        TAU / mean_motion_rad_s(self.central.gm(), a)
    }
    /// 高度 [m] = 長半径 − 中心天体の赤道半径（円軌道の高度。楕円では平均の高さ）。
    pub fn altitude_m(&self) -> f64 {
        altitude_from_semi_major_axis(self.central, self.elements.semi_major_axis.value())
    }
    /// 元期の要素を SI・rad の [`KeplerElements`] にし、値を検査する。
    pub fn kepler_elements(&self) -> Result<KeplerElements, AstroError> {
        let e = &self.elements;
        let el = KeplerElements {
            a_m: e.semi_major_axis.value(),
            e: e.eccentricity,
            i_rad: e.inclination.value(),
            raan_rad: e.raan.value(),
            argp_rad: e.arg_perigee.value(),
            mean_anomaly_rad: e.mean_anomaly.value(),
        };
        let finite = [
            el.a_m,
            el.e,
            el.i_rad,
            el.raan_rad,
            el.argp_rad,
            el.mean_anomaly_rad,
        ]
        .iter()
        .all(|v| v.is_finite());
        if !finite {
            return Err(AstroError::InvalidInput(
                "Orbital elements contain a non-finite value".into(),
            ));
        }
        if el.a_m <= 0.0 {
            return Err(AstroError::InvalidInput(format!(
                "Semi-major axis must be positive ({} m)",
                el.a_m
            )));
        }
        if !(0.0..1.0).contains(&el.e) {
            return Err(AstroError::InvalidInput(format!(
                "Eccentricity must be at least 0 and less than 1 ({})",
                el.e
            )));
        }
        Ok(el)
    }
    /// 元期から `t_s` 秒後の平均要素（ECI 基準。J2・数値伝播なら J2 永年項込み、Kepler なら二体）。
    pub fn mean_elements_at(&self, t_s: f64) -> Result<KeplerElements, AstroError> {
        let mut analytic_spec = self.clone();
        if matches!(self.propagator, Propagator::Numerical { .. }) {
            analytic_spec.propagator = Propagator::J2;
        }
        analytic::Analytic::new(&analytic_spec)?.mean_elements_at(t_s)
    }
}

/// 高度 [m] → 長半径 [m]（中心天体の赤道半径を足す）。**高度 ⇄ 長半径の換算はここ（と逆の関数）だけ。**
pub fn semi_major_axis_from_altitude(central: CentralBody, altitude_m: f64) -> f64 {
    central.radius_m() + altitude_m
}

/// 長半径 [m] → 高度 [m]（中心天体の赤道半径を引く）。
pub fn altitude_from_semi_major_axis(central: CentralBody, semi_major_axis_m: f64) -> f64 {
    semi_major_axis_m - central.radius_m()
}

/// 平均運動 n = √(μ/a³) [rad/s]。
pub fn mean_motion_rad_s(mu: f64, semi_major_axis_m: f64) -> f64 {
    (mu / (semi_major_axis_m * semi_major_axis_m * semi_major_axis_m)).sqrt()
}

/// 古典軌道要素 → ECI の位置・速度（重力定数 `mu` [m³/s²]）。
pub fn elements_to_state(el: &KeplerElements, mu: f64) -> OrbitState {
    let ea = solve_kepler(el.mean_anomaly_rad, el.e);
    let (se, ce) = ea.sin_cos();
    let b = (1.0 - el.e * el.e).sqrt();
    let r = el.a_m * (1.0 - el.e * ce);
    // 近点座標系（P: 近地点方向、Q: 進行方向）
    let p = Vec3::new(el.a_m * (ce - el.e), el.a_m * b * se, 0.0);
    let vk = (mu * el.a_m).sqrt() / r;
    let v = Vec3::new(-vk * se, vk * b * ce, 0.0);
    let rot = perifocal_to_eci(el.raan_rad, el.i_rad, el.argp_rad);
    OrbitState {
        r_eci: rot.mul_vec(p),
        v_eci: rot.mul_vec(v),
    }
}

/// ECI の位置・速度 → 接触軌道要素（楕円軌道。重力定数 `mu` [m³/s²]）。
/// 円軌道・赤道軌道で未定義になる角は 0 にし、その分を平均近点角に寄せる。
pub fn osculating_elements(state: &OrbitState, mu: f64) -> Result<KeplerElements, AstroError> {
    let (r, v) = (state.r_eci, state.v_eci);
    let rn = r.norm();
    let h = r.cross(v);
    let energy = 0.5 * v.norm_squared() - mu / rn;
    if energy.is_nan() || energy >= 0.0 || rn <= 0.0 {
        return Err(AstroError::InvalidInput("Not an elliptical orbit".into()));
    }
    let a = -mu / (2.0 * energy);
    let e_vec = v.cross(h) / mu - r / rn;
    let e = e_vec.norm();
    let i = (h.z / h.norm()).clamp(-1.0, 1.0).acos();
    let node = Vec3::Z.cross(h);
    let eps = 1e-11;
    let raan = if node.norm() > eps * h.norm() {
        node.y.atan2(node.x).rem_euclid(TAU)
    } else {
        0.0
    };
    let node_dir = if node.norm() > eps * h.norm() {
        node.normalized().unwrap_or(Vec3::X)
    } else {
        Vec3::X
    };
    let hn = h.normalized().unwrap_or(Vec3::Z);
    // 軌道面内の角は「昇交点方向から進行方向に測る」。
    let angle_in_plane = |x: Vec3| hn.dot(node_dir.cross(x)).atan2(node_dir.dot(x));
    let (argp, nu) = if e > eps {
        let argp = angle_in_plane(e_vec).rem_euclid(TAU);
        let nu = hn.dot(e_vec.cross(r)).atan2(e_vec.dot(r));
        (argp, nu)
    } else {
        (0.0, angle_in_plane(r))
    };
    let ea = 2.0 * ((1.0 - e).sqrt() * (nu / 2.0).sin()).atan2((1.0 + e).sqrt() * (nu / 2.0).cos());
    let m = (ea - e * ea.sin()).rem_euclid(TAU);
    Ok(KeplerElements {
        a_m: a,
        e,
        i_rad: i,
        raan_rad: raan,
        argp_rad: argp,
        mean_anomaly_rad: m,
    })
}

/// 地球を回る太陽同期軌道の傾斜角 [rad]（[`sso_inclination_about_rad`] の地球版）。
/// 成立しない（|cos i| > 1、長半径が大きすぎる）ときは NaN。
pub fn sso_inclination_rad(semi_major_axis_m: f64, eccentricity: f64) -> f64 {
    sso_inclination_about_rad(CentralBody::Earth, semi_major_axis_m, eccentricity)
        .unwrap_or(f64::NAN)
}

/// 中心天体 `central` を回る太陽同期軌道の傾斜角 [rad]（**天体の赤道に対する**傾斜角）。
///
/// J2 による RAAN の回転率 `−1.5 n J2 (R/p)² cos i` が、その天体が太陽を回る平均運動
/// （[`CentralBody::sun_mean_motion_rad_s`]）に一致する傾斜角。地球以外では要素の基準面（ECI の赤道）と
/// 天体の赤道が違うので、返す値を `OrbitSpec` の傾斜角（ECI 基準）にそのまま入れてはいけない。
/// 太陽と月（太陽を回る年が地球と同じで、モデルに独自の年が無い）は `InvalidInput`。
/// 長半径が大きすぎて傾斜角が存在しないときも `InvalidInput`。
pub fn sso_inclination_about_rad(
    central: CentralBody,
    semi_major_axis_m: f64,
    eccentricity: f64,
) -> Result<f64, AstroError> {
    let Some(sun_n) = central.sun_mean_motion_rad_s() else {
        return Err(AstroError::InvalidInput(
            "Sun-synchronous orbit is undefined for this central body (it has no year around the Sun)".into(),
        ));
    };
    let a = semi_major_axis_m;
    let p = a * (1.0 - eccentricity * eccentricity);
    let n = mean_motion_rad_s(central.gm(), a);
    let cos_i = -sun_n / (1.5 * n * central.j2() * (central.radius_m() / p).powi(2));
    if cos_i.abs() > 1.0 || !cos_i.is_finite() {
        return Err(AstroError::InvalidInput(
            "No sun-synchronous inclination exists for this semi-major axis".into(),
        ));
    }
    Ok(cos_i.acos())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod central_tests;
