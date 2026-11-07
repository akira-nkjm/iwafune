//! DOP853 の 1 ステップと 7 次の密出力（Hairer の連続拡張を `y0 + Σ w_j(s)·k_j` の形に直したもの）。

use super::OdeState;
use super::dop853_coeffs::{A, D, N_STAGES, N_STAGES_EXTENDED};

/// 受理された 1 ステップ（密出力に必要なものを持つ）。
pub(super) struct Step<S> {
    pub(super) t0: f64,
    pub(super) h: f64,
    pub(super) y0: S,
    pub(super) y1: S,
    /// 16 段の微分値（k[12] は f(t+h, y1)）
    pub(super) k: Vec<S>,
}

impl<S: OdeState> Step<S> {
    /// 区間内の時刻 `t` での補間値。
    pub(super) fn interpolate(&self, t: f64) -> S {
        let s = (t - self.t0) / self.h;
        let w = dense_weights(s, self.h);
        let mut y = self.y0.clone();
        for (wj, kj) in w.iter().zip(self.k.iter()) {
            if *wj != 0.0 {
                y = y.axpy(*wj, kj);
            }
        }
        y
    }
}

/// 密出力 `y(s) = y0 + Σ_j w_j k_j` の重み。
///
/// Hairer の連続拡張 `cont0 + s(cont1 + s1(cont2 + s(cont3 + s1(cont4 + s(cont5 + s1(cont6 + s cont7))))))`
/// で `cont1..7` はすべて `k_j` の一次結合なので、係数ベクトルどうしで同じ Horner 形を畳む。
fn dense_weights(s: f64, h: f64) -> [f64; N_STAGES_EXTENDED] {
    let s1 = 1.0 - s;
    let zero = [0.0; N_STAGES_EXTENDED];
    // cont1 = Δy = h Σ b_j k_j
    let mut c1 = zero;
    for (j, c) in c1.iter_mut().enumerate().take(N_STAGES) {
        *c = h * A[N_STAGES][j];
    }
    // cont2 = h k0 − Δy
    let mut c2 = zero;
    for (j, c) in c2.iter_mut().enumerate() {
        *c = -c1[j];
    }
    c2[0] += h;
    // cont3 = Δy − h k12 − cont2
    let mut c3 = zero;
    for (j, c) in c3.iter_mut().enumerate() {
        *c = c1[j] - c2[j];
    }
    c3[N_STAGES] -= h;
    // cont4..7 = h Σ D[r][j] k_j
    let mut c_hi = [zero; 4];
    for (r, row) in c_hi.iter_mut().enumerate() {
        for (j, c) in row.iter_mut().enumerate() {
            *c = h * D[r][j];
        }
    }
    let mut w = [0.0; N_STAGES_EXTENDED];
    for j in 0..N_STAGES_EXTENDED {
        let inner = c_hi[2][j] + s * c_hi[3][j];
        let inner = c_hi[1][j] + s1 * inner;
        let inner = c_hi[0][j] + s * inner;
        let inner = c3[j] + s1 * inner;
        let inner = c2[j] + s * inner;
        let inner = c1[j] + s1 * inner;
        w[j] = s * inner;
    }
    w
}
