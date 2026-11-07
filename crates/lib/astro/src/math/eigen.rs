//! 対称 3×3 行列の固有値分解（巡回 Jacobi 法）。慣性テンソルの主慣性モーメント・主軸に使う。
// 3×3 の添字を明示したほうが式（Jacobi 回転）と対応が取りやすいので、範囲ループを許す。
#![allow(clippy::needless_range_loop)]

use super::{Mat3, Vec3};

impl Mat3 {
    /// 対称行列の固有値（昇順）と固有ベクトル（列。単位ベクトルで右手系）。
    ///
    /// 巡回 Jacobi 法で非対角成分を消す。入力は対称とみなし、上三角と下三角の平均を使う。
    pub fn symmetric_eigen(&self) -> (Vec3, Mat3) {
        let mut a = self.m;
        for i in 0..3 {
            for j in (i + 1)..3 {
                let s = 0.5 * (a[i][j] + a[j][i]);
                a[i][j] = s;
                a[j][i] = s;
            }
        }
        let mut v = Mat3::IDENTITY.m;
        let scale = a.iter().flatten().fold(0.0_f64, |m, x| m.max(x.abs()));
        for _sweep in 0..50 {
            let off = a[0][1].abs() + a[0][2].abs() + a[1][2].abs();
            if off <= f64::EPSILON * scale * 1e-2 || off == 0.0 {
                break;
            }
            for (p, q) in [(0, 1), (0, 2), (1, 2)] {
                rotate(&mut a, &mut v, p, q);
            }
        }
        sorted(a, v)
    }
}

/// (p, q) 成分を消す Jacobi 回転を `a` に施し、`v` に累積する。
fn rotate(a: &mut [[f64; 3]; 3], v: &mut [[f64; 3]; 3], p: usize, q: usize) {
    if a[p][q] == 0.0 {
        return;
    }
    let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
    let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
    let t = if theta == 0.0 { 1.0 } else { t };
    let c = 1.0 / (t * t + 1.0).sqrt();
    let s = t * c;
    for k in 0..3 {
        let (akp, akq) = (a[k][p], a[k][q]);
        a[k][p] = c * akp - s * akq;
        a[k][q] = s * akp + c * akq;
    }
    for k in 0..3 {
        let (apk, aqk) = (a[p][k], a[q][k]);
        a[p][k] = c * apk - s * aqk;
        a[q][k] = s * apk + c * aqk;
    }
    for row in v.iter_mut() {
        let (vp, vq) = (row[p], row[q]);
        row[p] = c * vp - s * vq;
        row[q] = s * vp + c * vq;
    }
}

/// 固有値を昇順に並べ、固有ベクトルの列を右手系にそろえる。
fn sorted(a: [[f64; 3]; 3], v: [[f64; 3]; 3]) -> (Vec3, Mat3) {
    let mut idx = [0usize, 1, 2];
    idx.sort_by(|&i, &j| a[i][i].total_cmp(&a[j][j]));
    let col = |i: usize| Vec3::new(v[0][i], v[1][i], v[2][i]);
    let (c0, c1) = (col(idx[0]), col(idx[1]));
    let c2 = c0.cross(c1);
    (
        Vec3::new(a[idx[0]][idx[0]], a[idx[1]][idx[1]], a[idx[2]][idx[2]]),
        Mat3::from_columns(c0, c1, c2),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Quat;

    #[test]
    fn diagonal_tensor() {
        let m = Mat3::diagonal(Vec3::new(3.0, 1.0, 2.0));
        let (w, v) = m.symmetric_eigen();
        assert_eq!(w, Vec3::new(1.0, 2.0, 3.0));
        assert!((v.column(0).dot(Vec3::Y)).abs() > 1.0 - 1e-15);
        assert!((v.determinant() - 1.0).abs() < 1e-14);
    }

    #[test]
    fn rotated_diagonal_tensor_recovers_axes() {
        let d = Vec3::new(0.05, 0.12, 0.31);
        let r = Quat::from_axis_angle(Vec3::new(0.3, -1.0, 0.7), 1.1).to_matrix();
        // I = R D Rᵀ
        let inertia = r.mul(&Mat3::diagonal(d)).mul(&r.transpose());
        let (w, v) = inertia.symmetric_eigen();
        assert!((w - d).norm() < 1e-14, "{w:?}");
        for i in 0..3 {
            // 固有ベクトルは R の列と（符号を除いて）一致し、I v = λ v
            assert!((v.column(i).dot(r.column(i)).abs() - 1.0).abs() < 1e-12);
            let iv = inertia.mul_vec(v.column(i));
            assert!((iv - v.column(i) * w.to_array()[i]).norm() < 1e-14);
        }
        assert!((v.determinant() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn repeated_eigenvalues_are_handled() {
        let m = Mat3 {
            m: [[2.0, 1.0, 0.0], [1.0, 2.0, 0.0], [0.0, 0.0, 3.0]],
        };
        let (w, v) = m.symmetric_eigen();
        assert!((w - Vec3::new(1.0, 3.0, 3.0)).norm() < 1e-14);
        let back = v.mul(&Mat3::diagonal(w)).mul(&v.transpose());
        for i in 0..3 {
            for j in 0..3 {
                assert!((back.m[i][j] - m.m[i][j]).abs() < 1e-13);
            }
        }
    }
}
