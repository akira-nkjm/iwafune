//! 時系列の計算（擾乱トルク → ECI での角運動量 → 機体座標系 → ホイールへの配分）と、永年・周期成分の分離。

use crate::aocs::disturbance::{DisturbanceSpacecraft, DisturbanceTorques};
use crate::aocs::wheels::WheelSet;
use crate::spec::orbit_segments;
use sat_astro::ephem::CentralBody;
use sat_astro::math::Vec3;
use sat_astro::ode::rk4_linear_step;
use sat_astro::timeline::TimelineSample;
use std::ops::Range;

/// 時系列の計算結果（値は SI の生の f64）。
pub(super) struct Trace {
    pub(super) t: Vec<f64>,
    pub(super) torques: Vec<DisturbanceTorques>,
    pub(super) h_eci: Vec<Vec3>,
    pub(super) h_body: Vec<Vec3>,
    /// `wheel_h[i][k]`: ホイール `i` のサンプル `k` での角運動量 [N·m·s]
    pub(super) wheel_h: Vec<Vec<f64>>,
    /// `wheel_tau[i][k]`: ホイール `i` が出すべきトルク [N·m]
    pub(super) wheel_tau: Vec<Vec<f64>>,
}

impl Trace {
    pub(super) fn build(
        sc: &DisturbanceSpacecraft,
        central: CentralBody,
        set: &WheelSet,
        samples: &[TimelineSample],
    ) -> Self {
        let t: Vec<f64> = samples.iter().map(|s| s.t_s).collect();
        let torques: Vec<DisturbanceTorques> =
            samples.iter().map(|s| sc.torques(central, s)).collect();
        let tau_eci: Vec<Vec3> = samples
            .iter()
            .zip(&torques)
            .map(|(s, d)| s.to_eci(d.total()))
            .collect();
        let h_eci = integrate_momentum(&t, &tau_eci);
        let h_body: Vec<Vec3> = samples
            .iter()
            .zip(&h_eci)
            .map(|(s, &h)| s.to_body(h))
            .collect();
        let n = set.distribute(Vec3::ZERO).len();
        let mut wheel_h = vec![Vec::with_capacity(t.len()); n];
        let mut wheel_tau = vec![Vec::with_capacity(t.len()); n];
        for (h, d) in h_body.iter().zip(&torques) {
            for (i, v) in set.distribute(*h).into_iter().enumerate() {
                wheel_h[i].push(v);
            }
            for (i, v) in set.distribute(d.total()).into_iter().enumerate() {
                wheel_tau[i].push(v);
            }
        }
        Self {
            t,
            torques,
            h_eci,
            h_body,
            wheel_h,
            wheel_tau,
        }
    }
}

/// ECI のトルク [N·m] を積分した角運動量 [N·m·s]（`H(0) = 0`）。刻みの間は線形補間。
pub(super) fn integrate_momentum(t: &[f64], tau_eci: &[Vec3]) -> Vec<Vec3> {
    let mut h = Vec::with_capacity(t.len());
    let mut x = [0.0; 3];
    for k in 0..t.len() {
        h.push(Vec3::new(x[0], x[1], x[2]));
        if k + 1 == t.len() {
            break;
        }
        let (t0, dt, a, b) = (t[k], t[k + 1] - t[k], tau_eci[k], tau_eci[k + 1]);
        x = rk4_linear_step(|w, _: &[f64; 3]| (a + (b - a) * w).to_array(), t0, &x, dt);
    }
    h
}

/// 1 周あたりの永年成分（ECI）と、周期成分の振幅。周回が無ければ全期間を 1 区間として 1 周分に換算する。
pub(super) fn secular_and_cyclic(
    t: &[f64],
    h: &[Vec3],
    ranges: &[Range<usize>],
    period_s: f64,
) -> (Vec3, f64) {
    if t.len() < 2 {
        return (Vec3::ZERO, 0.0);
    }
    let segments = orbit_segments(t.len(), ranges);
    let mut secular = Vec3::ZERO;
    let mut amplitude = 0.0_f64;
    for seg in &segments {
        let (s, e) = (*seg.start(), *seg.end());
        let duration = t[e] - t[s];
        if duration <= 0.0 {
            continue;
        }
        let dh = h[e] - h[s];
        secular += dh * (period_s / duration);
        for k in s..=e {
            let dev = h[k] - h[s] - dh * ((t[k] - t[s]) / duration);
            amplitude = amplitude.max(dev.norm());
        }
    }
    (secular / segments.len() as f64, amplitude)
}
