//! 状態ベクトルの軌道: 元期・原点の天体・位置と速度・力の設定・期間。要素で書けない軌道（惑星間の巡航、
//! ラグランジュ点のまわり、2 天体の重力が同程度の領域）を、**既存の数値伝搬**（[`super::propagate_states`]。
//! 力モデルは [`crate::force`]、積分器は [`crate::ode::Dop853`]）でそのまま伝搬する。ここには伝搬も力の式も無く、
//! 「どの力・どの積分器で、どこまでの表を作るか」の既定だけがある。
//!
//! # 座標系
//! 位置・速度は**原点の天体の中心**からの相対で、向きは ECI（J2000 平均赤道の慣性系。[`OrbitState`] の `r_eci`・`v_eci` と同じ。
//! 黄道系ではない）。元期は時刻 0 で、期間 `span_s` は [`Cycle::OneOff`](super::Cycle) の長さになる。
//!
//! # 既定
//! - 力: 書かなければ [`ForceConfig::deep_space_defaults`]（原点の天体で決まる。ここで決めるのではなく力モデルの側が持つ）。
//! - 積分器: 太陽が原点なら [`HELIOCENTRIC_INTEGRATOR`]（刻みの上限 1 日）、それ以外は [`NUMERICAL_INTEGRATOR`]（600 s）。
//! - 任意の時刻の表（[`StateSpec::table_step_s`]）: 近点での 1 周の時間の [`TABLE_STEPS_PER_ORBIT`] 分の 1。

use super::numerical::{HELIOCENTRIC_INTEGRATOR, NUMERICAL_INTEGRATOR, propagate_states};
use super::track::TABLE_STEPS_PER_ORBIT;
use crate::AstroError;
use crate::ephem::CentralBody;
use crate::force::ForceConfig;
use crate::ode::Dop853;
use crate::state::OrbitState;
use crate::time::Epoch;
use core::f64::consts::TAU;
use serde::{Deserialize, Serialize};

/// 状態ベクトルで書かれた軌道（[`super::Trajectory::State`]）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StateSpec {
    /// 元期（時刻 0。状態はこの時刻のもの）
    pub epoch: Epoch,
    /// 原点の天体（位置・速度の基準。直下点・日陰・遮りの基準でもある）
    #[serde(default)]
    pub central: CentralBody,
    /// 元期の位置 [m]・速度 [m/s]（原点の天体の中心から。向きは ECI）
    pub state: OrbitState,
    /// 力の設定
    pub forces: ForceConfig,
    /// 1 回限りの期間 [s]（[`Cycle::OneOff`](super::Cycle) の長さ。解析が評価する範囲）
    pub span_s: f64,
}

impl StateSpec {
    /// 力の設定を書かなければ慣例の既定（[`ForceConfig::deep_space_defaults`]）で作る。値の検査は [`StateSpec::validate`]。
    pub fn with_defaults(
        epoch: Epoch,
        central: CentralBody,
        state: OrbitState,
        span_s: f64,
        forces: Option<ForceConfig>,
    ) -> Self {
        Self {
            epoch,
            central,
            state,
            forces: forces.unwrap_or_else(|| ForceConfig::deep_space_defaults(central)),
            span_s,
        }
    }

    /// 値を検査する（期間は正の有限、位置・速度は有限で位置は 0 でない、力の設定は原点の天体で使えるもの）。
    pub fn validate(&self) -> Result<(), AstroError> {
        let (r, v) = (self.state.r_eci, self.state.v_eci);
        if !(self.span_s.is_finite() && self.span_s > 0.0) {
            return Err(AstroError::InvalidInput(format!(
                "The span of a state orbit must be positive ({} s)",
                self.span_s
            )));
        }
        let finite = [r.x, r.y, r.z, v.x, v.y, v.z].iter().all(|c| c.is_finite());
        if !finite || r.norm() == 0.0 {
            return Err(AstroError::InvalidInput(
                "The position and velocity of a state orbit must be finite and the position must not be zero".into(),
            ));
        }
        crate::force::validate_about(self.central, &self.forces)
    }

    /// 伝搬に使う積分器（原点が太陽なら刻みの上限 1 日）。
    pub fn integrator(&self) -> Dop853 {
        match self.central {
            CentralBody::Sun => HELIOCENTRIC_INTEGRATOR,
            _ => NUMERICAL_INTEGRATOR,
        }
    }

    /// 元期からの秒 `times_s`（昇順・非負）での状態。1 回の積分の密出力で返す（時刻ごとに積分し直さない）。
    pub fn states_at(&self, times_s: &[f64]) -> Result<Vec<OrbitState>, AstroError> {
        self.validate()?;
        propagate_states(
            self.central,
            &self.forces,
            self.epoch,
            self.state,
            times_s,
            &self.integrator(),
        )
    }

    /// 任意の時刻の表（[`super::OrbitTrack`]）の格子の刻み [s]: 元期の状態の接触軌道の、近点での 1 周の時間
    /// `2π q² / h`（`q` は近点距離、`h` は比角運動量。円軌道では周期）の [`TABLE_STEPS_PER_ORBIT`] 分の 1。
    /// 位置と速度からの 3 次のエルミート補間の相対誤差は、この刻みの円軌道で `(2π/100)⁴/384 ≈ 4e-8`。
    /// 角運動量が 0（直線の軌道）のときは、元期の距離での円軌道の周期を使う。
    pub fn table_step_s(&self) -> f64 {
        let mu = self.central.gm();
        let (r, v) = (self.state.r_eci, self.state.v_eci);
        let h_vec = r.cross(v);
        let h = h_vec.norm();
        let e = (v.cross(h_vec) / mu - r / r.norm()).norm();
        let q = h * h / (mu * (1.0 + e));
        let period = if h > 0.0 && q.is_finite() {
            TAU * q * q / h
        } else {
            TAU * (r.norm().powi(3) / mu).sqrt()
        };
        period / TABLE_STEPS_PER_ORBIT
    }
}

#[cfg(test)]
mod tests;
