//! ラグランジュ点 L1〜L5 と、共線点まわりの線形化（固有振動数）。

use super::{Cr3bp, Cr3bpError};
use crate::math::bisect;

/// ラグランジュ点。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibrationPoint {
    L1,
    L2,
    L3,
    L4,
    L5,
}

/// 共線点まわりの線形化（正規化の単位）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearModes {
    /// c = (1−μ)/|x+μ|³ + μ/|x−1+μ|³（Ωxx = 1+2c、Ωyy = 1−c、Ωzz = −c）
    pub c: f64,
    /// 面内の振動の角振動数 ω_p（λ⁴ + (2−c)λ² + (1+2c)(1−c) = 0 の虚根）
    pub omega_planar: f64,
    /// 鉛直の振動の角振動数 ω_z = √c
    pub omega_vertical: f64,
}

/// 共線点の探索で特異点（天体）から離す距離。
const EDGE: f64 = 1e-9;
/// 二分法の許容幅（正規化）と反復の上限。
const ROOT_TOL: f64 = 1e-16;
const ROOT_ITER: usize = 400;

impl Cr3bp {
    /// x 軸上（y = z = 0）の Ω の勾配 Ωx。共線点はこの零点。
    pub fn axis_gradient(&self, x: f64) -> f64 {
        self.potential_gradient([x, 0.0, 0.0])[0]
    }

    /// 回転系でのラグランジュ点の位置（z = 0）。共線点は x 軸の勾配の零点を [`bisect`] で、L4/L5 は閉形式
    /// （(1/2 − μ, ±√3/2)）。
    pub fn libration_point(&self, p: LibrationPoint) -> Result<[f64; 3], Cr3bpError> {
        let mu = self.mu();
        let (lo, hi) = match p {
            LibrationPoint::L4 => return Ok([0.5 - mu, 3.0_f64.sqrt() / 2.0, 0.0]),
            LibrationPoint::L5 => return Ok([0.5 - mu, -(3.0_f64.sqrt()) / 2.0, 0.0]),
            LibrationPoint::L1 => (-mu + EDGE, 1.0 - mu - EDGE),
            LibrationPoint::L2 => (1.0 - mu + EDGE, 2.0),
            LibrationPoint::L3 => (-2.0, -mu - EDGE),
        };
        bisect(|x| self.axis_gradient(x), lo, hi, ROOT_TOL, ROOT_ITER)
            .map(|x| [x, 0.0, 0.0])
            .ok_or(Cr3bpError::NoRoot(p))
    }

    /// 共線点から最も近い主天体までの距離 γ（L1・L2 は副天体から、L3 は主天体から）。
    pub fn gamma(&self, p: LibrationPoint) -> Result<f64, Cr3bpError> {
        let x = self.libration_point(p)?[0];
        match p {
            LibrationPoint::L1 | LibrationPoint::L2 => Ok((x - (1.0 - self.mu())).abs()),
            LibrationPoint::L3 => Ok((x + self.mu()).abs()),
            _ => Err(Cr3bpError::NotCollinear(p)),
        }
    }

    /// 共線点まわりの線形化の振動数。ヘッセ行列から導く（Ωxx = 1+2c、Ωyy = 1−c、Ωzz = −c）。
    pub fn linear_modes(&self, p: LibrationPoint) -> Result<LinearModes, Cr3bpError> {
        if matches!(p, LibrationPoint::L4 | LibrationPoint::L5) {
            return Err(Cr3bpError::NotCollinear(p));
        }
        let pos = self.libration_point(p)?;
        let c = -self.potential_hessian(pos)[2][2];
        let omega_planar = (0.5 * ((c * (9.0 * c - 8.0)).sqrt() + 2.0 - c)).sqrt();
        Ok(LinearModes {
            c,
            omega_planar,
            omega_vertical: c.sqrt(),
        })
    }
}
