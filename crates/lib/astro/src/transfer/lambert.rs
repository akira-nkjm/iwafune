//! Lambert の問題（2 点と飛行時間から、通る軌道の両端の速度を求める）。
//!
//! 普遍変数 z と Stumpff 関数 S(z)・C(z) の定式化（Curtis, *Orbital Mechanics for Engineering Students*, 3rd ed.,
//! Algorithm 5.2。Vallado, *Fundamentals of Astrodynamics and Applications*, 4th ed., Algorithm 58）。
//! 周回数 0 の解だけを扱い、z は二分法で求める（F(z) は y(z) > 0 の範囲で単調増加。Vallado の解き方）。

use crate::AstroError;
use crate::math::Vec3;
use core::f64::consts::PI;

/// Lambert の問題の解（両端の速度。位置と同じ座標系）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LambertSolution {
    /// 出発の点での速度 [m/s]
    pub v1_m_s: Vec3,
    /// 到着の点での速度 [m/s]
    pub v2_m_s: Vec3,
}

/// Stumpff 関数 S(z)。
fn stumpff_s(z: f64) -> f64 {
    if z > 1e-6 {
        let s = z.sqrt();
        (s - s.sin()) / (s * s * s)
    } else if z < -1e-6 {
        let s = (-z).sqrt();
        (s.sinh() - s) / (s * s * s)
    } else {
        1.0 / 6.0 - z / 120.0 + z * z / 5040.0
    }
}

/// Stumpff 関数 C(z)。
fn stumpff_c(z: f64) -> f64 {
    if z > 1e-6 {
        (1.0 - z.sqrt().cos()) / z
    } else if z < -1e-6 {
        ((-z).sqrt().cosh() - 1.0) / (-z)
    } else {
        0.5 - z / 24.0 + z * z / 720.0
    }
}

/// 二分法の上限の回数。
const MAX_ITERATIONS: usize = 400;

/// 重力定数 `mu` [m³/s²] の天体のまわりで、位置 `r1_m` から `r2_m` [m] へ飛行時間 `tof_s` [s] で行く軌道（周回数 0）。
/// `prograde` が真なら角運動量の z 成分が正の向き（順行）に回る解を選ぶ。2 点が同じ向き・正反対で面が決まらない、
/// 飛行時間が正でないときは `InvalidInput`。
pub fn lambert(
    mu: f64,
    r1_m: Vec3,
    r2_m: Vec3,
    tof_s: f64,
    prograde: bool,
) -> Result<LambertSolution, AstroError> {
    let invalid =
        |why: &str| AstroError::InvalidInput(format!("Lambert の問題を解けません（{why}）"));
    if !(mu > 0.0 && tof_s > 0.0 && tof_s.is_finite()) {
        return Err(invalid("重力定数と飛行時間は正にしてください"));
    }
    let (r1, r2) = (r1_m.norm(), r2_m.norm());
    let cos_dnu = (r1_m.dot(r2_m) / (r1 * r2)).clamp(-1.0, 1.0);
    let mut dnu = cos_dnu.acos();
    let z_up = r1_m.cross(r2_m).z;
    if (prograde && z_up < 0.0) || (!prograde && z_up >= 0.0) {
        dnu = 2.0 * PI - dnu;
    }
    let a = dnu.sin() * (r1 * r2 / (1.0 - cos_dnu)).sqrt();
    if !a.is_finite() || a.abs() < 1e-12 * (r1 + r2) {
        return Err(invalid("2 点の向きから軌道面が決まりません"));
    }
    let y = |z: f64| r1 + r2 + a * (z * stumpff_s(z) - 1.0) / stumpff_c(z).sqrt();
    let f = |z: f64, y: f64| {
        (y / stumpff_c(z)).powf(1.5) * stumpff_s(z) + a * y.sqrt() - mu.sqrt() * tof_s
    };
    let (mut lo, mut hi) = (-4.0 * PI * PI, 4.0 * PI * PI * (1.0 - 1e-12));
    let mut z = 0.0;
    for _ in 0..MAX_ITERATIONS {
        z = 0.5 * (lo + hi);
        let yz = y(z);
        if yz < 0.0 || f(z, yz) < 0.0 {
            lo = z;
        } else {
            hi = z;
        }
        if hi - lo < 1e-13 * (1.0 + z.abs()) {
            break;
        }
    }
    let yz = y(z);
    if yz.is_nan() || yz <= 0.0 || f(z, yz).abs() > 1e-6 * mu.sqrt() * tof_s {
        return Err(invalid("普遍変数が収束しません"));
    }
    let lf = 1.0 - yz / r1;
    let g = a * (yz / mu).sqrt();
    let gdot = 1.0 - yz / r2;
    Ok(LambertSolution {
        v1_m_s: (r2_m - r1_m * lf) / g,
        v2_m_s: (r2_m * gdot - r1_m) / g,
    })
}
