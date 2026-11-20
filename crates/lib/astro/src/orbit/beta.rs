//! β 角（軌道面と太陽方向のなす角）と、β 角から元期の RAAN を逆算する関数。

use super::{OrbitSpec, Propagator, j2_secular_rates};
use crate::AstroError;
use crate::ephem::{CentralBody, sun_direction_from};
use crate::math::Vec3;
use crate::time::Epoch;
use core::f64::consts::TAU;

/// β 角 [rad]（軌道面と太陽方向のなす角）。`at` の時刻での RAAN（J2 ドリフト込み）を使う。
///
/// 正は軌道の角運動量の側に太陽があることを表す。数値伝搬と地球以外の中心天体では `at` の状態の
/// 角運動量から求める。太陽の方向は中心天体から見たもの。中心天体が太陽のときは太陽の方向が決まらないので
/// `InvalidInput`。
pub fn beta_angle_rad(spec: &OrbitSpec, at: Epoch) -> Result<f64, AstroError> {
    let sun = central_sun_direction(spec.central, at)?;
    let t = at.seconds_since(spec.epoch);
    let use_state = matches!(spec.propagator, Propagator::Numerical { .. })
        || spec.central != CentralBody::Earth;
    let normal = match use_state {
        true => spec.state_at(t)?.orbit_normal(),
        false => {
            let el = spec.mean_elements_at(t)?;
            let (si, ci) = el.i_rad.sin_cos();
            let (so, co) = el.raan_rad.sin_cos();
            Vec3::new(si * so, -si * co, ci)
        }
    };
    Ok(normal.dot(sun).clamp(-1.0, 1.0).asin())
}

/// 中心天体から見た太陽の向き。中心天体が太陽のときは β 角を定義できないのでエラー。
fn central_sun_direction(central: CentralBody, at: Epoch) -> Result<Vec3, AstroError> {
    sun_direction_from(central, at).ok_or_else(|| {
        AstroError::InvalidInput(
            "Beta angle is undefined when the Sun is the central body, because the Sun direction is not defined".into(),
        )
    })
}

/// 時刻 `at` の β 角が `beta_rad` になる、元期の RAAN [rad]（0〜2π）。
///
/// 太陽の赤経 α・赤緯 δ（`at` の時刻）に対して、[`beta_angle_rad`] と同じ符号の約束で
/// `sin β = cos δ · sin i · sin(Ω − α) + sin δ · cos i`。解は 2 つあり、**`Ω − α` が −90°〜+90° の側**
/// （昇交点が昼側。β = 0 なら昇交点の地方時が正午、β が正で大きいほど 18 時側）を返す。
/// 元期から `at` までの RAAN の変化は J2 永年項で戻す（Kepler 伝搬では変化なし。数値伝搬では近似）。
/// 地球以外の中心天体では RAAN の変化を戻さない（自転軸が ECI の z と違うため。`at` を元期にすれば厳密）。
/// 傾斜角から届かない β（`|sin β − sin δ cos i| > cos δ sin i`）と、中心天体が太陽のときは `InvalidInput`。
pub fn raan_for_beta(spec: &OrbitSpec, at: Epoch, beta_rad: f64) -> Result<f64, AstroError> {
    let el = spec.kepler_elements()?;
    if !beta_rad.is_finite() {
        return Err(AstroError::InvalidInput("Beta angle is not finite".into()));
    }
    let s = central_sun_direction(spec.central, at)?;
    let (alpha, sin_dec) = (s.y.atan2(s.x), s.z.clamp(-1.0, 1.0));
    let cos_dec = (1.0 - sin_dec * sin_dec).sqrt();
    let (si, ci) = el.i_rad.sin_cos();
    let denom = cos_dec * si;
    let k = (beta_rad.sin() - sin_dec * ci) / denom;
    if denom.abs() < 1e-12 || !k.is_finite() || k.abs() > 1.0 {
        return Err(AstroError::InvalidInput(format!(
            "Beta = {:.3} deg is unreachable at inclination {:.3} deg",
            beta_rad.to_degrees(),
            el.i_rad.to_degrees()
        )));
    }
    let raan_at = alpha + k.asin();
    let dt = at.seconds_since(spec.epoch);
    let drift = match spec.propagator {
        _ if spec.central != CentralBody::Earth => 0.0,
        Propagator::Kepler => 0.0,
        _ => j2_secular_rates(el.a_m, el.e, el.i_rad).0 * dt,
    };
    Ok((raan_at - drift).rem_euclid(TAU))
}
