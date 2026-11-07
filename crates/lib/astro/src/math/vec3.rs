//! 3 次元ベクトル（成分は SI）と演算子。

use core::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};
use serde::{Deserialize, Serialize};

/// 3 次元ベクトル（成分は SI）。
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    pub const X: Self = Self {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    pub const Y: Self = Self {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    pub const Z: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    pub fn dot(self, o: Self) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    pub fn cross(self, o: Self) -> Self {
        Self::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }
    pub fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }
    /// ノルムの 2 乗。
    pub fn norm_squared(self) -> f64 {
        self.dot(self)
    }
    /// 単位ベクトル。ゼロベクトルなら `None`。
    pub fn normalized(self) -> Option<Self> {
        let n = self.norm();
        if n > 0.0 && n.is_finite() {
            Some(self.scale(1.0 / n))
        } else {
            None
        }
    }
    pub fn scale(self, k: f64) -> Self {
        Self::new(self.x * k, self.y * k, self.z * k)
    }
    /// 2 つのベクトルのなす角 [rad]。
    pub fn angle_to(self, o: Self) -> f64 {
        // atan2(|a×b|, a·b) は 0 と π の近くでも精度を保つ。
        self.cross(o).norm().atan2(self.dot(o))
    }
    /// 成分を配列にする。
    pub const fn to_array(self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
    /// 配列から作る。
    pub const fn from_array(a: [f64; 3]) -> Self {
        Self::new(a[0], a[1], a[2])
    }
    /// 経度（方位角）`lon_rad`・緯度（仰角）`lat_rad` の単位ベクトル（赤経・赤緯、黄経・黄緯などに使う）。
    pub fn from_spherical(lon_rad: f64, lat_rad: f64) -> Self {
        let (sl, cl) = lat_rad.sin_cos();
        let (so, co) = lon_rad.sin_cos();
        Self::new(cl * co, cl * so, sl)
    }
    /// `self` に直交する単位ベクトルを 1 つ返す（ゼロベクトルなら X）。
    pub fn any_orthogonal(self) -> Self {
        let a = Vec3::new(self.x.abs(), self.y.abs(), self.z.abs());
        let helper = if a.x <= a.y && a.x <= a.z {
            Vec3::X
        } else if a.y <= a.z {
            Vec3::Y
        } else {
            Vec3::Z
        };
        self.cross(helper).normalized().unwrap_or(Vec3::X)
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, o: Vec3) {
        *self = *self + o;
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl SubAssign for Vec3 {
    fn sub_assign(&mut self, o: Vec3) {
        *self = *self - o;
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

impl Mul<f64> for Vec3 {
    type Output = Vec3;
    fn mul(self, k: f64) -> Vec3 {
        self.scale(k)
    }
}

impl Mul<Vec3> for f64 {
    type Output = Vec3;
    fn mul(self, v: Vec3) -> Vec3 {
        v.scale(self)
    }
}

impl Div<f64> for Vec3 {
    type Output = Vec3;
    fn div(self, k: f64) -> Vec3 {
        Vec3::new(self.x / k, self.y / k, self.z / k)
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
    fn vector_basics() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(-2.0, 0.5, 4.0);
        assert_eq!(a.dot(b), -2.0 + 1.0 + 12.0);
        close(Vec3::X.cross(Vec3::Y), Vec3::Z, 1e-15);
        assert!((a.norm() - 14.0_f64.sqrt()).abs() < 1e-15);
        assert!(Vec3::ZERO.normalized().is_none());
        assert!((Vec3::X.angle_to(Vec3::Y) - FRAC_PI_2).abs() < 1e-15);
        assert!((a.cross(b).dot(a)).abs() < 1e-12);
        assert!(a.any_orthogonal().dot(a).abs() < 1e-12);
    }
}
