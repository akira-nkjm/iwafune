//! 固定軸 X → Y → Z の回転角 ⇄ クォータニオン（部品の配置の姿勢を角度で見せる・打つ）。**角度 ⇄ 姿勢の換算はここだけ**。
//!
//! 決め方: 親の座標軸のまわりに X → Y → Z の順に回す（固定軸の XYZ）。`q = qz ⊗ qy ⊗ qx`、行列は `R = Rz·Ry·Rx`。
//! 逆向きの角度は X・Z が (−π, π]、Y が [−π/2, π/2]。Y が ±π/2（ジンバルロック）のときは X を 0 にして Z に寄せる。

use super::{Quat, Vec3};
use core::f64::consts::PI;

/// Y の余弦（`hypot(R00, R10)`）がこれより小さければジンバルロックとみなす。
const GIMBAL_LOCK_COS: f64 = 1e-9;

impl Quat {
    /// 固定軸 X → Y → Z の回転角 `[x, y, z]` [rad] から作る（`q = qz ⊗ qy ⊗ qx`。スカラー部を非負にそろえる）。
    pub fn from_fixed_xyz(angles_rad: [f64; 3]) -> Self {
        let [x, y, z] = angles_rad;
        let q = Self::from_axis_angle(Vec3::Z, z)
            .mul(Self::from_axis_angle(Vec3::Y, y))
            .mul(Self::from_axis_angle(Vec3::X, x));
        q.with_nonnegative_w()
    }

    /// 固定軸 X → Y → Z の回転角 `[x, y, z]` [rad]（X・Z は (−π, π]、Y は [−π/2, π/2]。ジンバルロックでは X = 0）。
    pub fn to_fixed_xyz(self) -> [f64; 3] {
        let m = self.to_matrix().m;
        let cos_y = m[0][0].hypot(m[1][0]);
        let y = (-m[2][0]).atan2(cos_y);
        let (x, z) = if cos_y < GIMBAL_LOCK_COS {
            // R01 = −sin(z ∓ x)、R11 = cos(z ∓ x)。X を 0 にすれば Y の符号によらず z = atan2(−R01, R11)
            (0.0, (-m[0][1]).atan2(m[1][1]))
        } else {
            (m[2][1].atan2(m[2][2]), m[1][0].atan2(m[0][0]))
        };
        [half_open(x), y + 0.0, half_open(z)]
    }
}

/// 角度を (−π, π] にそろえる（atan2 の −π は π に。−0 は 0 に）。
fn half_open(a: f64) -> f64 {
    if a <= -PI { a + 2.0 * PI } else { a + 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::{FRAC_1_SQRT_2, FRAC_PI_2};

    fn deg(a: [f64; 3]) -> [f64; 3] {
        a.map(f64::to_radians)
    }

    fn close(a: [f64; 3], b: [f64; 3]) {
        for i in 0..3 {
            assert!((a[i] - b[i]).abs() < 1e-12, "{a:?} != {b:?}");
        }
    }

    /// 手計算: Y まわり 90° は (cos 45°, 0, sin 45°, 0)。デモの SS_PX の姿勢 (0.7071, 0, 0.7071, 0) は X 0°・Y 90°・Z 0°。
    #[test]
    #[allow(clippy::approx_constant)] // ファイルに 6 桁で書く値（0.707107）を確かめる
    fn y_quarter_turn_matches_hand_calculation() {
        let q = Quat::from_fixed_xyz(deg([0.0, 90.0, 0.0]));
        let want = [FRAC_1_SQRT_2, 0.0, FRAC_1_SQRT_2, 0.0];
        for (a, b) in q.to_wxyz().iter().zip(want) {
            assert!((a - b).abs() < 1e-15, "{q:?}");
        }
        close(Quat::from_wxyz(want).to_fixed_xyz(), [0.0, FRAC_PI_2, 0.0]);
        // ファイルに 6 桁で書いた値も同じ角度になる
        let rounded = Quat::from_wxyz([0.707107, 0.0, 0.707107, 0.0]).normalized();
        close(rounded.to_fixed_xyz(), [0.0, FRAC_PI_2, 0.0]);
    }

    /// 手計算: X まわり 90° は (cos 45°, sin 45°, 0, 0)、Z まわり −90° は (cos 45°, 0, 0, −sin 45°)。
    /// 固定軸の順: X 90° → Z 90° で機体の +Y は +Z へ、+X は +Y へ。
    #[test]
    fn single_axes_and_order_match_hand_calculation() {
        let qx = Quat::from_fixed_xyz(deg([90.0, 0.0, 0.0])).to_wxyz();
        assert!((qx[0] - FRAC_1_SQRT_2).abs() < 1e-15 && (qx[1] - FRAC_1_SQRT_2).abs() < 1e-15);
        let qz = Quat::from_fixed_xyz(deg([0.0, 0.0, -90.0])).to_wxyz();
        assert!((qz[0] - FRAC_1_SQRT_2).abs() < 1e-15 && (qz[3] + FRAC_1_SQRT_2).abs() < 1e-15);
        let q = Quat::from_fixed_xyz(deg([90.0, 0.0, 90.0]));
        let (y, x) = (q.rotate(Vec3::Y), q.rotate(Vec3::X));
        assert!(
            (y - Vec3::Z).norm() < 1e-15 && (x - Vec3::Y).norm() < 1e-15,
            "{y:?} {x:?}"
        );
    }

    /// 角度 → クォータニオン → 角度が範囲の中で戻る（X・Z の ±180° は 180° に、スカラー部は非負）。
    #[test]
    fn angles_round_trip_within_ranges() {
        for a in [
            [10.0, 20.0, 30.0],
            [-170.0, 80.0, 179.0],
            [45.0, -89.0, -120.0],
            [180.0, 0.0, -45.0],
            [0.0, 0.0, 0.0],
        ] {
            let q = Quat::from_fixed_xyz(deg(a));
            assert!(q.w >= 0.0, "{q:?}");
            close(q.to_fixed_xyz(), deg(a));
        }
        close(
            Quat::from_fixed_xyz(deg([-180.0, 0.0, 0.0])).to_fixed_xyz(),
            deg([180.0, 0.0, 0.0]),
        );
        // Y の範囲の外（120°）は同じ回転の範囲の中の角度（X 180°・Y 60°・Z 180°）になる
        let q = Quat::from_fixed_xyz(deg([0.0, 120.0, 0.0]));
        close(q.to_fixed_xyz(), deg([180.0, 60.0, 180.0]));
    }

    /// ジンバルロック（Y = ±90°）では X を 0 にして Z に寄せる（同じ回転のまま）。
    #[test]
    fn gimbal_lock_folds_x_into_z() {
        for (a, want) in [
            ([30.0, 90.0, 10.0], [0.0, 90.0, -20.0]),
            ([30.0, -90.0, 10.0], [0.0, -90.0, 40.0]),
        ] {
            let q = Quat::from_fixed_xyz(deg(a));
            let back = q.to_fixed_xyz();
            close(back, deg(want));
            assert!(q.angle_to(Quat::from_fixed_xyz(back)) < 1e-12);
        }
    }

    /// 任意の姿勢も、角度にして戻すと同じ回転になる。
    #[test]
    fn any_rotation_survives_the_round_trip() {
        let q = Quat::from_axis_angle(Vec3::new(1.0, -2.0, 0.5), 2.7);
        assert!(q.angle_to(Quat::from_fixed_xyz(q.to_fixed_xyz())) < 1e-12);
    }
}
