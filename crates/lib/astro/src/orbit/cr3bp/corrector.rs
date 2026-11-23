//! xz 平面に対称な周期軌道の補正（単一シューティング）。
//!
//! 初期状態 `(x0, 0, z0, 0, vy0, 0)`（xz 平面を垂直に横切る点）から、次に xz 平面を横切る点まで積分し
//! （[`Dop853::integrate_segment`] のイベント）、そこで `vx = vz = 0`（垂直に横切る）ようにする。
//! 対称性から、半周期で垂直に横切れば全周期で閉じる。補正は状態遷移行列で線形化した 2×2 の連立一次方程式
//! （[`crate::math::solve_dense`]）を Newton 法で解く。横切る時刻の変化は `δt = −δy_f / vy_f` で入れる。

use super::dynamics::{State6, State42, initial_42, stm_of};
use super::{Cr3bp, Cr3bpError};
use crate::math::solve_dense;
use crate::ode::Dop853;

/// 初期状態の位置成分のうち固定するもの。残りの成分と `vy0` を補正する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fixed {
    /// `z0` を固定（x0 と vy0 を補正。ハロー軌道の族の追跡の普通のやり方）
    Z0,
    /// `x0` を固定（z0 と vy0 を補正。z0 = 0 から始めれば面内のリアプノフ軌道）
    X0,
}

/// 補正の初期値。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeriodicGuess {
    /// xz 平面を横切る点の状態（`y = 0`、`vx = vz = 0`、`vy ≠ 0`）
    pub state: State6,
    /// 周期の目安（正規化）。最初の横切りを探す開始位置の決定にだけ使う
    pub period: f64,
}

/// 補正の設定。
#[derive(Debug, Clone, Copy)]
pub struct CorrectorConfig {
    /// どの成分を固定するか
    pub fixed: Fixed,
    /// 反復の上限
    pub max_iterations: usize,
    /// 収束の判定（半周期の点での `max(|vx|, |vz|)`、正規化）
    pub tolerance: f64,
    /// 積分器の設定
    pub integrator: Dop853,
}

impl Default for CorrectorConfig {
    fn default() -> Self {
        Self {
            fixed: Fixed::Z0,
            max_iterations: 30,
            tolerance: 1e-11,
            integrator: Dop853 {
                rtol: 1e-13,
                atol: 1e-13,
                ..Dop853::default()
            },
        }
    }
}

/// 補正した周期軌道。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeriodicOrbit {
    /// 質量比
    pub mu: f64,
    /// xz 平面を横切る点の状態（正規化、回転系）
    pub state: State6,
    /// 周期（正規化。時間の単位は 1/n）
    pub period: f64,
    /// ヤコビ定数
    pub jacobi: f64,
    /// 反復の回数（初期値がすでに収束していれば 0）
    pub iterations: usize,
    /// 最後に測った半周期の点での `max(|vx|, |vz|)`
    pub residual: f64,
}

/// 半周期の点での積分の結果。
struct Crossing {
    t: f64,
    s: State42,
}

impl Cr3bp {
    /// xz 平面の次の横切りまで状態と状態遷移行列を積分する。
    fn next_crossing(
        &self,
        x0: State6,
        skip: f64,
        limit: f64,
        cfg: &CorrectorConfig,
    ) -> Result<Crossing, Cr3bpError> {
        let side = x0[4].signum();
        // 出発点は y = 0 なので、`skip` の間はイベントを無効にして離れてから、y の符号が戻る点を探す。
        let event = move |t: f64, s: &State42| if t < skip { 1.0 } else { side * s[1] };
        let seg = cfg.integrator.integrate_segment(
            |_, s: &State42| self.derivative_with_stm(s),
            0.0,
            initial_42(x0),
            limit,
            None,
            event,
        )?;
        if !seg.event {
            return Err(Cr3bpError::NoCrossing { limit });
        }
        Ok(Crossing { t: seg.t, s: seg.y })
    }

    /// 周期軌道を補正する（[`PeriodicGuess`] から出発して、半周期の点で垂直に横切るまで Newton 法）。
    pub fn correct_periodic(
        &self,
        guess: &PeriodicGuess,
        cfg: &CorrectorConfig,
    ) -> Result<PeriodicOrbit, Cr3bpError> {
        let free = match cfg.fixed {
            Fixed::Z0 => 0,
            Fixed::X0 => 2,
        };
        if guess.state[4] == 0.0 || !guess.state[4].is_finite() {
            return Err(Cr3bpError::InvalidArgument(
                "vy0 must be non-zero (it sets the direction of departure from the xz-plane)"
                    .to_string(),
            ));
        }
        let skip = 0.02 * guess.period;
        let limit = 2.0 * guess.period.max(1.0) + 10.0;
        let mut x0 = guess.state;
        let mut residual = f64::INFINITY;
        for iteration in 0..=cfg.max_iterations {
            let c = self.next_crossing(x0, skip, limit, cfg)?;
            let f = &c.s;
            residual = f[3].abs().max(f[5].abs());
            if residual <= cfg.tolerance {
                return Ok(PeriodicOrbit {
                    mu: self.mu(),
                    state: x0,
                    period: 2.0 * c.t,
                    jacobi: self.jacobi(&x0),
                    iterations: iteration,
                    residual,
                });
            }
            let step = self
                .newton_step(&c, free)
                .ok_or(Cr3bpError::SingularCorrection { iteration })?;
            x0[free] += step[0];
            x0[4] += step[1];
        }
        Err(Cr3bpError::NotConverged {
            iterations: cfg.max_iterations,
            residual,
        })
    }

    /// 半周期の点の `(vx, vz)` を 0 にする `(δ(自由な位置成分), δvy0)`。
    fn newton_step(&self, c: &Crossing, free: usize) -> Option<[f64; 2]> {
        let phi = stm_of(&c.s);
        let end: State6 = [c.s[0], c.s[1], c.s[2], c.s[3], c.s[4], c.s[5]];
        let acc = self.derivative(&end);
        let vy = end[4];
        // 横切る時刻の変化を入れた、(vx, vz) の自由変数に対する感度
        let sens = |row: usize, col: usize, a: f64| phi[row][col] - a * phi[1][col] / vy;
        let m = vec![
            sens(3, free, acc[3]),
            sens(3, 4, acc[3]),
            sens(5, free, acc[5]),
            sens(5, 4, acc[5]),
        ];
        let d = solve_dense(m, vec![-end[3], -end[5]])?;
        Some([d[0], d[1]])
    }

    /// 周期軌道の 1 周期後の状態遷移行列（モノドロミー行列）。
    pub fn monodromy(
        &self,
        orbit: &PeriodicOrbit,
        integrator: &Dop853,
    ) -> Result<[[f64; 6]; 6], Cr3bpError> {
        Ok(self.flow_with_stm(orbit.state, orbit.period, integrator)?.1)
    }
}
