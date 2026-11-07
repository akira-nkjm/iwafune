//! 固体地球潮汐による 2 次の重力場の変分（IERS 2010 Conventions §6.1 step 1、周波数に依らない公称 Love 数）。
//!
//! ```text
//!   ΔC̄_2m − i ΔS̄_2m = (k_2m/5) Σ_{j=月,太陽} (GM_j/GM_⊕)(R_⊕/r_j)³ P̄_2m(sin φ_j) e^{−i m λ_j}
//! ```
//!
//! φ_j・λ_j は天体の地球固定（ECEF）の緯度・経度。変分を 2 次だけの重力場として
//! [`super::gravity`] の同じ漸化式で評価する（中心項なしの摂動加速度）。
//! 周波数依存の補正（step 2）・3 次の潮汐・極潮汐・海洋潮汐は扱わない。

use super::gravity::GravityTable;
use crate::constants::{MU_EARTH, R_EARTH};
use crate::earth::{ecef_to_eci, eci_to_ecef};
use crate::ephem::{Body, position_eci_m};
use crate::math::Vec3;
use crate::time::Epoch;

/// 公称 Love 数 k20・k21・k22（IERS 2010 表 6.3、弾性地球）。
const LOVE_K2: [f64; 3] = [0.29525, 0.29470, 0.29801];

/// 太陽・月による 2 次の係数の変分 `[ΔC̄20, ΔC̄21, ΔS̄21, ΔC̄22, ΔS̄22]`。
pub(super) fn delta_coefficients(epoch: Epoch) -> [f64; 5] {
    let mut d = [0.0; 5];
    for body in [Body::Moon, Body::Sun] {
        let p = eci_to_ecef(position_eci_m(body, epoch), epoch);
        let r = p.norm();
        if r.is_nan() || r <= 0.0 {
            continue;
        }
        let x = p.z / r; // sin φ
        let cos_phi = (1.0 - x * x).max(0.0).sqrt();
        let lambda = p.y.atan2(p.x);
        let pre = body.gm() / MU_EARTH * (R_EARTH / r).powi(3) / 5.0;
        let sqrt5 = 5f64.sqrt();
        let sqrt15 = 15f64.sqrt();
        let p20 = sqrt5 * (3.0 * x * x - 1.0) / 2.0;
        let p21 = sqrt15 * x * cos_phi;
        let p22 = sqrt15 / 2.0 * cos_phi * cos_phi;
        let (s1, c1) = lambda.sin_cos();
        let (s2, c2) = (2.0 * lambda).sin_cos();
        d[0] += LOVE_K2[0] * pre * p20;
        d[1] += LOVE_K2[1] * pre * p21 * c1;
        d[2] += LOVE_K2[1] * pre * p21 * s1;
        d[3] += LOVE_K2[2] * pre * p22 * c2;
        d[4] += LOVE_K2[2] * pre * p22 * s2;
    }
    d
}

/// 固体潮汐による摂動加速度（ECI）[m/s²]。
pub fn solid_tide_eci(epoch: Epoch, r_eci_m: Vec3) -> Vec3 {
    let table = GravityTable::degree_two(MU_EARTH, R_EARTH, delta_coefficients(epoch));
    let a = table.acceleration_ecef(eci_to_ecef(r_eci_m, epoch), 2, 2, 2);
    ecef_to_eci(a, epoch)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn epoch() -> Epoch {
        Epoch::parse("2027-04-01T00:00:00Z").expect("元期")
    }

    /// 変分の大きさは 1e-8 級（月が支配的。ΔC̄20 の振幅は ~1e-8）。
    #[test]
    fn delta_coefficients_have_expected_magnitude() {
        let d = delta_coefficients(epoch());
        let max = d.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
        assert!(max > 1e-9 && max < 1e-7, "{d:?}");
    }

    /// LEO での潮汐加速度は ~1e-7 m/s² 以下（主重力の 1e-8 程度）。
    #[test]
    fn leo_acceleration_magnitude() {
        let r = Vec3::new(R_EARTH + 550e3, 0.0, 0.0);
        let a = solid_tide_eci(epoch(), r).norm();
        assert!(a > 1e-9 && a < 3e-7, "{a}");
    }

    /// 月の真下では、月の方向に伸びた膨らみが衛星と地心の間にあるので、変分の加速度は内向き。
    #[test]
    fn tide_under_the_moon_points_inward() {
        let e = epoch();
        // 太陽の寄与も入るので、月の方向の動径成分が正であることだけを見る。
        let moon_dir = position_eci_m(Body::Moon, e).normalized().expect("非ゼロ");
        let r = moon_dir * (R_EARTH + 550e3);
        let a = solid_tide_eci(e, r);
        assert!(a.dot(moon_dir) < 0.0, "{a:?}");
    }
}
