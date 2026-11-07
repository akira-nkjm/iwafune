//! 3×3 行列（行優先）。

use super::Vec3;
use core::ops::Add;
use serde::{Deserialize, Serialize};

/// 3×3 行列（行優先）。慣性テンソルや回転行列に使う。
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Mat3 {
    pub m: [[f64; 3]; 3],
}

impl Mat3 {
    pub const IDENTITY: Self = Self {
        m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    };
    pub fn mul_vec(&self, v: Vec3) -> Vec3 {
        let m = &self.m;
        Vec3::new(
            m[0][0] * v.x + m[0][1] * v.y + m[0][2] * v.z,
            m[1][0] * v.x + m[1][1] * v.y + m[1][2] * v.z,
            m[2][0] * v.x + m[2][1] * v.y + m[2][2] * v.z,
        )
    }
    pub fn transpose(&self) -> Self {
        let m = &self.m;
        Self {
            m: [
                [m[0][0], m[1][0], m[2][0]],
                [m[0][1], m[1][1], m[2][1]],
                [m[0][2], m[1][2], m[2][2]],
            ],
        }
    }
    /// 3 つの列ベクトルから作る。
    pub fn from_columns(c0: Vec3, c1: Vec3, c2: Vec3) -> Self {
        Self {
            m: [[c0.x, c1.x, c2.x], [c0.y, c1.y, c2.y], [c0.z, c1.z, c2.z]],
        }
    }
    /// 3 つの行ベクトルから作る。
    pub fn from_rows(r0: Vec3, r1: Vec3, r2: Vec3) -> Self {
        Self {
            m: [r0.to_array(), r1.to_array(), r2.to_array()],
        }
    }
    /// 対角行列。
    pub fn diagonal(d: Vec3) -> Self {
        Self {
            m: [[d.x, 0.0, 0.0], [0.0, d.y, 0.0], [0.0, 0.0, d.z]],
        }
    }
    /// 第 `i` 列。
    pub fn column(&self, i: usize) -> Vec3 {
        Vec3::new(self.m[0][i], self.m[1][i], self.m[2][i])
    }
    /// 行列の積 `self × o`。
    #[allow(clippy::should_implement_trait)]
    pub fn mul(&self, o: &Mat3) -> Mat3 {
        let mut out = [[0.0; 3]; 3];
        for (i, row) in out.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = (0..3).map(|k| self.m[i][k] * o.m[k][j]).sum();
            }
        }
        Mat3 { m: out }
    }
    /// x 軸まわりにベクトルを `angle_rad` 回す回転行列（右手系、正の向き）。**軸まわりの回転行列はここだけ。**
    pub fn rotation_x(angle_rad: f64) -> Mat3 {
        let (s, c) = angle_rad.sin_cos();
        Mat3 {
            m: [[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]],
        }
    }
    /// y 軸まわりにベクトルを `angle_rad` 回す回転行列。
    pub fn rotation_y(angle_rad: f64) -> Mat3 {
        let (s, c) = angle_rad.sin_cos();
        Mat3 {
            m: [[c, 0.0, s], [0.0, 1.0, 0.0], [-s, 0.0, c]],
        }
    }
    /// z 軸まわりにベクトルを `angle_rad` 回す回転行列。
    pub fn rotation_z(angle_rad: f64) -> Mat3 {
        let (s, c) = angle_rad.sin_cos();
        Mat3 {
            m: [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]],
        }
    }
    /// 外積行列 `a bᵀ`。
    pub fn outer(a: Vec3, b: Vec3) -> Mat3 {
        Mat3 {
            m: [
                b.scale(a.x).to_array(),
                b.scale(a.y).to_array(),
                b.scale(a.z).to_array(),
            ],
        }
    }
    /// すべての要素を `k` 倍する。
    pub fn scale(&self, k: f64) -> Mat3 {
        let mut out = self.m;
        out.iter_mut().flatten().for_each(|v| *v *= k);
        Mat3 { m: out }
    }
    /// 行列式。
    pub fn determinant(&self) -> f64 {
        let m = &self.m;
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    }
    /// 逆行列。特異（|det| がほぼ 0）なら `None`。
    pub fn inverse(&self) -> Option<Mat3> {
        let det = self.determinant();
        let scale = self.m.iter().flatten().fold(0.0_f64, |a, v| a.max(v.abs()));
        if !det.is_finite() || det.abs() <= 1e-300_f64.max(1e-14 * scale.powi(3)) {
            return None;
        }
        let m = &self.m;
        let cof = |r0: usize, r1: usize, c0: usize, c1: usize| {
            m[r0][c0] * m[r1][c1] - m[r0][c1] * m[r1][c0]
        };
        let adj = [
            [cof(1, 2, 1, 2), -cof(0, 2, 1, 2), cof(0, 1, 1, 2)],
            [-cof(1, 2, 0, 2), cof(0, 2, 0, 2), -cof(0, 1, 0, 2)],
            [cof(1, 2, 0, 1), -cof(0, 2, 0, 1), cof(0, 1, 0, 1)],
        ];
        let mut out = [[0.0; 3]; 3];
        for (i, row) in out.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = adj[i][j] / det;
            }
        }
        Some(Mat3 { m: out })
    }
}

impl Add for Mat3 {
    type Output = Mat3;
    fn add(self, o: Mat3) -> Mat3 {
        let mut out = self.m;
        for (row, orow) in out.iter_mut().zip(o.m.iter()) {
            for (v, ov) in row.iter_mut().zip(orow.iter()) {
                *v += ov;
            }
        }
        Mat3 { m: out }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3, tol: f64) {
        assert!((a - b).norm() < tol, "{a:?} != {b:?}");
    }

    #[test]
    fn matrix_basics() {
        let m = Mat3::from_columns(Vec3::Y, Vec3::Z, Vec3::X);
        close(m.mul_vec(Vec3::X), Vec3::Y, 1e-15);
        close(m.transpose().mul_vec(Vec3::Y), Vec3::X, 1e-15);
        let a = Mat3 {
            m: [[2.0, 1.0, 0.0], [0.0, 3.0, 1.0], [1.0, 0.0, 4.0]],
        };
        let inv = a.inverse().expect("正則");
        let id = a.mul(&inv);
        for i in 0..3 {
            for j in 0..3 {
                let e = if i == j { 1.0 } else { 0.0 };
                assert!((id.m[i][j] - e).abs() < 1e-14);
            }
        }
        assert!(Mat3::default().inverse().is_none());
    }
}

#[cfg(test)]
mod rotation_tests {
    use super::*;
    use core::f64::consts::FRAC_PI_2;

    #[test]
    fn axis_rotations_are_right_handed() {
        let close = |a: Vec3, b: Vec3| {
            assert!((a.x - b.x).abs() + (a.y - b.y).abs() + (a.z - b.z).abs() < 1e-15)
        };
        close(Mat3::rotation_z(FRAC_PI_2).mul_vec(Vec3::X), Vec3::Y);
        close(Mat3::rotation_x(FRAC_PI_2).mul_vec(Vec3::Y), Vec3::Z);
        close(Mat3::rotation_y(FRAC_PI_2).mul_vec(Vec3::Z), Vec3::X);
        assert!((Mat3::rotation_y(0.7).determinant() - 1.0).abs() < 1e-15);
        close(Vec3::from_spherical(FRAC_PI_2, 0.0), Vec3::Y);
        close(Vec3::from_spherical(0.3, FRAC_PI_2), Vec3::Z);
    }
}
