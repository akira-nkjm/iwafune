//! 接触要素から平均の長半径を求める（J2 の短周期項を 1 次で引く）。推力を入れた伝搬が「目標の高度に着いたか」を
//! 判定するのに使う（接触の長半径は LEO で J2 により ±10 km ほど振れる）。
//!
//! Brouwer の 1 次の短周期項（Vallado, *Fundamentals of Astrodynamics and Applications*, 4th ed., §9.6 の
//! 平均 ⇄ 接触の換算）: `a_sp = (J2 R²/a) [ (3cos²i − 1)/2 · ((a/r)³ − (1 − e²)^(−3/2)) + (3/2) sin²i (a/r)³ cos 2u ]`、
//! `a_平均 = a_接触 − a_sp`。右辺は接触要素で評価する（誤差は J2² の程度）。傾斜角は ECI の赤道に対して測る。

use super::{
    KeplerElements, OrbitState, elements_to_state, mean_motion_rad_s, osculating_elements,
};
use crate::AstroError;
use crate::ephem::CentralBody;
use crate::force::{ForceConfig, GravityModel};
use crate::math::Vec3;
use crate::time::Epoch;
use core::f64::consts::TAU;

/// 周回平均の要素を求めるときの 1 周の標本の数。
const AVERAGE_SAMPLES: usize = 72;

/// 平均の長半径 [m]。`with_j2` が偽（点質量の重力）なら接触の長半径のまま。
pub fn mean_semi_major_axis_m(
    central: CentralBody,
    state: &OrbitState,
    with_j2: bool,
) -> Result<f64, AstroError> {
    let el = osculating_elements(state, central.gm())?;
    if !with_j2 {
        return Ok(el.a_m);
    }
    let r = state.r_eci.norm();
    let ar3 = (el.a_m / r).powi(3);
    let (si, ci) = el.i_rad.sin_cos();
    let eta3 = (1.0 - el.e * el.e).powf(-1.5);
    let k = central.j2() * central.radius_m().powi(2) / el.a_m;
    let cos2u = cos_two_u(state);
    let sp = k * ((3.0 * ci * ci - 1.0) / 2.0 * (ar3 - eta3) + 1.5 * si * si * ar3 * cos2u);
    Ok(el.a_m - sp)
}

/// 平均要素（[`super::Propagator::J2`] の要素の意味）→ 接触の位置・速度。長半径だけ J2 の短周期項を足す
/// （[`mean_semi_major_axis_m`] の逆。ほかの要素の短周期項は小さいので平均のまま）。推力を入れた伝搬の始めの状態に使う。
pub fn osculating_state_from_mean(
    central: CentralBody,
    mean: &KeplerElements,
) -> Result<OrbitState, AstroError> {
    let mu = central.gm();
    let mut el = *mean;
    let mut state = elements_to_state(&el, mu);
    for _ in 0..3 {
        let a_mean = mean_semi_major_axis_m(central, &state, true)?;
        el.a_m += mean.a_m - a_mean;
        state = elements_to_state(&el, mu);
    }
    Ok(state)
}

/// 状態 `state`（元期 `epoch`）の近似の平均要素: 長半径は [`mean_semi_major_axis_m`]、離心率と近点引数は
/// 力 `forces` で 1 周だけ伝搬した接触の離心率ベクトルの平均（J2 の短周期項は 1 周で平均すると消える）、
/// 傾斜角・RAAN は接触のまま、平均近点角は位置の昇交点からの引数 − 近点引数（離心率が小さい近似）。
/// 推力を入れた伝搬の終わりの軌道を、J2 の平均要素の軌道（[`super::Propagator::J2`]）として続けるのに使う。
pub fn orbit_averaged_elements(
    central: CentralBody,
    forces: &ForceConfig,
    epoch: Epoch,
    state: &OrbitState,
) -> Result<KeplerElements, AstroError> {
    let mu = central.gm();
    let osc = osculating_elements(state, mu)?;
    let with_j2 = forces.gravity != GravityModel::PointMass;
    let a = mean_semi_major_axis_m(central, state, with_j2)?;
    let period = TAU / mean_motion_rad_s(mu, a);
    let times: Vec<f64> = (0..AVERAGE_SAMPLES)
        .map(|k| period * k as f64 / AVERAGE_SAMPLES as f64)
        .collect();
    let states = super::numerical::states_at(central, forces, epoch, *state, &times)?;
    let mut e_vec = Vec3::ZERO;
    for s in &states {
        let h = s.r_eci.cross(s.v_eci);
        e_vec += s.v_eci.cross(h) / mu - s.r_eci / s.r_eci.norm();
    }
    e_vec = e_vec / AVERAGE_SAMPLES as f64;
    let h = state.orbit_normal();
    let node = Vec3::Z.cross(h).normalized().unwrap_or(Vec3::X);
    let in_plane = |x: Vec3| h.dot(node.cross(x)).atan2(node.dot(x));
    let e = e_vec.norm();
    let argp = if e > 1e-12 {
        in_plane(e_vec).rem_euclid(TAU)
    } else {
        0.0
    };
    let u = in_plane(state.r_eci);
    Ok(KeplerElements {
        a_m: a,
        e,
        argp_rad: argp,
        mean_anomaly_rad: (u - argp).rem_euclid(TAU),
        ..osc
    })
}

/// `cos 2u`（`u` は昇交点からの引数）。
fn cos_two_u(state: &OrbitState) -> f64 {
    let h = state.orbit_normal();
    let node = Vec3::Z.cross(h).normalized().unwrap_or(Vec3::X);
    let r = state.r_eci;
    let (x, y) = (r.dot(node), r.dot(h.cross(node)));
    let n2 = x * x + y * y;
    if n2 > 0.0 { (x * x - y * y) / n2 } else { 1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::force::{ForceConfig, GravityModel};
    use crate::orbit::{OrbitElements, OrbitSpec, Propagator};
    use crate::time::Epoch;
    use sat_units::typed::Qty;

    /// J2 だけの数値伝搬で、接触の長半径は ±9 km 振れるが、平均の長半径はほぼ一定（1 周で 100 m 以内）。
    #[test]
    fn mean_semi_major_axis_is_steady_under_j2() {
        let a = 6_928_137.0;
        let spec = OrbitSpec::new(
            Epoch { j2000_s: 0.0 },
            OrbitElements::with_defaults(
                Qty::from_raw(a),
                Qty::from_raw(97.6_f64.to_radians()),
                None,
                None,
                None,
                None,
            ),
            Propagator::Numerical {
                forces: ForceConfig::gravity_only(GravityModel::J2),
            },
        );
        let times: Vec<f64> = (0..=60).map(|k| k as f64 * 100.0).collect();
        let states = spec.states_at(&times).unwrap();
        let osc: Vec<f64> = states
            .iter()
            .map(|s| mean_semi_major_axis_m(CentralBody::Earth, s, false).unwrap())
            .collect();
        let mean: Vec<f64> = states
            .iter()
            .map(|s| mean_semi_major_axis_m(CentralBody::Earth, s, true).unwrap())
            .collect();
        let spread = |v: &[f64]| {
            v.iter().copied().fold(f64::MIN, f64::max) - v.iter().copied().fold(f64::MAX, f64::min)
        };
        assert!(spread(&osc) > 15_000.0, "{}", spread(&osc));
        assert!(spread(&mean) < 100.0, "{}", spread(&mean));
        // 逆向き: 平均要素から作った接触の状態の平均の長半径は元の値
        let el = spec.kepler_elements().unwrap();
        let x = osculating_state_from_mean(CentralBody::Earth, &el).unwrap();
        let back = mean_semi_major_axis_m(CentralBody::Earth, &x, true).unwrap();
        assert!((back - a).abs() < 1e-3, "{back}");
        // 接触の離心率は J2 で周回の中で振れるが、周回平均は周回のどこから求めてもほぼ同じ（1e-5 以内）
        let forces = ForceConfig::gravity_only(GravityModel::J2);
        let e_at = |k: usize| {
            osculating_elements(&states[k], CentralBody::Earth.gm())
                .unwrap()
                .e
        };
        let avg = |k: usize| {
            let t = spec.epoch.plus_seconds(times[k]);
            orbit_averaged_elements(CentralBody::Earth, &forces, t, &states[k]).unwrap()
        };
        let osc: Vec<f64> = (0..states.len()).map(e_at).collect();
        assert!(spread(&osc) > 4e-4, "{}", spread(&osc));
        let (a10, a40) = (avg(10), avg(40));
        assert!((a10.e - a40.e).abs() < 1e-5, "{} vs {}", a10.e, a40.e);
        assert!((a10.a_m - mean[10]).abs() < 1e-6);
    }
}
