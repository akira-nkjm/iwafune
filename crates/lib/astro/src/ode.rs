//! 常微分方程式の積分器。**ワークスペースで積分器はここだけ**（軌道・姿勢・熱・電池の時間積分はすべてこれを使う）。
//!
//! - [`rk4_step`] / [`integrate`]: 固定刻みの古典的 RK4。入力を刻みの間一定にする（sample-and-hold）ループ向け。
//! - [`Dop853`]: 適応刻みの Dormand–Prince 8(5,3)（密出力付き）。精度が要る軌道伝搬などに使う。
//!   Hairer の `dop853.f` を移植し、任意の状態ベクトルに使えるようにしたもの。

mod dense_output;
mod dop853;
mod dop853_coeffs;

/// 積分できる状態ベクトル。
pub trait OdeState: Clone {
    /// `self + k * other`
    fn axpy(&self, k: f64, other: &Self) -> Self;
    /// 成分の列（適応刻みの誤差評価に使う）。
    fn as_slice(&self) -> &[f64];
}

impl OdeState for Vec<f64> {
    fn axpy(&self, k: f64, other: &Self) -> Self {
        self.iter()
            .zip(other.iter())
            .map(|(a, b)| a + k * b)
            .collect()
    }
    fn as_slice(&self) -> &[f64] {
        self
    }
}

impl<const N: usize> OdeState for [f64; N] {
    fn axpy(&self, k: f64, other: &Self) -> Self {
        let mut out = *self;
        for (o, b) in out.iter_mut().zip(other.iter()) {
            *o += k * b;
        }
        out
    }
    fn as_slice(&self) -> &[f64] {
        self
    }
}

/// 古典的 4 次ルンゲクッタで 1 ステップ進める。`f(t, x)` は微分。
pub fn rk4_step<S: OdeState>(f: impl Fn(f64, &S) -> S, t: f64, x: &S, dt: f64) -> S {
    let half = 0.5 * dt;
    let k1 = f(t, x);
    let k2 = f(t + half, &x.axpy(half, &k1));
    let k3 = f(t + half, &x.axpy(half, &k2));
    let k4 = f(t + dt, &x.axpy(dt, &k3));
    let sixth = dt / 6.0;
    x.axpy(sixth, &k1)
        .axpy(2.0 * sixth, &k2)
        .axpy(2.0 * sixth, &k3)
        .axpy(sixth, &k4)
}

/// 刻み `[t0, t0 + dt]` の間の入力を両端の値の線形補間とみなして、[`rk4_step`] で 1 刻み進める。
/// `rate(w, x)` は区間の中の位置 `w`（0 = 始め、1 = 終わり）での微分。
/// **標本の間を線形補間して状態を積分する（電池・角運動量など）ときはこれを使う。**
pub fn rk4_linear_step<S: OdeState>(rate: impl Fn(f64, &S) -> S, t0: f64, x: &S, dt: f64) -> S {
    let w = |t: f64| if dt > 0.0 { (t - t0) / dt } else { 0.0 };
    rk4_step(|t, x: &S| rate(w(t), x), t0, x, dt)
}

/// 固定刻みで `t0` から `t1` まで積分し、各ステップ後に `on_step(t, x)` を呼ぶ。最後の刻みは端数に合わせる。
///
/// `dt` が正でない・有限でない、または `t1 <= t0` のときは何もせず `x0` を返す。
/// 刻みの時刻は `t0 + i·dt` で計算する（足し込みによる丸め誤差を溜めない）。
pub fn integrate<S: OdeState>(
    f: impl Fn(f64, &S) -> S,
    t0: f64,
    x0: S,
    t1: f64,
    dt: f64,
    mut on_step: impl FnMut(f64, &S),
) -> S {
    if !(dt > 0.0 && dt.is_finite()) || t1 <= t0 || !t1.is_finite() {
        return x0;
    }
    let offsets = step_offsets(t1 - t0, dt);
    let last = offsets.len() - 1;
    let mut x = x0;
    let mut t = t0;
    for (i, off) in offsets.iter().enumerate().skip(1) {
        let t_next = if i == last { t1 } else { (t0 + off).min(t1) };
        x = rk4_step(&f, t, &x, t_next - t);
        t = t_next;
        on_step(t, &x);
    }
    x
}

/// 固定刻みの格子 `0, dt, 2·dt, …, span` の点の数（最後は端数でも `span` ちょうど。端数が刻みの 1e-9 未満なら
/// 1 刻み増やさずに丸める）。**固定刻みの格子はここだけ**（[`integrate`] と時系列の時刻列が使う）。
pub fn step_count(span: f64, dt: f64) -> usize {
    if !(dt > 0.0 && dt.is_finite() && span >= 0.0 && span.is_finite()) {
        return 1;
    }
    let n_full = (span / dt * (1.0 + 1e-12)).floor();
    let n = n_full as usize;
    let tail = span - (n_full * dt).min(span) > 1e-9 * dt;
    n + 1 + usize::from(tail)
}

/// 固定刻みの格子の点（`0` からの経過。[`step_count`] と同じ規則）。刻みや期間が不正なら `[0]`。
pub fn step_offsets(span: f64, dt: f64) -> Vec<f64> {
    let count = step_count(span, dt);
    let mut out: Vec<f64> = (0..count).map(|i| (i as f64 * dt).min(span)).collect();
    if let Some(last) = out.last_mut()
        && count > 1
    {
        *last = span;
    }
    out
}

/// 適応刻みの Dormand–Prince 8(5,3) 積分器の設定。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dop853 {
    /// 相対許容誤差
    pub rtol: f64,
    /// 絶対許容誤差
    pub atol: f64,
    /// 最大刻み [s]
    pub max_step: f64,
    /// 最大ステップ数（超えたらエラー）
    pub max_steps: usize,
}

impl Default for Dop853 {
    fn default() -> Self {
        Self {
            rtol: 1e-10,
            atol: 1e-10,
            max_step: f64::INFINITY,
            max_steps: 1_000_000,
        }
    }
}

impl Dop853 {
    /// `t0` から各時刻 `times`（昇順）での状態を、密出力で補間して返す。
    ///
    /// `times` に `t0` より前の時刻があればエラー。`t0` ちょうどの時刻には `x0` を返す。
    pub fn integrate_at<S: OdeState>(
        &self,
        f: impl Fn(f64, &S) -> S,
        t0: f64,
        x0: S,
        times: &[f64],
    ) -> Result<Vec<S>, crate::AstroError> {
        dop853::integrate_at(self, &f, t0, x0, times)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rk4_exponential_decay() {
        // dx/dt = -x, x(0)=1 → x(1) = e^-1。刻み 0.01 で誤差 ~1e-11。
        let f = |_t: f64, x: &[f64; 1]| [-x[0]];
        let x = integrate(f, 0.0, [1.0], 1.0, 0.01, |_, _| {});
        assert!((x[0] - (-1.0_f64).exp()).abs() < 1e-10, "{}", x[0]);
    }

    #[test]
    fn rk4_harmonic_oscillator_vec_state() {
        let f = |_t: f64, x: &Vec<f64>| vec![x[1], -x[0]];
        let tau = core::f64::consts::TAU;
        let x = integrate(f, 0.0, vec![1.0, 0.0], tau, 0.001, |_, _| {});
        assert!((x[0] - 1.0).abs() < 1e-10 && x[1].abs() < 1e-10, "{x:?}");
    }

    #[test]
    fn integrate_calls_back_each_step_and_ends_at_t1() {
        let mut times = Vec::new();
        let f = |_t: f64, _x: &[f64; 1]| [1.0];
        let x = integrate(f, 0.0, [0.0], 1.05, 0.25, |t, _| times.push(t));
        assert_eq!(times.len(), 5);
        assert!((times[4] - 1.05).abs() < 1e-15);
        assert!((x[0] - 1.05).abs() < 1e-12);
        // ちょうど割り切れる場合は余分な刻みを足さない。
        let mut n = 0;
        let _ = integrate(f, 0.0, [0.0], 1.0, 0.1, |_, _| n += 1);
        assert_eq!(n, 10);
    }

    #[test]
    fn integrate_ignores_bad_step() {
        let f = |_t: f64, _x: &[f64; 1]| [1.0];
        assert_eq!(integrate(f, 0.0, [3.0], 1.0, 0.0, |_, _| {}), [3.0]);
        assert_eq!(integrate(f, 1.0, [3.0], 0.0, 0.1, |_, _| {}), [3.0]);
    }
}

#[cfg(test)]
mod linear_step_tests {
    #[test]
    fn rk4_linear_step_integrates_a_ramp_exactly() {
        // dx/dt = 0 → 2 の線形補間を 10 s: 面積 10
        let x = super::rk4_linear_step(|w, _: &[f64; 1]| [2.0 * w], 5.0, &[1.0], 10.0);
        assert!((x[0] - 11.0).abs() < 1e-12);
    }
}
