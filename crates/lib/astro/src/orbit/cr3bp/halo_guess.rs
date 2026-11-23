//! ハロー軌道の 3 次の解析近似（Richardson 1980）と、それを補正した軌道。
//!
//! 出典: D. L. Richardson, "Analytic construction of periodic orbits about the collinear points",
//! Celestial Mechanics 22, 241-253 (1980)。係数の式は論文そのものを手元で読めなかったので、
//! その実装である Orekit の `RichardsonExpansion`（Apache-2.0。Koon, Lo, Marsden, Ross の
//! *Dynamical Systems, the Three-Body Problem, and Space Mission Design* を参照している）の式を写した。
//! **論文との照合はしていない。** 係数 a31 の第 2 項の `3·c3` は、Orekit では `2·c3`、記憶では `3·c3` で食い違う
//! （b31 の対応する項が `3·c3` なので `3·c3` を採った。第 3 調和の項で、補正の結果には影響しない）。
//! この近似はあくまで補正の初期値で、採用の根拠は補正後の周期性の残差とヤコビ定数の保存である。
//!
//! 座標は、ラグランジュ点を原点とし γ（点から副天体までの距離）で割った長さ。回転系の x は主天体から副天体へ。

use super::corrector::{CorrectorConfig, PeriodicGuess, PeriodicOrbit};
use super::dynamics::State6;
use super::{Cr3bp, Cr3bpError, LibrationPoint};

/// ハロー軌道の枝。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Branch {
    /// 北（z > 0 の側が xz 平面の横切りでの z0）
    North,
    /// 南（北の枝を z → −z に写したもの）
    South,
}

/// Richardson の係数（γ で割った単位）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RichardsonCoefficients {
    pub gamma: f64,
    pub c2: f64,
    pub c3: f64,
    pub c4: f64,
    /// 面内の線形振動数 λ（ω_p）
    pub lambda: f64,
    pub k: f64,
    /// Δ = λ² − c2
    pub delta: f64,
    pub a21: f64,
    pub a22: f64,
    pub a23: f64,
    pub a24: f64,
    pub b21: f64,
    pub b22: f64,
    pub d21: f64,
    pub a31: f64,
    pub a32: f64,
    pub b31: f64,
    pub b32: f64,
    pub d31: f64,
    pub d32: f64,
    pub s1: f64,
    pub s2: f64,
    pub l1: f64,
    pub l2: f64,
}

/// 3 次の解析近似から作った初期値。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HaloGuess {
    /// xz 平面を横切る点（位相 0）の状態（正規化、回転系）
    pub state: State6,
    /// 周期（正規化）
    pub period: f64,
    /// x 振幅（γ で割った単位）
    pub ax: f64,
    /// z 振幅（γ で割った単位）
    pub az: f64,
}

/// 補正したハロー軌道と、補正前の近似との差。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HaloOrbit {
    pub guess: HaloGuess,
    pub orbit: PeriodicOrbit,
}

impl HaloOrbit {
    /// 補正後の状態 − 近似の状態（正規化）。
    pub fn state_difference(&self) -> State6 {
        std::array::from_fn(|i| self.orbit.state[i] - self.guess.state[i])
    }

    /// 補正後の周期 − 近似の周期（正規化）。
    pub fn period_difference(&self) -> f64 {
        self.orbit.period - self.guess.period
    }
}

/// `c_n`（n 次の係数）。L1 は上の符号、L2 は下の符号。
fn c_n(mu: f64, gamma: f64, point: LibrationPoint, n: i32) -> f64 {
    let sign = if n % 2 == 0 { 1.0 } else { -1.0 };
    let g = gamma.powi(n + 1);
    let v = match point {
        LibrationPoint::L1 => mu + sign * (1.0 - mu) * g / (1.0 - gamma).powi(n + 1),
        _ => sign * mu + sign * (1.0 - mu) * g / (1.0 + gamma).powi(n + 1),
    };
    v / gamma.powi(3)
}

impl RichardsonCoefficients {
    /// L1 または L2 の係数。
    pub fn new(cr: &Cr3bp, point: LibrationPoint) -> Result<Self, Cr3bpError> {
        if !matches!(point, LibrationPoint::L1 | LibrationPoint::L2) {
            return Err(Cr3bpError::HaloPointUnsupported(point));
        }
        let mu = cr.mu();
        let gamma = cr.gamma(point)?;
        let (c2, c3, c4) = (
            c_n(mu, gamma, point, 2),
            c_n(mu, gamma, point, 3),
            c_n(mu, gamma, point, 4),
        );
        let lambda = (0.5 * (2.0 - c2 + (9.0 * c2 * c2 - 8.0 * c2).sqrt())).sqrt();
        let k = (lambda * lambda + 1.0 + 2.0 * c2) / (2.0 * lambda);
        let mut c = Self::second_order(gamma, [c2, c3, c4], lambda, k);
        c.third_order();
        c.frequency_terms();
        Ok(c)
    }

    /// 2 次の係数（a21〜a24、b21、b22、d21）まで。
    fn second_order(gamma: f64, [c2, c3, c4]: [f64; 3], ld: f64, k: f64) -> Self {
        let d1 = 3.0 * ld * ld * (k * (6.0 * ld * ld - 1.0) - 2.0 * ld) / k;
        Self {
            gamma,
            c2,
            c3,
            c4,
            lambda: ld,
            k,
            delta: ld * ld - c2,
            a21: 3.0 * c3 * (k * k - 2.0) / (4.0 * (1.0 + 2.0 * c2)),
            a22: 3.0 * c3 / (4.0 * (1.0 + 2.0 * c2)),
            a23: -3.0 * c3 * ld * (3.0 * k * k * k * ld - 6.0 * k * (k - ld) + 4.0)
                / (4.0 * k * d1),
            a24: -3.0 * c3 * ld * (2.0 + 3.0 * k * ld) / (4.0 * k * d1),
            b21: -3.0 * c3 * ld * (3.0 * k * ld - 4.0) / (2.0 * d1),
            b22: 3.0 * c3 * ld / d1,
            d21: -c3 / (2.0 * ld * ld),
            ..Self::zero()
        }
    }

    fn zero() -> Self {
        Self {
            gamma: 0.0,
            c2: 0.0,
            c3: 0.0,
            c4: 0.0,
            lambda: 0.0,
            k: 0.0,
            delta: 0.0,
            a21: 0.0,
            a22: 0.0,
            a23: 0.0,
            a24: 0.0,
            b21: 0.0,
            b22: 0.0,
            d21: 0.0,
            a31: 0.0,
            a32: 0.0,
            b31: 0.0,
            b32: 0.0,
            d31: 0.0,
            d32: 0.0,
            s1: 0.0,
            s2: 0.0,
            l1: 0.0,
            l2: 0.0,
        }
    }

    /// 3 次の係数（a31、a32、b31、b32、d31、d32）。
    fn third_order(&mut self) {
        let (c2, c3, c4, ld, k) = (self.c2, self.c3, self.c4, self.lambda, self.k);
        let d2 = 8.0 * ld * ld * (k * (11.0 * ld * ld - 1.0) - 2.0 * ld) / k;
        let (a23, a24, b21, b22, d21) = (self.a23, self.a24, self.b21, self.b22, self.d21);
        let (q1, q2) = (9.0 * ld * ld + 1.0 - c2, 9.0 * ld * ld + 1.0 + 2.0 * c2);
        self.a31 = -9.0 * ld * (4.0 * c3 * (k * a23 - b21) + k * c4 * (4.0 + k * k)) / (4.0 * d2)
            + q1 / (2.0 * d2) * (3.0 * c3 * (2.0 * a23 - k * b21) + c4 * (2.0 + 3.0 * k * k));
        self.a32 = -9.0 * ld * (4.0 * c3 * (k * a24 - b22) + k * c4) / (4.0 * d2)
            - 3.0 * q1 * (c3 * (k * b22 + d21 - 2.0 * a24) - c4) / (2.0 * d2);
        self.b31 = 3.0 * 8.0 * ld * (3.0 * c3 * (k * b21 - 2.0 * a23) - c4 * (2.0 + 3.0 * k * k))
            / (8.0 * d2)
            + 3.0 * q2 * (4.0 * c3 * (k * a23 - b21) + k * c4 * (4.0 + k * k)) / (8.0 * d2);
        self.b32 = 9.0 * ld * (c3 * (k * b22 + d21 - 2.0 * a24) - c4) / d2
            + 3.0 * q2 * (4.0 * c3 * (k * a24 - b22) + k * c4) / (8.0 * d2);
        self.d31 = 3.0 / (64.0 * ld * ld) * (4.0 * c3 * a24 + c4);
        self.d32 = 3.0 / (64.0 * ld * ld) * (4.0 * c3 * (a23 - d21) + c4 * (4.0 + k * k));
    }

    /// 振動数の補正（s1、s2）と、振幅の関係式の係数（l1、l2）。
    fn frequency_terms(&mut self) {
        let (c3, c4, ld, k) = (self.c3, self.c4, self.lambda, self.k);
        let den = 2.0 * ld * (ld * (1.0 + k * k) - 2.0 * k);
        self.s1 = (3.0
            * c3
            * (2.0 * self.a21 * (k * k - 2.0) - self.a23 * (k * k + 2.0) - 2.0 * k * self.b21)
            / 2.0
            - 3.0 * c4 * (3.0 * k.powi(4) - 8.0 * k * k + 8.0) / 8.0)
            / den;
        self.s2 = (3.0
            * c3
            * (2.0 * self.a22 * (k * k - 2.0)
                + self.a24 * (k * k + 2.0)
                + 2.0 * k * self.b22
                + 5.0 * self.d21)
            / 2.0
            + 3.0 * c4 * (12.0 - k * k) / 8.0)
            / den;
        self.l1 = -3.0 * c3 * (2.0 * self.a21 + self.a23 + 5.0 * self.d21) / 2.0
            - 3.0 * c4 * (12.0 - k * k) / 8.0
            + 2.0 * ld * ld * self.s1;
        self.l2 =
            3.0 * c3 * (self.a24 - 2.0 * self.a22) / 2.0 + 9.0 * c4 / 8.0 + 2.0 * ld * ld * self.s2;
    }

    /// 振幅 `(ax, az)`（γ で割った単位）と位相 τ₁ での解析解の位置と速度（γ で割った単位、点が原点。速度は
    /// 正規化時間 1/n あたり）。`dm` は北が +1、南が −1。
    pub fn local_state(&self, ax: f64, az: f64, dm: f64, tau1: f64) -> State6 {
        let nu = 1.0 + self.s1 * ax * ax + self.s2 * az * az;
        let w = self.lambda * nu;
        let (s1, c1) = tau1.sin_cos();
        let (s2, c2) = (2.0 * tau1).sin_cos();
        let (s3, c3) = (3.0 * tau1).sin_cos();
        let px = self.a23 * ax * ax - self.a24 * az * az;
        let qx = self.a31 * ax.powi(3) - self.a32 * ax * az * az;
        let py = self.b21 * ax * ax - self.b22 * az * az;
        let qy = self.b31 * ax.powi(3) - self.b32 * ax * az * az;
        let pz = self.d21 * ax * az;
        let qz = self.d32 * az * ax * ax - self.d31 * az.powi(3);
        [
            self.a21 * ax * ax + self.a22 * az * az - ax * c1 + px * c2 + qx * c3,
            self.k * ax * s1 + py * s2 + qy * s3,
            dm * (az * c1 + pz * (c2 - 3.0) + qz * c3),
            w * (ax * s1 - 2.0 * px * s2 - 3.0 * qx * s3),
            w * (self.k * ax * c1 + 2.0 * py * c2 + 3.0 * qy * c3),
            -dm * w * (az * s1 + 2.0 * pz * s2 + 3.0 * qz * s3),
        ]
    }
}

impl Cr3bp {
    /// z 振幅 `z_amplitude`（正規化の長さ。2 天体の距離を 1 とする）のハロー軌道の 3 次の解析近似。
    /// 振幅は近似の z の最大値（γ 倍した値）。x 振幅は関係式 `l1·ax² + l2·az² + Δ = 0` から決まる。
    pub fn halo_guess(
        &self,
        point: LibrationPoint,
        branch: Branch,
        z_amplitude: f64,
    ) -> Result<HaloGuess, Cr3bpError> {
        let c = RichardsonCoefficients::new(self, point)?;
        let az = z_amplitude.abs() / c.gamma;
        let ax2 = -(c.delta + c.l2 * az * az) / c.l1;
        if ax2.is_nan() || ax2 <= 0.0 {
            return Err(Cr3bpError::NoHaloAtAmplitude { z_amplitude });
        }
        let ax = ax2.sqrt();
        let dm = if branch == Branch::North { 1.0 } else { -1.0 };
        let local = c.local_state(ax, az, dm, 0.0);
        let x_point = self.libration_point(point)?[0];
        let mut state: State6 = std::array::from_fn(|i| local[i] * c.gamma);
        state[0] += x_point;
        let nu = 1.0 + c.s1 * ax * ax + c.s2 * az * az;
        Ok(HaloGuess {
            state,
            period: (std::f64::consts::TAU / (c.lambda * nu)).abs(),
            ax,
            az,
        })
    }

    /// 3 次の解析近似から出発して補正したハロー軌道。補正は z0 を固定する（[`CorrectorConfig::default`]）。
    pub fn halo_orbit(
        &self,
        point: LibrationPoint,
        branch: Branch,
        z_amplitude: f64,
        cfg: &CorrectorConfig,
    ) -> Result<HaloOrbit, Cr3bpError> {
        let guess = self.halo_guess(point, branch, z_amplitude)?;
        let orbit = self.correct_periodic(
            &PeriodicGuess {
                state: guess.state,
                period: guess.period,
            },
            cfg,
        )?;
        Ok(HaloOrbit { guess, orbit })
    }
}
