//! 単位クォータニオン（スカラー部が先）。

use super::{Mat3, Vec3};
use serde::{Deserialize, Serialize};

/// 単位クォータニオン（スカラー部 w が先）。`rotate` は「この姿勢の座標系 → 親の座標系」への回転。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Quat {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Quat {
    pub const IDENTITY: Self = Self {
        w: 1.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    /// `[w, x, y, z]` から作る（正規化はしない）。
    pub const fn from_wxyz(q: [f64; 4]) -> Self {
        Self {
            w: q[0],
            x: q[1],
            y: q[2],
            z: q[3],
        }
    }
    /// `[w, x, y, z]` にする。
    pub const fn to_wxyz(self) -> [f64; 4] {
        [self.w, self.x, self.y, self.z]
    }
    /// ベクトル部。
    pub fn vector(self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }
    pub fn normalized(self) -> Self {
        let n = (self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        if n > 0.0 && n.is_finite() {
            Self {
                w: self.w / n,
                x: self.x / n,
                y: self.y / n,
                z: self.z / n,
            }
        } else {
            Self::IDENTITY
        }
    }
    pub fn conjugate(self) -> Self {
        Self {
            w: self.w,
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
    /// クォータニオンの積 `self ⊗ o`（先に `o`、次に `self` の回転）。
    #[allow(clippy::should_implement_trait)]
    pub fn mul(self, o: Self) -> Self {
        Self {
            w: self.w * o.w - self.x * o.x - self.y * o.y - self.z * o.z,
            x: self.w * o.x + self.x * o.w + self.y * o.z - self.z * o.y,
            y: self.w * o.y - self.x * o.z + self.y * o.w + self.z * o.x,
            z: self.w * o.z + self.x * o.y - self.y * o.x + self.z * o.w,
        }
    }
    pub fn rotate(self, v: Vec3) -> Vec3 {
        // v' = v + 2w (q×v) + 2 q×(q×v)
        let q = self.vector();
        let t = q.cross(v).scale(2.0);
        v + t.scale(self.w) + q.cross(t)
    }
    /// 逆回転（親の座標系 → この姿勢の座標系）。
    pub fn rotate_inverse(self, v: Vec3) -> Vec3 {
        self.conjugate().rotate(v)
    }
    pub fn from_axis_angle(axis: Vec3, angle_rad: f64) -> Self {
        let Some(u) = axis.normalized() else {
            return Self::IDENTITY;
        };
        let (s, c) = (0.5 * angle_rad).sin_cos();
        Self {
            w: c,
            x: u.x * s,
            y: u.y * s,
            z: u.z * s,
        }
    }
    /// 回転行列（この姿勢の座標系 → 親の座標系）から作る。
    pub fn from_matrix(m: &Mat3) -> Self {
        // Shepperd の方法（対角要素の最大のものから求めて数値誤差を抑える）。
        let a = &m.m;
        let trace = a[0][0] + a[1][1] + a[2][2];
        let q = if trace > a[0][0].max(a[1][1]).max(a[2][2]) {
            let s = (1.0 + trace).sqrt() * 2.0;
            Self {
                w: 0.25 * s,
                x: (a[2][1] - a[1][2]) / s,
                y: (a[0][2] - a[2][0]) / s,
                z: (a[1][0] - a[0][1]) / s,
            }
        } else if a[0][0] >= a[1][1] && a[0][0] >= a[2][2] {
            let s = (1.0 + a[0][0] - a[1][1] - a[2][2]).sqrt() * 2.0;
            Self {
                w: (a[2][1] - a[1][2]) / s,
                x: 0.25 * s,
                y: (a[0][1] + a[1][0]) / s,
                z: (a[0][2] + a[2][0]) / s,
            }
        } else if a[1][1] >= a[2][2] {
            let s = (1.0 + a[1][1] - a[0][0] - a[2][2]).sqrt() * 2.0;
            Self {
                w: (a[0][2] - a[2][0]) / s,
                x: (a[0][1] + a[1][0]) / s,
                y: 0.25 * s,
                z: (a[1][2] + a[2][1]) / s,
            }
        } else {
            let s = (1.0 + a[2][2] - a[0][0] - a[1][1]).sqrt() * 2.0;
            Self {
                w: (a[1][0] - a[0][1]) / s,
                x: (a[0][2] + a[2][0]) / s,
                y: (a[1][2] + a[2][1]) / s,
                z: 0.25 * s,
            }
        };
        let q = q.normalized();
        // スカラー部を非負にそろえる（同じ回転の 2 つの表現のうち片方に決める）。
        if q.w < 0.0 {
            Self {
                w: -q.w,
                x: -q.x,
                y: -q.y,
                z: -q.z,
            }
        } else {
            q
        }
    }
    pub fn to_matrix(self) -> Mat3 {
        let Quat { w, x, y, z } = self.normalized();
        Mat3 {
            m: [
                [
                    1.0 - 2.0 * (y * y + z * z),
                    2.0 * (x * y - w * z),
                    2.0 * (x * z + w * y),
                ],
                [
                    2.0 * (x * y + w * z),
                    1.0 - 2.0 * (x * x + z * z),
                    2.0 * (y * z - w * x),
                ],
                [
                    2.0 * (x * z - w * y),
                    2.0 * (y * z + w * x),
                    1.0 - 2.0 * (x * x + y * y),
                ],
            ],
        }
    }
    /// 2 つの姿勢の間の回転角 [rad]（0〜π）。
    pub fn angle_to(self, o: Self) -> f64 {
        let d = self.conjugate().mul(o);
        2.0 * d.vector().norm().atan2(d.w.abs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::FRAC_PI_2;

    fn close(a: Vec3, b: Vec3, tol: f64) {
        assert!((a - b).norm() < tol, "{a:?} != {b:?}");
    }

    #[test]
    fn quaternion_rotation_matches_matrix() {
        let q = Quat::from_axis_angle(Vec3::Z, FRAC_PI_2);
        close(q.rotate(Vec3::X), Vec3::Y, 1e-15);
        close(q.to_matrix().mul_vec(Vec3::X), Vec3::Y, 1e-15);
        let q2 = Quat::from_axis_angle(Vec3::new(1.0, -2.0, 0.5), 2.7);
        let back = Quat::from_matrix(&q2.to_matrix());
        assert!(q2.angle_to(back) < 1e-12);
        let v = Vec3::new(0.3, -1.2, 2.0);
        close(q2.rotate_inverse(q2.rotate(v)), v, 1e-14);
        // 積の順序: 先に o、次に self。
        let a = Quat::from_axis_angle(Vec3::Z, FRAC_PI_2);
        let b = Quat::from_axis_angle(Vec3::X, FRAC_PI_2);
        close(a.mul(b).rotate(Vec3::Y), a.rotate(b.rotate(Vec3::Y)), 1e-15);
    }

    #[test]
    fn wxyz_round_trip() {
        let q = Quat::from_wxyz([0.5, 0.5, -0.5, 0.5]);
        assert_eq!(q.to_wxyz(), [0.5, 0.5, -0.5, 0.5]);
    }

    #[test]
    fn from_matrix_handles_half_turn() {
        for axis in [Vec3::X, Vec3::Y, Vec3::Z, Vec3::new(1.0, 1.0, 0.0)] {
            let q = Quat::from_axis_angle(axis, core::f64::consts::PI);
            let back = Quat::from_matrix(&q.to_matrix());
            assert!(q.angle_to(back) < 1e-12, "{axis:?}");
        }
    }
}
