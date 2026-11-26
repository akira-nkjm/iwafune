//! 解析的な伝播（二体・J2 永年項）。地球は ECI の要素のまま、ほかの中心天体は自転軸を z にした
//! 天体の赤道座標系で J2 の永年項を足し、ECI に戻す（自転軸は元期の向きで固定）。

use super::{
    KeplerElements, OrbitSpec, OrbitState, Propagator, elements_to_state, mean_motion_rad_s,
    osculating_elements,
};
use crate::AstroError;
use crate::constants::{J2_EARTH, MU_EARTH, R_EARTH};
use crate::ephem::CentralBody;
use crate::ephem::orientation::equatorial_frame;
use crate::math::Mat3;
use core::f64::consts::TAU;

/// J2 による永年変化率 (dΩ/dt, dω/dt, dM/dt − n) [rad/s]（地球）。
pub fn j2_secular_rates(a_m: f64, e: f64, i_rad: f64) -> (f64, f64, f64) {
    j2_secular_rates_about(MU_EARTH, R_EARTH, J2_EARTH, a_m, e, i_rad)
}

/// J2 による永年変化率（μ [m³/s²]・基準半径 [m]・J2 を指定。`i_rad` は天体の赤道に対する傾斜角）。
pub fn j2_secular_rates_about(
    mu: f64,
    re: f64,
    j2: f64,
    a_m: f64,
    e: f64,
    i_rad: f64,
) -> (f64, f64, f64) {
    let n = mean_motion_rad_s(mu, a_m);
    let p = a_m * (1.0 - e * e);
    let k = n * j2 * (re / p).powi(2);
    let c = i_rad.cos();
    let raan_dot = -1.5 * k * c;
    let argp_dot = 0.75 * k * (5.0 * c * c - 1.0);
    let m_dot = 0.75 * k * (1.0 - e * e).sqrt() * (3.0 * c * c - 1.0);
    (raan_dot, argp_dot, m_dot)
}

/// 解析的なプロパゲータ（元期の要素と、伝播に使う座標系）。
pub(super) struct Analytic {
    mu: f64,
    re: f64,
    /// J2 の永年項を足すときの J2（足さないなら `None`）
    j2: Option<f64>,
    /// 伝播の座標系での元期の要素
    el: KeplerElements,
    /// ECI → 伝播の座標系（地球と二体は `None` = ECI のまま）
    rot: Option<Mat3>,
}

impl Analytic {
    pub(super) fn new(spec: &OrbitSpec) -> Result<Self, AstroError> {
        let el = spec.kepler_elements()?;
        let central = spec.central;
        let (mu, re) = (central.gm(), central.radius_m());
        let j2 = (spec.propagator != Propagator::Kepler).then_some(central.j2());
        if central == CentralBody::Earth || j2.is_none() {
            return Ok(Self {
                mu,
                re,
                j2,
                el,
                rot: None,
            });
        }
        let rot = equatorial_frame(central, spec.epoch);
        let s = elements_to_state(&el, mu);
        let s_eq = OrbitState {
            r_eci: rot.mul_vec(s.r_eci),
            v_eci: rot.mul_vec(s.v_eci),
        };
        Ok(Self {
            mu,
            re,
            j2,
            el: osculating_elements(&s_eq, mu)?,
            rot: Some(rot),
        })
    }

    /// 伝播の座標系での `t_s` 秒後の要素。
    fn elements_at(&self, t_s: f64) -> KeplerElements {
        let el = &self.el;
        let n = mean_motion_rad_s(self.mu, el.a_m);
        let (raan_dot, argp_dot, m_dot) = match self.j2 {
            Some(j2) => j2_secular_rates_about(self.mu, self.re, j2, el.a_m, el.e, el.i_rad),
            None => (0.0, 0.0, 0.0),
        };
        KeplerElements {
            raan_rad: (el.raan_rad + raan_dot * t_s).rem_euclid(TAU),
            argp_rad: (el.argp_rad + argp_dot * t_s).rem_euclid(TAU),
            mean_anomaly_rad: (el.mean_anomaly_rad + (n + m_dot) * t_s).rem_euclid(TAU),
            ..*el
        }
    }

    /// `t_s` 秒後の ECI の状態（中心天体からの相対）。
    pub(super) fn state_at(&self, t_s: f64) -> OrbitState {
        let s = elements_to_state(&self.elements_at(t_s), self.mu);
        match &self.rot {
            None => s,
            Some(rot) => {
                let back = rot.transpose();
                OrbitState {
                    r_eci: back.mul_vec(s.r_eci),
                    v_eci: back.mul_vec(s.v_eci),
                }
            }
        }
    }

    /// `t_s` 秒後の ECI 基準の平均要素。
    pub(super) fn mean_elements_at(&self, t_s: f64) -> Result<KeplerElements, AstroError> {
        match self.rot {
            None => Ok(self.elements_at(t_s)),
            Some(_) => osculating_elements(&self.state_at(t_s), self.mu),
        }
    }
}
