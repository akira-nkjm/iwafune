//! 面への入射（太陽・中心天体）。**電源（発電）と熱（入熱）で共有する。**

use crate::constants::{AU, SOLAR_CONSTANT};
use crate::math::Vec3;

/// 太陽光の入射係数 = max(0, n·s) × 日照率。`normal` と `sun_dir` は同じ座標系の単位ベクトル。
pub fn solar_incidence(normal: Vec3, sun_dir: Vec3, shadow_factor: f64) -> f64 {
    normal.dot(sun_dir).max(0.0) * shadow_factor.clamp(0.0, 1.0)
}

/// 太陽からの距離 `sun_distance_m` [m] での太陽光の強さ [W/m²] = SOLAR_CONSTANT·(AU/d)²。
pub fn solar_flux_w_m2(sun_distance_m: f64) -> f64 {
    if sun_distance_m <= 0.0 {
        return SOLAR_CONSTANT;
    }
    let ratio = AU / sun_distance_m;
    SOLAR_CONSTANT * ratio * ratio
}

/// 平板が半径 `body_radius_m` の中心天体（地球・月・惑星）を見る形態係数（0〜1）。`normal_eci` は面の外向き法線、
/// `r_eci_m` は中心天体の中心から衛星への位置 [m]（同じ座標系）。
///
/// 球に対する傾いた平板の形態係数（Lambert 面、片面）。天体の見込み半角を λ（sin λ = R/r）、
/// 法線と天底のなす角を θ とすると:
/// - θ ≤ π/2 − λ（天体全体が面の前にある）: F = cos θ · sin² λ
/// - θ ≥ π/2 + λ（天体が面の裏）: F = 0
/// - その間（地平線が面を横切る）: 部分的に見える場合の厳密式（Howell, View Factor Catalog C-9）
pub fn body_view_factor(normal_eci: Vec3, r_eci_m: Vec3, body_radius_m: f64) -> f64 {
    let r = r_eci_m.norm();
    let (Some(n), Some(nadir)) = (normal_eci.normalized(), (-r_eci_m).normalized()) else {
        return 0.0;
    };
    if r <= body_radius_m {
        // 地表以下: 天底を向く面は天体で覆われる。
        return n.dot(nadir).max(0.0);
    }
    let h = r / body_radius_m; // H = r/R
    let sin_lambda = 1.0 / h;
    let lambda = sin_lambda.asin();
    let cos_theta = n.dot(nadir).clamp(-1.0, 1.0);
    let theta = cos_theta.acos();
    let half_pi = core::f64::consts::FRAC_PI_2;
    if theta <= half_pi - lambda {
        return cos_theta * sin_lambda * sin_lambda;
    }
    if theta >= half_pi + lambda {
        return 0.0;
    }
    partial_view_factor(h, theta)
}

/// 地平線が面を横切る場合の形態係数（Howell C-9 の式）。
fn partial_view_factor(h: f64, theta: f64) -> f64 {
    let pi = core::f64::consts::PI;
    let (sin_t, cos_t) = theta.sin_cos();
    let x = (h * h - 1.0).sqrt();
    let y = -x * cos_t / sin_t;
    let y_clamped = y.clamp(-1.0, 1.0);
    let term1 = (1.0 / (pi * h * h))
        * (cos_t * y_clamped.acos() - x * sin_t * (1.0 - y * y).max(0.0).sqrt());
    let term2 = (1.0 / pi) * (sin_t * (1.0 - y * y).max(0.0).sqrt() / x).atan();
    (term1 + term2).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{R_EARTH, R_MOON};

    fn earth_view_factor(n: Vec3, r: Vec3) -> f64 {
        body_view_factor(n, r, R_EARTH)
    }

    #[test]
    fn incidence_is_cosine_and_clipped() {
        assert!((solar_incidence(Vec3::X, Vec3::X, 1.0) - 1.0).abs() < 1e-15);
        assert_eq!(solar_incidence(Vec3::X, -Vec3::X, 1.0), 0.0);
        let s = Vec3::new(1.0, 1.0, 0.0).normalized().expect("非ゼロ");
        assert!((solar_incidence(Vec3::X, s, 0.5) - 0.5 / 2.0_f64.sqrt()).abs() < 1e-15);
    }

    #[test]
    fn flux_at_one_au_is_solar_constant() {
        assert!((solar_flux_w_m2(AU) - SOLAR_CONSTANT).abs() < 1e-9);
        assert!((solar_flux_w_m2(2.0 * AU) - SOLAR_CONSTANT / 4.0).abs() < 1e-9);
    }

    #[test]
    fn nadir_facing_plate_at_550_km() {
        // 天底向き: F = (R/r)²。550 km で ≈ 0.8526。
        let r = Vec3::new(R_EARTH + 550e3, 0.0, 0.0);
        let f = earth_view_factor(-Vec3::X, r);
        let expected = (R_EARTH / (R_EARTH + 550e3)).powi(2);
        assert!((f - expected).abs() < 1e-12, "{f}");
        assert_eq!(earth_view_factor(Vec3::X, r), 0.0);
    }

    /// 月の 100 km の天底向き: F = (R/r)² = (1737.4/1837.4)² ≈ 0.894（半径だけが違う同じ式）。
    #[test]
    fn nadir_facing_plate_over_the_moon() {
        let r = Vec3::new(R_MOON + 100e3, 0.0, 0.0);
        let f = body_view_factor(-Vec3::X, r, R_MOON);
        assert!((f - 0.894_1).abs() < 1e-3, "{f}");
    }

    #[test]
    fn view_factor_is_continuous_across_regimes() {
        let r = Vec3::new(R_EARTH + 550e3, 0.0, 0.0);
        let mut prev = earth_view_factor(-Vec3::X, r);
        for i in 1..=1800 {
            let theta = (i as f64) * core::f64::consts::PI / 1800.0;
            let n = Vec3::new(-theta.cos(), theta.sin(), 0.0);
            let f = earth_view_factor(n, r);
            assert!((f - prev).abs() < 5e-3, "θ={theta}: {prev} → {f}");
            assert!(f <= prev + 1e-12, "単調減少でない θ={theta}");
            prev = f;
        }
        // 側面（θ = 90°）の値: 550 km で ≈ 0.31（Howell の表と整合する大きさ）
        let side = earth_view_factor(Vec3::Y, r);
        assert!(side > 0.25 && side < 0.36, "{side}");
    }
}
