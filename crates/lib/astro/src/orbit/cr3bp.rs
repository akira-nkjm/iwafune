//! 円制限三体問題（CR3BP）の数学。2 天体の質量比 μ だけで決まる、任意の主天体・副天体の組に使える純粋な計算。
//!
//! 積分は [`crate::ode::Dop853`]、根の探索は [`crate::math::bisect`]、連立一次方程式は [`crate::math::solve_dense`]、
//! 慣性系の状態は [`crate::state::OrbitState`] を使い回す（ここに積分器・求根・線形解法は無い）。
//!
//! ## 規約
//! - **正規化**: 長さの単位は 2 天体の距離 a、時間の単位は 1/n（n = √((GM₁+GM₂)/a³)）、質量は m₁+m₂。
//!   μ = m₂/(m₁+m₂)（副天体の比）。
//! - **回転座標系**: 原点は 2 天体の重心、x 軸は主天体から副天体へ向く、z 軸は公転の角運動量の向き。
//!   主天体は (−μ, 0, 0)、副天体は (1−μ, 0, 0)。状態は `[x, y, z, vx, vy, vz]`（回転系から見た速度）。
//! - **有効ポテンシャル** Ω = (x²+y²)/2 + (1−μ)/r₁ + μ/r₂（定数項 μ(1−μ)/2 は含めない）、
//!   **ヤコビ定数** C = 2Ω − v²。
//!
//! | 機能 | ファイル |
//! |---|---|
//! | 質量比・[`Cr3bp`]・エラー | `cr3bp.rs` |
//! | ラグランジュ点 L1〜L5・線形化の固有振動数 | `cr3bp/points.rs` |
//! | 運動方程式・ポテンシャル・ヤコビ定数・変分方程式（42 状態）・流れ | `cr3bp/dynamics.rs` |
//! | xz 対称な周期軌道の補正（単一シューティング） | `cr3bp/corrector.rs` |
//! | ハロー軌道の 3 次の解析近似（Richardson 1980） | `cr3bp/halo_guess.rs` |
//! | 1 周期の表（エルミート補間用） | `cr3bp/sampling.rs` |
//! | 回転系 ⇄ 慣性系（脈動座標系） | `cr3bp/frame.rs` |

mod corrector;
mod dynamics;
mod frame;
mod halo_guess;
mod points;
mod sampling;

#[cfg(test)]
mod tests_corrector;
#[cfg(test)]
mod tests_dynamics;
#[cfg(test)]
mod tests_frame;
#[cfg(test)]
mod tests_halo;
#[cfg(test)]
mod tests_points;

pub use corrector::{CorrectorConfig, Fixed, PeriodicGuess, PeriodicOrbit};
pub use dynamics::{State6, State42};
pub use halo_guess::{Branch, HaloGuess, HaloOrbit, RichardsonCoefficients};
pub use points::{LibrationPoint, LinearModes};
pub use sampling::SampledOrbit;

use crate::AstroError;
use crate::constants::MU_EARTH;
use crate::ephem::CentralBody;

/// CR3BP の失敗。
#[derive(Debug, thiserror::Error)]
pub enum Cr3bpError {
    /// 質量比が (0, 0.5] の外。
    #[error("The mass ratio must be in (0, 0.5], got {0}")]
    InvalidMassRatio(f64),
    /// 共線点でない点に共線点の関数を使った。
    #[error("{0:?} is not a collinear libration point")]
    NotCollinear(LibrationPoint),
    /// 3 次の解析近似は L1 と L2 だけ。
    #[error("The third-order halo approximation exists only for L1 and L2, not {0:?}")]
    HaloPointUnsupported(LibrationPoint),
    /// 共線点の根が区間に無い。
    #[error("No root of the axis gradient was found for {0:?}")]
    NoRoot(LibrationPoint),
    /// この z 振幅ではハロー軌道の近似が存在しない（x 振幅の 2 乗が負）。
    #[error(
        "The halo approximation has no real x-amplitude at the z-amplitude {z_amplitude} (normalised)"
    )]
    NoHaloAtAmplitude { z_amplitude: f64 },
    /// 1 回の積分の間に xz 平面を再び横切らなかった。
    #[error("The orbit did not cross the xz-plane again within {limit} time units")]
    NoCrossing { limit: f64 },
    /// 補正の連立一次方程式が特異。
    #[error("The correction matrix is singular at iteration {iteration}")]
    SingularCorrection { iteration: usize },
    /// 反復の上限までに収束しなかった。
    #[error("The corrector did not converge in {iterations} iterations (residual {residual:e})")]
    NotConverged { iterations: usize, residual: f64 },
    /// 引数が不正（点数が足りない、など）。
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    /// 積分器のエラー。
    #[error(transparent)]
    Integration(#[from] AstroError),
}

/// 副天体の質量の取り方。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Secondary {
    /// その天体だけ。
    Body(CentralBody),
    /// 地球と月を 1 つの質点とみなす（太陽–地球系の L1/L2 では普通こちら。GM は地球 + 月）。
    EarthMoonSystem,
}

impl Secondary {
    /// 副天体の重力定数 GM [m³/s²]。
    pub fn gm(self) -> f64 {
        match self {
            Self::Body(b) => b.gm(),
            Self::EarthMoonSystem => MU_EARTH + CentralBody::Moon.gm(),
        }
    }
}

/// 質量比 μ で決まる円制限三体問題。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cr3bp {
    mu: f64,
}

impl Cr3bp {
    /// 質量比 μ = m₂/(m₁+m₂) から作る。μ は (0, 0.5]。
    pub fn new(mu: f64) -> Result<Self, Cr3bpError> {
        if mu.is_finite() && mu > 0.0 && mu <= 0.5 {
            Ok(Self { mu })
        } else {
            Err(Cr3bpError::InvalidMassRatio(mu))
        }
    }

    /// 2 つの GM [m³/s²] から作る（μ = GM₂/(GM₁+GM₂)）。
    pub fn from_gm(gm_primary: f64, gm_secondary: f64) -> Result<Self, Cr3bpError> {
        Self::new(gm_secondary / (gm_primary + gm_secondary))
    }

    /// 主天体と副天体（[`Secondary`]）の組から作る。GM は [`CentralBody::gm`] を使う。
    pub fn from_bodies(primary: CentralBody, secondary: Secondary) -> Result<Self, Cr3bpError> {
        Self::from_gm(primary.gm(), secondary.gm())
    }

    /// 質量比 μ。
    pub fn mu(&self) -> f64 {
        self.mu
    }
}
