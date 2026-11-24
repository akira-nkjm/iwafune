//! 円錐と箱の交差のテスト。手計算（三角関数）の値と比べる。
//!
//! 共通の設定: 頂点は原点、軸は +x、半頂角 30°（`tan 30° = 1/√3`、`sin 30° = 1/2`）。

use super::*;
use sat_astro::math::Quat;

const DEG: f64 = std::f64::consts::PI / 180.0;

fn cone(half_deg: f64, range: Option<f64>) -> Cone {
    Cone::new(Vec3::ZERO, Vec3::X, half_deg * DEG, range).unwrap()
}

fn cube(center: [f64; 3]) -> OrientedBox {
    boxed(center, [1.0; 3], Quat::IDENTITY)
}

fn boxed(center: [f64; 3], size: [f64; 3], q: Quat) -> OrientedBox {
    OrientedBox::from_size(
        Vec3::new(center[0], center[1], center[2]),
        q,
        Vec3::new(size[0], size[1], size[2]),
    )
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}

/// 一辺 1 の立方体が軸の上（中心 (3, 0, 0)）: 軸が当たり、角度 0。最短距離は手前の面までの 3 − 0.5 = 2.5。
#[test]
fn cube_on_the_axis() {
    let c = contact(&cone(30.0, None), &cube([3.0, 0.0, 0.0])).unwrap();
    close(c.min_angle, 0.0);
    close(c.min_distance, 2.5);
}

/// 中心 (3, y, 0) の立方体（x ∈ [2.5, 3.5]、y ∈ [y−0.5, y+0.5]）。軸に最も近い点は (3.5, y−0.5, 0) で、角度は atan((y−0.5)/3.5)。
/// 30° に入る条件は (y−0.5)/3.5 ≤ tan30° ⟺ y ≤ 0.5 + 3.5/√3 = 2.5207。
/// y = 2.5 は atan(2/3.5) = 29.74° で入り、y = 2.55 は atan(2.05/3.5) = 30.36° で外れる。
#[test]
fn nearest_edge_just_inside_and_just_outside_the_cone() {
    let inside = contact(&cone(30.0, None), &cube([3.0, 2.5, 0.0])).unwrap();
    close(inside.min_angle, (2.0_f64 / 3.5).atan());
    assert!(inside.min_angle < 30.0 * DEG);
    assert!(contact(&cone(30.0, None), &cube([3.0, 2.55, 0.0])).is_none());
}

/// 同じ立方体 (3, 2.5, 0) の最短距離: 円錐の中の最小は、面 y = 2 の上で円錐の面に乗る点。
/// 母線の上で y = 2 になるのは x = 2/tan30° = 2√3（箱の x ∈ [2.5, 3.5] の中）なので、点は (2√3, 2, 0) で、距離は √(12 + 4) = 4。
/// （面と円錐の交線の上の停留点。辺や頂点の候補では 4 にならない）
#[test]
fn nearest_point_lies_on_the_cone_surface_inside_a_face() {
    let c = contact(&cone(30.0, None), &cube([3.0, 2.5, 0.0])).unwrap();
    close(c.min_distance, 4.0);
}

/// 頂点の後ろ（中心 (−3, 0, 0)）は、半頂角が 90° 近くても交わらない。
#[test]
fn box_behind_the_apex_never_intersects() {
    for half in [30.0, 80.0, 89.9] {
        assert!(contact(&cone(half, None), &cube([-3.0, 0.0, 0.0])).is_none());
    }
    // 大きな箱でも後ろ（x ∈ [−12, −2]）は同じ
    let big = boxed([-7.0, 0.0, 0.0], [10.0, 40.0, 40.0], Quat::IDENTITY);
    assert!(contact(&cone(89.9, None), &big).is_none());
}

/// 距離の上限: 最短距離 2.5 の箱は、上限 2.4 では交わらず、2.5・2.6 では交わる（以下で数える）。
#[test]
fn range_cuts_boxes_beyond_it() {
    let b = cube([3.0, 0.0, 0.0]);
    assert!(contact(&cone(30.0, Some(2.4)), &b).is_none());
    assert!(contact(&cone(30.0, Some(2.5)), &b).is_some());
    assert!(contact(&cone(30.0, Some(2.6)), &b).is_some());
}

/// 頂点が箱の中（中心 (0.2, 0, 0)）: 角度 0・距離 0。上限が 0 でも交わる。
#[test]
fn apex_inside_the_box() {
    let c = contact(&cone(30.0, Some(0.0)), &cube([0.2, 0.0, 0.0])).unwrap();
    assert_eq!((c.min_angle, c.min_distance), (0.0, 0.0));
}

/// 薄い板 (0.02, 2, 2)（x が厚み）を中心 (3, 1.5, 0) に置く。
/// 面を軸に向けた置き方（板の面の法線が x）: y ∈ [0.5, 2.5]、x ≈ 3。軸に最も近い点 (3.01, 0.5, 0) は atan(0.5/3.01) = 9.43° で、15° の円錐に入る。
/// 辺を軸に向けた置き方（z まわりに 90° 回して厚みが y）: x ∈ [2, 4]、y ∈ [1.49, 1.51]。最も近い点 (4, 1.49, 0) は atan(1.49/4) = 20.4° > 15° で、入らない。
#[test]
fn thin_plate_face_on_and_edge_on() {
    let face_on = boxed([3.0, 1.5, 0.0], [0.02, 2.0, 2.0], Quat::IDENTITY);
    let turn = Quat::from_axis_angle(Vec3::Z, 90.0 * DEG);
    let edge_on = boxed([3.0, 1.5, 0.0], [0.02, 2.0, 2.0], turn);
    let c15 = cone(15.0, None);
    let hit = contact(&c15, &face_on).unwrap();
    close(hit.min_angle, (0.5_f64 / 3.01).atan());
    close(hit.min_distance, (2.99_f64 * 2.99 + 0.25).sqrt());
    assert!(contact(&c15, &edge_on).is_none());
    // 30° の円錐なら辺を向けた板も入る: 最短距離は母線の上の点で y / sin30° = 1.49 / 0.5 = 2.98（x = 1.49/tan30° = 2.5807 は箱の x ∈ [2, 4] の中）
    let c30 = contact(&cone(30.0, None), &edge_on).unwrap();
    close(c30.min_angle, (1.49_f64 / 4.0).atan());
    close(c30.min_distance, 2.98);
}

/// z まわりに 45° 回した立方体（ひし形。頂点は中心から ±√2/2）を中心 (3, 2, 0) に置く。
/// 中心の角度は atan(2/3) = 33.7° > 30° で、中心だけの判定なら外れる。しかし下の角 (3, 2 − √2/2, 0) は atan(1.2929/3) = 23.3° で円錐に入る。
/// 軸に最も近い点はこの角（右の角 (3.7071, 2) は 28.3°、辺の上の点は角から離れるほど角度が増える）。
/// 最短距離: 下の角と左の角 (3 − √2/2, 2) を結ぶ辺 (3 − t, 1.2929 + t) が母線 y = x tan30° と交わる t = (3 tan30° − 1.2929)/(1 + tan30°) = 0.27842 の点で、
/// 距離は y / sin30° = 2 (1.2929 + t) = 3.14262（辺の上では距離が t とともに減るので、円錐に入る最も左の点）。
#[test]
fn rotated_box_corner_pokes_into_the_cone_while_the_centre_is_outside() {
    let q = Quat::from_axis_angle(Vec3::Z, 45.0 * DEG);
    let b = boxed([3.0, 2.0, 0.0], [1.0; 3], q);
    let centre_angle = (2.0_f64 / 3.0).atan();
    assert!(centre_angle > 30.0 * DEG);
    let c = contact(&cone(30.0, None), &b).unwrap();
    let r = std::f64::consts::FRAC_1_SQRT_2;
    close(c.min_angle, ((2.0 - r) / 3.0).atan());
    let t30 = (30.0 * DEG).tan();
    let t = (3.0 * t30 - (2.0 - r)) / (1.0 + t30);
    close(c.min_distance, 2.0 * ((2.0 - r) + t));
}

/// 頂点が 1 つも円錐に入らず、軸も当たらないのに、箱が円錐を横切る例: 軸に垂直な細い棒 (0.1, 10, 0.1) を中心 (3, 0, 1) に置く
/// （x ∈ [2.95, 3.05]、y ∈ [−5, 5]、z ∈ [0.95, 1.05]）。端の頂点 (·, ±5, ·) は 59° 以上で外れるが、y = 0 の付近は円錐を横切る
/// （x = 3 での円錐の半径 √3 = 1.73 に対し、z = 1 で |y| ≤ √(3 − 1) = 1.41 の範囲）。
/// 軸に最も近い点は (3.05, 0, 0.95) で atan(0.95/3.05) = 17.3°、円錐の中の最短距離は手前の面の点 (2.95, 0, 0.95) の √(2.95² + 0.95²)。
#[test]
fn box_crossing_the_cone_without_a_vertex_inside() {
    let rod = boxed([3.0, 0.0, 1.0], [0.1, 10.0, 0.1], Quat::IDENTITY);
    let cn = cone(30.0, None);
    let l = Local::new(&cn, &rod);
    assert!((0..8).all(|k| {
        let p = l.vertex(k);
        angle_between(p, Vec3::X) > 30.0 * DEG
    }));
    let c = contact(&cn, &rod).unwrap();
    close(c.min_angle, (0.95_f64 / 3.05).atan());
    close(c.min_distance, (2.95_f64 * 2.95 + 0.95 * 0.95).sqrt());
}

/// 円錐と箱をそろえて剛体運動（回転と平行移動）させても、結果は変わらない。
#[test]
fn rigid_motion_changes_nothing() {
    let cases = [
        boxed([3.0, 2.5, 0.0], [1.0; 3], Quat::IDENTITY),
        boxed(
            [3.0, 2.0, 0.0],
            [1.0; 3],
            Quat::from_axis_angle(Vec3::Z, 45.0 * DEG),
        ),
        boxed([3.0, 0.0, 1.0], [0.1, 10.0, 0.1], Quat::IDENTITY),
        boxed(
            [2.0, 1.0, -1.0],
            [0.5, 1.5, 0.3],
            Quat::from_axis_angle(Vec3::new(1.0, 2.0, 3.0), 0.7),
        ),
    ];
    let motion = Quat::from_axis_angle(Vec3::new(-1.0, 0.5, 2.0), 1.1);
    let shift = Vec3::new(10.0, -4.0, 2.5);
    for b in cases {
        let base = contact(&cone(30.0, None), &b);
        let moved_cone = Cone::new(shift, motion.rotate(Vec3::X), 30.0 * DEG, None).unwrap();
        let moved_box = OrientedBox {
            center: motion.rotate(b.center) + shift,
            orientation: motion.mul(b.orientation),
            half_extent: b.half_extent,
        };
        let moved = contact(&moved_cone, &moved_box);
        match (base, moved) {
            (Some(x), Some(y)) => {
                close(x.min_angle, y.min_angle);
                close(x.min_distance, y.min_distance);
            }
            (None, None) => {}
            other => panic!("the result changed under a rigid motion: {other:?}"),
        }
    }
}

/// 軸に対して対称な置き方は、同じ角度・距離になる（y・−y・z・−z の 4 方向）。
#[test]
fn symmetric_placements_give_equal_values() {
    let at = |c: [f64; 3]| contact(&cone(30.0, None), &cube(c)).unwrap();
    let base = at([3.0, 2.0, 0.0]);
    for c in [[3.0, -2.0, 0.0], [3.0, 0.0, 2.0], [3.0, 0.0, -2.0]] {
        let other = at(c);
        close(other.min_angle, base.min_angle);
        close(other.min_distance, base.min_distance);
    }
}

/// 円錐を作れない入力（軸が零、半頂角が 0・90° 以上・非有限、上限が負）は `None`。
#[test]
fn invalid_cones_are_rejected() {
    let ok = |axis: Vec3, half: f64, range: Option<f64>| Cone::new(Vec3::ZERO, axis, half, range);
    assert!(ok(Vec3::ZERO, 0.5, None).is_none());
    assert!(ok(Vec3::X, 0.0, None).is_none());
    assert!(ok(Vec3::X, FRAC_PI_2, None).is_none());
    assert!(ok(Vec3::X, f64::NAN, None).is_none());
    assert!(ok(Vec3::X, 0.5, Some(-1.0)).is_none());
    assert!(ok(Vec3::new(2.0, 0.0, 0.0), 0.5, Some(1.0)).is_some());
}

/// 乱数の箱・円錐を、箱の表面の格子点（実在する点）からの値と比べる。
/// 格子点で円錐に入る点が見つかれば、必ず交わる（見落とさない）。厳密な距離・角度は、格子点の最小以下（候補の取りこぼしがあれば、
/// 厳密な値が格子点より大きくなって失敗する）。
#[test]
fn agrees_with_dense_sampling_of_the_box_surface() {
    let mut state = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 11) as f64 / (1u64 << 53) as f64
    };
    let mut compared = 0;
    for _ in 0..300 {
        let mut r = |lo: f64, hi: f64| lo + (hi - lo) * next();
        let b = boxed(
            [r(-1.0, 4.0), r(-3.0, 3.0), r(-3.0, 3.0)],
            [r(0.05, 2.0), r(0.05, 2.0), r(0.05, 2.0)],
            Quat::from_axis_angle(
                Vec3::new(r(-1.0, 1.0), r(-1.0, 1.0), r(-1.0, 1.0)),
                r(0.0, 3.0),
            ),
        );
        let half = r(5.0, 60.0);
        let cn = Cone::new(Vec3::ZERO, Vec3::X, half * DEG, None).unwrap();
        let l = Local::new(&cn, &b);
        let n = 60;
        let mut sample_dist = f64::INFINITY;
        let mut sample_angle = f64::INFINITY;
        for i in 0..3 {
            for sign in [1.0, -1.0] {
                let (j, k) = ((i + 1) % 3, (i + 2) % 3);
                for a in 0..=n {
                    for c in 0..=n {
                        let (fa, fc) = (
                            2.0 * a as f64 / n as f64 - 1.0,
                            2.0 * c as f64 / n as f64 - 1.0,
                        );
                        let p = l.c
                            + l.u[i].scale(sign * l.h[i])
                            + l.u[j].scale(fa * l.h[j])
                            + l.u[k].scale(fc * l.h[k]);
                        let ang = angle_between(p, Vec3::X);
                        sample_angle = sample_angle.min(ang);
                        if ang <= half * DEG {
                            sample_dist = sample_dist.min(p.norm());
                        }
                    }
                }
            }
        }
        match contact(&cn, &b) {
            Some(c) => {
                assert!(
                    c.min_angle <= sample_angle + 1e-9,
                    "angle {c:?} {sample_angle}"
                );
                if sample_dist.is_finite() {
                    assert!(
                        c.min_distance <= sample_dist + 1e-9,
                        "dist {c:?} {sample_dist}"
                    );
                }
                if l.contains(Vec3::ZERO) || axis_entry(&l, Vec3::X).is_some() {
                    assert!(c.min_angle < 1e-9);
                }
                compared += 1;
            }
            None => assert!(
                !sample_dist.is_finite(),
                "a surface point is in the cone but the box was reported as not intersecting"
            ),
        }
    }
    assert!(compared > 40, "too few intersecting cases: {compared}");
}
