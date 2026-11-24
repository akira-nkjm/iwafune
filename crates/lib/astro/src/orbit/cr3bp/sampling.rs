//! 周期軌道の 1 周期を回転系の表にする（[`crate::orbit::OrbitTrack`] と同じ 3 次のエルミート補間に渡す形）。

use super::corrector::PeriodicOrbit;
use super::dynamics::State6;
use super::{Cr3bp, Cr3bpError};
use crate::math::Vec3;
use crate::ode::Dop853;
use crate::state::OrbitState;

/// 1 周期を等間隔に標本した周期軌道（正規化、回転系）。最後の点は最初の点と同じ状態（周期の終わり）。
#[derive(Debug, Clone, PartialEq)]
pub struct SampledOrbit {
    /// 質量比
    pub mu: f64,
    /// 周期（正規化）
    pub period: f64,
    /// ヤコビ定数
    pub jacobi: f64,
    /// 時刻（0 から周期まで、正規化）
    pub times: Vec<f64>,
    /// 各時刻の状態 `[x, y, z, vx, vy, vz]`
    pub states: Vec<State6>,
}

impl SampledOrbit {
    /// 位置と速度の表（位置は [`OrbitState::r_eci`]、速度は [`OrbitState::v_eci`] に入れる。
    /// 入っているのは回転系の値で、`orbit::OrbitTrack` の表と同じ補間に渡せる）。
    pub fn as_table(&self) -> (Vec<f64>, Vec<OrbitState>) {
        let s = self
            .states
            .iter()
            .map(|s| OrbitState {
                r_eci: Vec3::new(s[0], s[1], s[2]),
                v_eci: Vec3::new(s[3], s[4], s[5]),
            })
            .collect();
        (self.times.clone(), s)
    }

    /// z の最大の絶対値（正規化）。
    pub fn z_amplitude(&self) -> f64 {
        self.states.iter().fold(0.0, |m, s| m.max(s[2].abs()))
    }
}

impl Cr3bp {
    /// 周期軌道の 1 周期を `intervals` 個の等間隔の区間（`intervals + 1` 個の点）の表にする。
    pub fn sample_orbit(
        &self,
        orbit: &PeriodicOrbit,
        intervals: usize,
        integrator: &Dop853,
    ) -> Result<SampledOrbit, Cr3bpError> {
        if intervals < 2 {
            return Err(Cr3bpError::InvalidArgument(
                "at least 2 intervals are needed".to_string(),
            ));
        }
        let times: Vec<f64> = (0..=intervals)
            .map(|i| orbit.period * i as f64 / intervals as f64)
            .collect();
        let states = self.flow(orbit.state, &times, integrator)?;
        Ok(SampledOrbit {
            mu: self.mu(),
            period: orbit.period,
            jacobi: orbit.jacobi,
            times,
            states,
        })
    }
}
