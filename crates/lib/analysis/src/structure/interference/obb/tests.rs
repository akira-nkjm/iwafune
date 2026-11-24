//! 箱どうしの分離量のテスト（手計算との比較）。

use super::*;
use std::f64::consts::FRAC_1_SQRT_2;

fn cube(center: [f64; 3]) -> OrientedBox {
    OrientedBox::from_size(
        Vec3::new(center[0], center[1], center[2]),
        Quat::IDENTITY,
        Vec3::new(1.0, 1.0, 1.0),
    )
}

fn about_z_45(center: [f64; 3]) -> OrientedBox {
    OrientedBox::from_size(
        Vec3::new(center[0], center[1], center[2]),
        Quat::from_axis_angle(Vec3::Z, std::f64::consts::FRAC_PI_4),
        Vec3::new(1.0, 1.0, 1.0),
    )
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-12
}

fn close_vec(a: Vec3, b: Vec3) -> bool {
    close(a.x, b.x) && close(a.y, b.y) && close(a.z, b.z)
}

/// 一辺 1 m の立方体 2 つ。中心の距離 0.9 → 深さ 0.1（x 方向）、1.0 → 接している（0）、1.1 → 隙間 0.1。
#[test]
fn axis_aligned_cubes() {
    let s = separation(&cube([0.0; 3]), &cube([0.9, 0.0, 0.0]));
    assert!(close(s.distance, -0.1), "{s:?}");
    assert!(close(s.depth(), 0.1));
    assert!(close_vec(s.axis, Vec3::X));

    let touching = separation(&cube([0.0; 3]), &cube([1.0, 0.0, 0.0]));
    assert!(close(touching.distance, 0.0));
    assert!(close(touching.depth(), 0.0));

    let apart = separation(&cube([0.0; 3]), &cube([0.0, 1.1, 0.0]));
    assert!(close(apart.distance, 0.1));
    assert!(close_vec(apart.axis, Vec3::Y));
    assert!(close(apart.depth(), 0.0));
}

/// 軸が負の側（`b` が `a` の −x）にあるとき、軸は `a` から `b` へ向く（−x）。
#[test]
fn axis_points_from_a_to_b() {
    let s = separation(&cube([0.0; 3]), &cube([-0.9, 0.0, 0.0]));
    assert!(close_vec(s.axis, Vec3::new(-1.0, 0.0, 0.0)), "{s:?}");
}

/// 45° 回した立方体は x 方向に √2/2 の半幅を持つ。隣の軸平行な立方体（半幅 1/2）との分離距離は 1/2 + √2/2 ≈ 1.20711。
/// 中心距離 1.2 → 深さ 1.20711 − 1.2 = 0.00711（x 方向）、1.25 → 隙間 1.25 − 1.20711 = 0.04289（x 方向）。
/// 回した立方体の面の軸（1.2 のとき重なりは 0.5·√2 + 0.5 − 1.2·cos45° = 0.3586）は x より深いので、x が最良になる。
#[test]
fn rotated_cube_next_to_axis_aligned_cube() {
    let reach = 0.5 + 0.5 * 2.0_f64.sqrt();
    assert!(close(reach, 0.5 + FRAC_1_SQRT_2));
    let inside = separation(&cube([0.0; 3]), &about_z_45([1.2, 0.0, 0.0]));
    assert!(close(inside.distance, 1.2 - reach), "{inside:?}");
    assert!(close(inside.depth(), reach - 1.2));
    assert!(close_vec(inside.axis, Vec3::X));

    let outside = separation(&cube([0.0; 3]), &about_z_45([1.25, 0.0, 0.0]));
    assert!(close(outside.distance, 1.25 - reach), "{outside:?}");
    assert!(close_vec(outside.axis, Vec3::X));
}

/// 細い棒（辺 2 × 0.1 × 0.1 m）2 本が X の字に交差する（A は x 方向、B は y 方向）。どちらも自分の軸まわりに 45° ひねってあり、
/// 面の法線 6 本のどれも z 軸ではない。
///
/// 手計算（h = 0.05、c = s = √2/2）: A の軸 a1 = x、a2 = (0, c, s)、a3 = (0, −s, c)。B の軸 b1 = (c, 0, −s)、b2 = y、b3 = (s, 0, c)。
/// 外積の軸 a1 × b2 = z で、z への射影の半径は A: h(|s| + |c|) = h√2、B: h√2。B を z に dz だけずらすと、
/// 隙間 = dz − 2h√2 = dz − 0.141421。面の軸（a2 など）は 0.8071 − 0.7071·dz > 0 のままで、dz < 1.14 の間は離れない。
/// したがって dz = 0.13 は深さ 0.011421（z）、dz = 0.15 は隙間 0.008579（z）。面の軸 6 本だけだと、どちらも「深さ 0.70 の重なり」になってしまう。
fn crossed_bars(dz: f64) -> (OrientedBox, OrientedBox) {
    let quarter = std::f64::consts::FRAC_PI_4;
    let a = OrientedBox::from_size(
        Vec3::ZERO,
        Quat::from_axis_angle(Vec3::X, quarter),
        Vec3::new(2.0, 0.1, 0.1),
    );
    let b = OrientedBox::from_size(
        Vec3::new(0.0, 0.0, dz),
        Quat::from_axis_angle(Vec3::Y, quarter),
        Vec3::new(0.1, 2.0, 0.1),
    );
    (a, b)
}

#[test]
fn crossed_bars_need_the_edge_axes() {
    let edge = 2.0 * 0.05 * 2.0_f64.sqrt();
    let (a, b) = crossed_bars(0.13);
    let s = separation(&a, &b);
    assert!(close(s.distance, 0.13 - edge), "{s:?}");
    assert!(close_vec(s.axis, Vec3::Z), "{s:?}");
    // 面の法線 6 本だけでは、深さ 0.70 の重なりと誤って判定する
    let faces_only = search(&a, &b, false);
    assert!(faces_only.distance < -0.5, "{faces_only:?}");

    let (a, b) = crossed_bars(0.15);
    let s = separation(&a, &b);
    assert!(close(s.distance, 0.15 - edge), "{s:?}");
    assert!(close_vec(s.axis, Vec3::Z));
    assert!(search(&a, &b, false).distance < 0.0);
}

/// 辺どうしが交差する分の最小の侵入も、15 本の最大と一致する（上の dz = 0.13 は深さ 0.011421）。
#[test]
fn crossed_bars_depth_value() {
    let (a, b) = crossed_bars(0.13);
    assert!((separation(&a, &b).depth() - 0.011_421_356).abs() < 1e-8);
}

/// 大きな箱（辺 1）の中に小さな箱（辺 0.2、中心が +x に 0.2）が丸ごと入っている。
/// 外へ出るのに要る移動は、+x 側の面まで: 0.5 + 0.1 − 0.2 = 0.4（y・z 方向は 0.6）。軸は +x。
#[test]
fn contained_box_depth_is_the_distance_to_exit() {
    let big = cube([0.0; 3]);
    let small = OrientedBox::from_size(
        Vec3::new(0.2, 0.0, 0.0),
        Quat::IDENTITY,
        Vec3::new(0.2, 0.2, 0.2),
    );
    let s = separation(&big, &small);
    assert!(close(s.depth(), 0.4), "{s:?}");
    assert!(close_vec(s.axis, Vec3::X));
}

fn general(seed: f64) -> (OrientedBox, OrientedBox) {
    let a = OrientedBox::from_size(
        Vec3::new(0.1, -0.2, 0.05),
        Quat::from_axis_angle(Vec3::new(1.0, 2.0, 3.0), 0.7 + seed),
        Vec3::new(0.6, 0.3, 0.2),
    );
    let b = OrientedBox::from_size(
        Vec3::new(0.35, 0.05, 0.1),
        Quat::from_axis_angle(Vec3::new(-1.0, 0.5, 2.0), 1.9 - seed),
        Vec3::new(0.4, 0.5, 0.25),
    );
    (a, b)
}

/// 組の順を入れ替えても深さは同じで、軸は逆向きになる。
#[test]
fn swapping_the_pair_keeps_the_depth_and_flips_the_axis() {
    for seed in [0.0, 0.3, 1.1] {
        let (a, b) = general(seed);
        let (ab, ba) = (separation(&a, &b), separation(&b, &a));
        assert!(close(ab.distance, ba.distance), "{ab:?} {ba:?}");
        assert!(close_vec(ab.axis, ba.axis.scale(-1.0)), "{ab:?} {ba:?}");
    }
}

/// 2 つの箱に同じ剛体運動（回転 R と並進）を加えても、分離量は同じで、軸は R で回る。
#[test]
fn rigid_motion_rotates_the_axis_and_keeps_the_distance() {
    let r = Quat::from_axis_angle(Vec3::new(0.3, -1.0, 0.8), 1.234);
    let t = Vec3::new(5.0, -3.0, 2.0);
    for seed in [0.0, 0.3, 1.1] {
        let (a, b) = general(seed);
        let moved = |o: &OrientedBox| OrientedBox {
            center: r.rotate(o.center) + t,
            orientation: r.mul(o.orientation).normalized(),
            half_extent: o.half_extent,
        };
        let before = separation(&a, &b);
        let after = separation(&moved(&a), &moved(&b));
        assert!(
            close(before.distance, after.distance),
            "{before:?} {after:?}"
        );
        let rotated = r.rotate(before.axis);
        assert!(close_vec(rotated, after.axis), "{rotated:?} {after:?}");
    }
}

/// 深さの値は、その軸方向に `b` を深さ + ε だけ動かすと離れ、深さ − ε では離れない（最小の並進であること）。
#[test]
fn moving_along_the_axis_by_the_depth_separates_the_boxes() {
    let (a, b) = general(0.0);
    let s = separation(&a, &b);
    assert!(s.distance < 0.0, "試験の箱が重なっていない: {s:?}");
    let shifted = |by: f64| OrientedBox {
        center: b.center + s.axis.scale(by),
        ..b
    };
    assert!(separation(&a, &shifted(s.depth() + 1e-6)).distance > 0.0);
    assert!(separation(&a, &shifted(s.depth() - 1e-6)).distance < 0.0);
}
