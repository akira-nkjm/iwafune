//! β 角（軌道面と太陽方向のなす角）と、β 角から元期の RAAN を逆算する関数。

use super::{OrbitSpec, Propagator, j2_secular_rates};
use crate::AstroError;
use crate::ephem::{CentralBody, sun_direction_from};
use crate::math::Vec3;
use crate::time::Epoch;
use core::f64::consts::TAU;

/// β 角 [rad]（軌道面と太陽方向のなす角）。`at` の時刻での RAAN（J2 ドリフト込み）を使う。
///
/// 正は軌道の角運動量の側に太陽があることを表す。数値伝播と地球以外の中心天体では `at` の状態の
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

/// `at` の時刻に、傾斜角 `i_rad` の軌道が取れる β 角の範囲 `(最小, 最大)` [rad]。
/// `sin β = cos δ · sin i · sin(Ω − α) + sin δ · cos i`（δ は太陽の赤緯）の `sin(Ω − α)` が −1〜+1 を動くので、
/// `sin β` の範囲は `sin δ · cos i ∓ cos δ · sin i`（= `sin(δ ∓ i)`）。**取れる β の範囲の定義はここだけ**（[`raan_for_beta`] の
/// 誤りの文もこれを使う）。
fn beta_range_rad(i_rad: f64, sin_dec: f64) -> (f64, f64) {
    let cos_dec = (1.0 - sin_dec * sin_dec).sqrt();
    let (si, ci) = i_rad.sin_cos();
    let centre = sin_dec * ci;
    let half = cos_dec * si.abs();
    (
        (centre - half).clamp(-1.0, 1.0).asin(),
        (centre + half).clamp(-1.0, 1.0).asin(),
    )
}

/// 角度（rad）を表示の単位で整形した文字列（整形は sat-units に任せる）。
fn angle_text(rad: f64) -> String {
    use sat_units::format::{SIGNIFICANT_DIGITS, format_number, format_quantity};
    format_quantity(rad, "Angle")
        .map(|q| q.text)
        .unwrap_or_else(|_| format_number(rad, SIGNIFICANT_DIGITS))
}

/// 届かない β の誤りの文（その元期・傾斜角で取れる範囲と、直し方を添える）。
fn unreachable_message(beta_rad: f64, i_rad: f64, sin_dec: f64) -> String {
    let (lo, hi) = beta_range_rad(i_rad, sin_dec);
    format!(
        "Beta = {} is unreachable at inclination {}: at this epoch (Sun declination {}) the beta angle of this orbit can only be from {} to {}. \
         Give a beta inside that range, or change the epoch (an analysis case's `epoch`) or the inclination of the orbit",
        angle_text(beta_rad),
        angle_text(i_rad),
        angle_text(sin_dec.clamp(-1.0, 1.0).asin()),
        angle_text(lo),
        angle_text(hi),
    )
}

/// 時刻 `at` の β 角が `beta_rad` になる、元期の RAAN [rad]（0〜2π）。
///
/// 太陽の赤経 α・赤緯 δ（`at` の時刻）に対して、[`beta_angle_rad`] と同じ符号の約束で
/// `sin β = cos δ · sin i · sin(Ω − α) + sin δ · cos i`。解は 2 つあり、**`Ω − α` が −90°〜+90° の側**
/// （昇交点が昼側。β = 0 なら昇交点の地方時が正午、β が正で大きいほど 18 時側）を返す。
/// 元期から `at` までの RAAN の変化は J2 永年項で戻す（Kepler 伝播では変化なし。数値伝播では近似）。
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
        return Err(AstroError::InvalidInput(unreachable_message(
            beta_rad, el.i_rad, sin_dec,
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
