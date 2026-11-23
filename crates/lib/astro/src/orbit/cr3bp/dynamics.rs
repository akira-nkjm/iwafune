//! 回転系の運動方程式・有効ポテンシャル・ヤコビ定数・変分方程式（状態遷移行列）・流れ。

use super::{Cr3bp, Cr3bpError};
use crate::ode::Dop853;

/// 状態 `[x, y, z, vx, vy, vz]`（正規化、回転系）。
pub type State6 = [f64; 6];
/// 状態 6 + 状態遷移行列 36（行優先、`Φ[i][j] = s[6 + 6i + j]`）。
pub type State42 = [f64; 42];

/// 主天体・副天体までの距離 `(r1, r2)`。
fn distances(mu: f64, p: [f64; 3]) -> (f64, f64) {
    let r1 = ((p[0] + mu).powi(2) + p[1] * p[1] + p[2] * p[2]).sqrt();
    let r2 = ((p[0] - 1.0 + mu).powi(2) + p[1] * p[1] + p[2] * p[2]).sqrt();
    (r1, r2)
}

impl Cr3bp {
    /// 有効ポテンシャル Ω = (x²+y²)/2 + (1−μ)/r₁ + μ/r₂（正の符号の規約。定数項なし）。
    pub fn effective_potential(&self, p: [f64; 3]) -> f64 {
        let (r1, r2) = distances(self.mu, p);
        0.5 * (p[0] * p[0] + p[1] * p[1]) + (1.0 - self.mu) / r1 + self.mu / r2
    }

    /// Ω の勾配 `[Ωx, Ωy, Ωz]`。
    pub fn potential_gradient(&self, p: [f64; 3]) -> [f64; 3] {
        let mu = self.mu;
        let (r1, r2) = distances(mu, p);
        let (k1, k2) = ((1.0 - mu) / r1.powi(3), mu / r2.powi(3));
        [
            p[0] - k1 * (p[0] + mu) - k2 * (p[0] - 1.0 + mu),
            p[1] - (k1 + k2) * p[1],
            -(k1 + k2) * p[2],
        ]
    }

    /// Ω のヘッセ行列（対称、3×3）。
    pub fn potential_hessian(&self, p: [f64; 3]) -> [[f64; 3]; 3] {
        let mu = self.mu;
        let (r1, r2) = distances(mu, p);
        let (k1, k2) = ((1.0 - mu) / r1.powi(3), mu / r2.powi(3));
        let (m1, m2) = (3.0 * (1.0 - mu) / r1.powi(5), 3.0 * mu / r2.powi(5));
        let d = [[p[0] + mu, p[1], p[2]], [p[0] - 1.0 + mu, p[1], p[2]]];
        let mut h = [[0.0; 3]; 3];
        for (i, row) in h.iter_mut().enumerate() {
            for (j, v) in row.iter_mut().enumerate() {
                *v = m1 * d[0][i] * d[0][j] + m2 * d[1][i] * d[1][j];
                if i == j {
                    *v -= k1 + k2;
                    if i < 2 {
                        *v += 1.0;
                    }
                }
            }
        }
        h
    }

    /// ヤコビ定数 C = 2Ω − v²。
    pub fn jacobi(&self, s: &State6) -> f64 {
        let v2 = s[3] * s[3] + s[4] * s[4] + s[5] * s[5];
        2.0 * self.effective_potential([s[0], s[1], s[2]]) - v2
    }

    /// 運動方程式の右辺 `[v, a]`（回転系）。
    pub fn derivative(&self, s: &State6) -> State6 {
        let g = self.potential_gradient([s[0], s[1], s[2]]);
        [
            s[3],
            s[4],
            s[5],
            2.0 * s[4] + g[0],
            -2.0 * s[3] + g[1],
            g[2],
        ]
    }

    /// 状態 + 状態遷移行列の右辺（Φ' = A Φ、A = [[0, I], [Ωpp, 2K]]、K = [[0,1,0],[−1,0,0],[0,0,0]]）。
    pub fn derivative_with_stm(&self, s: &State42) -> State42 {
        let mut out = [0.0; 42];
        let x: State6 = [s[0], s[1], s[2], s[3], s[4], s[5]];
        out[..6].copy_from_slice(&self.derivative(&x));
        let h = self.potential_hessian([x[0], x[1], x[2]]);
        let mut a = [[0.0; 6]; 6];
        for i in 0..3 {
            a[i][3 + i] = 1.0;
            a[3 + i][..3].copy_from_slice(&h[i]);
        }
        a[3][4] = 2.0;
        a[4][3] = -2.0;
        for i in 0..6 {
            for j in 0..6 {
                out[6 + 6 * i + j] = (0..6).map(|k| a[i][k] * s[6 + 6 * k + j]).sum();
            }
        }
        out
    }

    /// 初期状態から各時刻（昇順、0 から測る）の状態を返す。
    pub fn flow(
        &self,
        x0: State6,
        times: &[f64],
        integrator: &Dop853,
    ) -> Result<Vec<State6>, Cr3bpError> {
        let out = integrator.integrate_at(|_, s: &State6| self.derivative(s), 0.0, x0, times)?;
        Ok(out)
    }

    /// 時刻 `t` の状態と状態遷移行列 Φ(t, 0)（6×6）。
    pub fn flow_with_stm(
        &self,
        x0: State6,
        t: f64,
        integrator: &Dop853,
    ) -> Result<(State6, [[f64; 6]; 6]), Cr3bpError> {
        let out = integrator.integrate_at(
            |_, s: &State42| self.derivative_with_stm(s),
            0.0,
            initial_42(x0),
            &[t],
        )?;
        let s = out[0];
        Ok(([s[0], s[1], s[2], s[3], s[4], s[5]], stm_of(&s)))
    }
}

/// 状態 + 単位行列の 42 要素。
pub(super) fn initial_42(x0: State6) -> State42 {
    let mut s = [0.0; 42];
    s[..6].copy_from_slice(&x0);
    for i in 0..6 {
        s[6 + 7 * i] = 1.0;
    }
    s
}

/// 42 要素から状態遷移行列を取り出す。
pub(super) fn stm_of(s: &State42) -> [[f64; 6]; 6] {
    let mut m = [[0.0; 6]; 6];
    for (i, row) in m.iter_mut().enumerate() {
        row.copy_from_slice(&s[6 + 6 * i..12 + 6 * i]);
    }
    m
}
