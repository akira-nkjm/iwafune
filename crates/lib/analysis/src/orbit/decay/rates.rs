//! 1 周で平均した抗力の効果（長半径の変化率・軌道を保つ ΔV の率・平均密度）。
//!
//! 長半径 `a`・RAAN を与えた 1 周を二体で標本化し（`sat_astro::orbit::OrbitSpec::states_at`）、各点で
//! 共回転大気に対する相対速度（`force::earth_relative_wind_eci`）・NRLMSISE-00 の密度（`atmosphere::earth_density_kg_m3`）・
//! 抗力の加速度（`force::drag_force` に `Cd·A/m`）を求める。Gauss の惑星方程式（エネルギーの式）
//! `da/dt = 2a²/μ · (v · a_D)` と、進行方向の抗力を打ち消す推力の大きさ `−v̂ · a_D` を周回平均
//! （`spec::orbit_mean`）する。離心率と近地点引数の変化は追わない（近円軌道の近似）。

use crate::spec::{AnalysisError, orbit_mean};
use sat_astro::atmosphere::{AtmosphereModel, earth_density_kg_m3};
use sat_astro::force::{drag_force, earth_relative_wind_eci};
use sat_astro::orbit::{OrbitElements, OrbitSpec, Propagator};
use sat_astro::timeline::sample_times;
use sat_units::typed::Qty;

/// 1 周の標本の数（等時間間隔。台形則の周回平均は周期関数で速く収束する）。
pub const ORBIT_SAMPLES: usize = 12;

/// 1 周で平均した抗力の効果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct MeanDrag {
    /// 周回平均の密度 [kg/m³]
    pub density: f64,
    /// 長半径の変化率 [m/s]（負が低下）
    pub a_dot: f64,
    /// 進行方向の抗力を打ち消す ΔV の率 [m/s²]（正）
    pub dv_rate: f64,
}

/// 経過 `t` [s]・長半径 `a` [m]・RAAN `raan` [rad] の 1 周の平均。`base` から離心率・傾斜角・近地点引数を取る。
/// `cd_a_over_m` は `Cd·A/m` [m²/kg]、`atmosphere` はその時刻の大気モデル。
pub(super) fn mean_drag(
    base: &OrbitSpec,
    t: f64,
    a: f64,
    raan: f64,
    cd_a_over_m: f64,
    atmosphere: &AtmosphereModel,
) -> Result<MeanDrag, AnalysisError> {
    let el = &base.elements;
    let elements = OrbitElements {
        semi_major_axis: Qty::from_raw(a),
        eccentricity: el.eccentricity,
        inclination: el.inclination,
        raan: Qty::from_raw(raan),
        arg_perigee: el.arg_perigee,
        mean_anomaly: Qty::ZERO,
    };
    let epoch = base.epoch.plus_seconds(t);
    let mut orbit = OrbitSpec::new(epoch, elements, Propagator::Kepler);
    orbit.central = base.central;
    let period = orbit.period_s();
    let times = sample_times(period, period / ORBIT_SAMPLES as f64)?;
    let states = orbit.states_at(&times)?;
    let mu = orbit.central.gm();
    let n = states.len();
    let (mut rho, mut a_dot, mut dv) = (
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
    );
    for (tau, s) in times.iter().zip(&states) {
        let density = earth_density_kg_m3(atmosphere, s.r_eci, epoch.plus_seconds(*tau));
        let acc = drag_force(
            density,
            cd_a_over_m,
            earth_relative_wind_eci(s.r_eci, s.v_eci),
        );
        rho.push(density);
        a_dot.push(2.0 * a * a / mu * s.v_eci.dot(acc));
        dv.push(s.v_eci.normalized().map_or(0.0, |u| -u.dot(acc)));
    }
    Ok(MeanDrag {
        density: orbit_mean(&times, &rho, &[]),
        a_dot: orbit_mean(&times, &a_dot, &[]),
        dv_rate: orbit_mean(&times, &dv, &[]),
    })
}
