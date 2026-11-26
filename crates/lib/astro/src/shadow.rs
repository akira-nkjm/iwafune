//! 地球の影。**蝕の判定はここだけ。**
//!
//! - 円柱影: 太陽方向に平行な半径 R_E の円柱の中（太陽と反対側）なら 0。
//! - 円錐影: 衛星から見た太陽円盤と地球円盤の重なりの面積比（Montenbruck & Gill §3.4.2）。

use crate::constants::{R_EARTH, R_SUN};
use crate::ephem::CentralBody;
use crate::math::Vec3;
use serde::{Deserialize, Serialize};

/// 影のモデル。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ShadowModel {
    /// 円柱影（本影だけ。0 か 1）
    Cylindrical,
    /// 円錐影（半影で連続的に変わる）
    #[default]
    Conical,
}

/// 太陽光の当たり具合。1 = 全日照、0 = 本影。
pub fn earth_shadow_factor(r_sat_eci_m: Vec3, r_sun_eci_m: Vec3, model: ShadowModel) -> f64 {
    body_shadow_factor(r_sat_eci_m, r_sun_eci_m, Vec3::ZERO, R_EARTH, model)
}

/// 中心 `occulter_m`・半径 `occulter_radius_m` の球（月・惑星など）による影。位置はどれも同じ原点から [m]。
pub fn body_shadow_factor(
    r_sat_m: Vec3,
    r_sun_m: Vec3,
    occulter_m: Vec3,
    occulter_radius_m: f64,
    model: ShadowModel,
) -> f64 {
    match model {
        ShadowModel::Cylindrical => cylindrical(
            r_sat_m - occulter_m,
            r_sun_m - occulter_m,
            occulter_radius_m,
        ),
        ShadowModel::Conical => conical(r_sat_m, r_sun_m, occulter_m, occulter_radius_m),
    }
}

/// 軌道の中心天体による影（位置はどれも中心天体からの相対）。地球・月・惑星はその天体の球で隠し、
/// 中心天体が太陽なら影なし（1）。時系列（日照率）と力モデル（SRP）は同じこれを使う。
pub fn central_shadow_factor(
    central: CentralBody,
    r_sat_m: Vec3,
    r_sun_m: Vec3,
    model: ShadowModel,
) -> f64 {
    match central {
        CentralBody::Sun => 1.0,
        other => body_shadow_factor(r_sat_m, r_sun_m, Vec3::ZERO, other.radius_m(), model),
    }
}

/// 観測者 `observer_m` から目標 `target_m` を見る線分が、中心 `occulter_m`・半径 `occulter_radius_m` の球に
/// さえぎられるか（例: 衛星から見て月が地球の陰にあるか。位置はどれも同じ原点から [m]）。
/// 観測者が球の中なら `true`。
pub fn occulted_by(
    occulter_m: Vec3,
    occulter_radius_m: f64,
    observer_m: Vec3,
    target_m: Vec3,
) -> bool {
    let to_occ = occulter_m - observer_m;
    if to_occ.norm() <= occulter_radius_m {
        return true;
    }
    let seg = target_m - observer_m;
    let len2 = seg.norm_squared();
    if len2 <= 0.0 {
        return false;
    }
    let s = (to_occ.dot(seg) / len2).clamp(0.0, 1.0);
    (observer_m + seg * s - occulter_m).norm() < occulter_radius_m
}

fn cylindrical(r_sat: Vec3, r_sun: Vec3, radius: f64) -> f64 {
    let Some(s) = r_sun.normalized() else {
        return 1.0;
    };
    let along = r_sat.dot(s);
    if along >= 0.0 {
        return 1.0;
    }
    let perp = (r_sat - s * along).norm();
    if perp < radius { 0.0 } else { 1.0 }
}

/// 中心 `occ_center`・半径 `occ_radius` の球による円錐影（月食にも使える形）。
fn conical(r_sat: Vec3, r_sun: Vec3, occ_center: Vec3, occ_radius: f64) -> f64 {
    let to_sun = r_sun - r_sat;
    let to_occ = occ_center - r_sat;
    let d_sun = to_sun.norm();
    let d_occ = to_occ.norm();
    if d_occ <= occ_radius {
        return 0.0;
    }
    if d_sun <= R_SUN {
        return 1.0;
    }
    let a = (R_SUN / d_sun).asin();
    let b = (occ_radius / d_occ).asin();
    let c = (to_sun.dot(to_occ) / (d_sun * d_occ))
        .clamp(-1.0, 1.0)
        .acos();
    if c >= a + b {
        1.0
    } else if c <= b - a {
        0.0
    } else if c <= a - b {
        1.0 - (b * b) / (a * a)
    } else {
        1.0 - circle_overlap_area(a, b, c) / (core::f64::consts::PI * a * a)
    }
}

/// 角半径 a・b の 2 円（中心間 c）の重なり面積。
fn circle_overlap_area(a: f64, b: f64, c: f64) -> f64 {
    let cos_alpha = ((c * c + a * a - b * b) / (2.0 * c * a)).clamp(-1.0, 1.0);
    let cos_beta = ((c * c + b * b - a * a) / (2.0 * c * b)).clamp(-1.0, 1.0);
    let tri = ((-c + a + b) * (c + a - b) * (c - a + b) * (c + a + b)).max(0.0);
    a * a * cos_alpha.acos() + b * b * cos_beta.acos() - 0.5 * tri.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::AU;

    const SUN: Vec3 = Vec3::new(AU, 0.0, 0.0);

    #[test]
    fn sunlit_and_umbra() {
        let r = R_EARTH + 550e3;
        for model in [ShadowModel::Cylindrical, ShadowModel::Conical] {
            assert_eq!(earth_shadow_factor(Vec3::new(r, 0.0, 0.0), SUN, model), 1.0);
            assert_eq!(
                earth_shadow_factor(Vec3::new(-r, 0.0, 0.0), SUN, model),
                0.0
            );
            assert_eq!(earth_shadow_factor(Vec3::new(0.0, r, 0.0), SUN, model), 1.0);
        }
    }

    #[test]
    fn penumbra_is_between_zero_and_one_and_monotonic() {
        // 地心から −x 側、影の縁付近を y 方向に横切る。
        let x = -(R_EARTH + 550e3) * 0.3;
        let mut prev = 0.0;
        let mut saw_partial = false;
        for i in 0..4000 {
            let y = R_EARTH - 40e3 + f64::from(i) * 20.0;
            let nu = earth_shadow_factor(Vec3::new(x, y, 0.0), SUN, ShadowModel::Conical);
            assert!((0.0..=1.0).contains(&nu));
            assert!(nu + 1e-12 >= prev, "not monotonic");
            saw_partial |= nu > 0.01 && nu < 0.99;
            prev = nu;
        }
        assert!(saw_partial);
        assert_eq!(prev, 1.0);
    }

    #[test]
    fn moon_shadow_matches_earth_shadow_when_scaled() {
        // 月の影: 月を原点に置いた場合と同じ幾何になる
        let moon = Vec3::new(3.8e8, 0.0, 0.0);
        let r = moon + Vec3::new(-(crate::constants::R_MOON + 100e3), 0.0, 0.0);
        let sun = Vec3::new(AU, 0.0, 0.0);
        for model in [ShadowModel::Cylindrical, ShadowModel::Conical] {
            assert_eq!(
                body_shadow_factor(r, sun, moon, crate::constants::R_MOON, model),
                0.0
            );
        }
        let lit = moon + Vec3::new(crate::constants::R_MOON + 100e3, 0.0, 0.0);
        assert_eq!(
            body_shadow_factor(
                lit,
                sun,
                moon,
                crate::constants::R_MOON,
                ShadowModel::Conical
            ),
            1.0
        );
    }

    #[test]
    fn occultation_of_the_moon_by_the_earth() {
        let moon = Vec3::new(3.84e8, 0.0, 0.0);
        let r = R_EARTH + 550e3;
        // 月の反対側の衛星からは地球の陰
        assert!(occulted_by(
            Vec3::ZERO,
            R_EARTH,
            Vec3::new(-r, 0.0, 0.0),
            moon
        ));
        // 月の側・横からは見える
        assert!(!occulted_by(
            Vec3::ZERO,
            R_EARTH,
            Vec3::new(r, 0.0, 0.0),
            moon
        ));
        assert!(!occulted_by(
            Vec3::ZERO,
            R_EARTH,
            Vec3::new(0.0, r, 0.0),
            moon
        ));
        // 目標が観測者と遮蔽物の間なら隠れない
        assert!(!occulted_by(
            Vec3::ZERO,
            R_EARTH,
            Vec3::new(-r, 0.0, 0.0),
            Vec3::new(-2.0 * r, 0.0, 0.0)
        ));
        assert_eq!(
            central_shadow_factor(
                CentralBody::Earth,
                Vec3::new(-r, 0.0, 0.0),
                SUN,
                ShadowModel::Conical
            ),
            earth_shadow_factor(Vec3::new(-r, 0.0, 0.0), SUN, ShadowModel::Conical)
        );
        assert_eq!(
            central_shadow_factor(CentralBody::Sun, Vec3::X, SUN, ShadowModel::Conical),
            1.0
        );
    }

    #[test]
    fn overlap_of_tangent_circles() {
        assert!(circle_overlap_area(0.3, 0.2, 0.5).abs() < 1e-7);
        let area = circle_overlap_area(0.5, 0.2, 0.3);
        assert!((area - core::f64::consts::PI * 0.04).abs() < 1e-7);
    }
}
