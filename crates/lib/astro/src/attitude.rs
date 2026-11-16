//! 姿勢則（目標姿勢の決め方）→ 機体姿勢。**姿勢則はここだけ。**
//!
//! 2 軸の合わせ込み（TRIAD）: 主軸は厳密に合わせ、副軸は主軸に直交する成分だけを合わせる。
//! 主軸と副軸の目標が平行になる特異な場合（β = ±90° の太陽指向など）は、別の基準へ切り替える。

use crate::math::{Mat3, Quat, Vec3};
use crate::orbit::OrbitState;
use serde::{Deserialize, Serialize};

/// 姿勢則で主軸を省略した場合の機体 +Z 軸。
pub const DEFAULT_PRIMARY_AXIS: Vec3 = Vec3::Z;
/// 姿勢則で副軸（速度方向を含む）を省略した場合の機体 +X 軸。
pub const DEFAULT_SECONDARY_AXIS: Vec3 = Vec3::X;
/// 慣性固定の姿勢則で向きを省略した場合の機体 → 慣性系の回転（恒等 = 機体軸が慣性軸に一致）。
pub const DEFAULT_INERTIAL_ORIENTATION: Quat = Quat::IDENTITY;

/// 姿勢則。軸はすべて機体座標系の単位ベクトル。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "law", rename_all = "snake_case")]
pub enum AttitudeLaw {
    /// `axis` を太陽に向け、`secondary` を軌道面の法線にできるだけ合わせる。
    SunPointing { axis: Vec3, secondary: Vec3 },
    /// `axis` を地心方向に向け、`along_track` を速度方向に合わせる。
    NadirPointing { axis: Vec3, along_track: Vec3 },
    /// 慣性空間に固定。
    Inertial { q_body_to_eci: Quat },
}

/// その時刻の機体姿勢（機体 → ECI の回転）。
pub fn body_to_eci(law: &AttitudeLaw, state: &OrbitState, sun_dir_eci: Vec3) -> Quat {
    match law {
        AttitudeLaw::SunPointing { axis, secondary } => {
            let sun = sun_dir_eci.normalized().unwrap_or(Vec3::X);
            let normal = state.orbit_normal();
            // β ≈ ±90° で法線が太陽と平行なら、速度方向を副軸の目標にする。
            let fallback = state
                .v_eci
                .normalized()
                .unwrap_or_else(|| sun.any_orthogonal());
            let target2 = pick_secondary(sun, normal, fallback);
            align_two_axes(*axis, sun, *secondary, target2)
        }
        AttitudeLaw::NadirPointing { axis, along_track } => {
            let nadir = (-state.r_eci).normalized().unwrap_or(-Vec3::X);
            let fallback = state.orbit_normal().cross(nadir);
            let target2 = pick_secondary(nadir, state.v_eci, fallback);
            align_two_axes(*axis, nadir, *along_track, target2)
        }
        AttitudeLaw::Inertial { q_body_to_eci } => q_body_to_eci.normalized(),
    }
}

/// 主軸の目標 `primary` と平行に近い副軸の目標を避ける。
fn pick_secondary(primary: Vec3, preferred: Vec3, fallback: Vec3) -> Vec3 {
    let ok = |v: Vec3| {
        v.normalized()
            .is_some_and(|u| u.cross(primary).norm() > 1e-6)
    };
    if ok(preferred) {
        preferred
    } else if ok(fallback) {
        fallback
    } else {
        primary.any_orthogonal()
    }
}

/// 機体の主軸 `body1` を `ref1` に厳密に、副軸 `body2` を `ref2` にできるだけ合わせる姿勢（機体 → 基準）。
///
/// 機体側の 2 軸が平行（または 0）のときは副軸に主軸の任意の直交方向を使う。
pub fn align_two_axes(body1: Vec3, ref1: Vec3, body2: Vec3, ref2: Vec3) -> Quat {
    let triad = |a: Vec3, b: Vec3| {
        let x = a.normalized().unwrap_or(Vec3::X);
        let y = x
            .cross(b)
            .normalized()
            .unwrap_or_else(|| x.any_orthogonal());
        let z = x.cross(y);
        Mat3::from_columns(x, y, z)
    };
    let body = triad(body1, body2);
    let reference = triad(ref1, ref2);
    Quat::from_matrix(&reference.mul(&body.transpose()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{MU_EARTH, R_EARTH};

    fn leo_state() -> OrbitState {
        let r = R_EARTH + 550e3;
        let v = (MU_EARTH / r).sqrt();
        // 傾斜 60° の円軌道、昇交点
        let i = 60f64.to_radians();
        OrbitState {
            r_eci: Vec3::new(r, 0.0, 0.0),
            v_eci: Vec3::new(0.0, v * i.cos(), v * i.sin()),
        }
    }

    fn close(a: Vec3, b: Vec3) {
        assert!((a - b).norm() < 1e-9, "{a:?} != {b:?}");
    }

    #[test]
    fn sun_pointing_aligns_axis_and_secondary() {
        let s = leo_state();
        let sun = Vec3::new(0.3, 0.9, -0.2).normalized().expect("nonzero");
        let law = AttitudeLaw::SunPointing {
            axis: Vec3::Z,
            secondary: Vec3::Y,
        };
        let q = body_to_eci(&law, &s, sun);
        close(q.rotate(Vec3::Z), sun);
        // 副軸は主軸に直交したまま、軌道法線に最も近い向き
        let y = q.rotate(Vec3::Y);
        let n = s.orbit_normal();
        let n_perp = (n - sun * n.dot(sun)).normalized().expect("non-parallel");
        close(y, n_perp);
    }

    #[test]
    fn sun_pointing_survives_beta_90() {
        let s = leo_state();
        let law = AttitudeLaw::SunPointing {
            axis: Vec3::X,
            secondary: Vec3::Z,
        };
        let q = body_to_eci(&law, &s, s.orbit_normal());
        close(q.rotate(Vec3::X), s.orbit_normal());
        assert!(q.w.is_finite());
    }

    #[test]
    fn nadir_pointing_aligns_with_velocity() {
        let s = leo_state();
        let law = AttitudeLaw::NadirPointing {
            axis: Vec3::Z,
            along_track: Vec3::X,
        };
        let q = body_to_eci(&law, &s, Vec3::X);
        close(q.rotate(Vec3::Z), -s.r_eci.normalized().expect("nonzero"));
        close(q.rotate(Vec3::X), s.v_eci.normalized().expect("nonzero"));
    }

    #[test]
    fn inertial_returns_normalized_quaternion() {
        let q = Quat {
            w: 2.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let out = body_to_eci(
            &AttitudeLaw::Inertial { q_body_to_eci: q },
            &leo_state(),
            Vec3::X,
        );
        assert_eq!(out, Quat::IDENTITY);
    }
}
