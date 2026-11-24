//! 外に出ている表面積のテスト（手計算。厳密な幾何なので相対 1e-12）。

use super::*;

fn exposure_of(parts: Vec<PartBox>) -> exposed::Exposure {
    ExternalGeometry::new(parts).exposure()
}

/// 箱 1 つ 0.3 × 0.2 × 0.1: 全表面 2(xy + yz + zx) = 2(0.06 + 0.02 + 0.03) = 0.22。
/// 向き別（軸に平行なので厳密）: ±X は yz = 0.02、±Y は zx = 0.03、±Z は xy = 0.06 がそれぞれ 1 面。
#[test]
fn a_single_box_exposes_its_whole_surface() {
    let e = exposure_of(vec![part("A", [1.0, 2.0, 3.0], [0.3, 0.2, 0.1])]);
    assert!(close(e.total, 0.22));
    for (got, want) in e
        .by_direction
        .iter()
        .zip([0.02, 0.02, 0.03, 0.03, 0.06, 0.06])
    {
        assert!(close(*got, want), "{got} vs {want}");
    }
    assert_eq!(e.by_part.len(), 1);
}

/// 面どうしを接した 2 つの立方体: 合計は和 − 接触の長方形の 2 倍 = 12 − 2 = 10。
/// 小さな箱 (0.5 立方) を大きな立方体の +x の面の中央に載せる: 大 6 + 小 1.5 − 2 × 0.25 = 7.0。
#[test]
fn glued_boxes_lose_the_contact_twice() {
    let e = exposure_of(vec![
        part("A", [0.5, 0.5, 0.5], [1.0; 3]),
        part("B", [1.5, 0.5, 0.5], [1.0; 3]),
    ]);
    assert!(close(e.total, 10.0));
    let e = exposure_of(vec![
        part("A", [0.5, 0.5, 0.5], [1.0; 3]),
        part("B", [1.25, 0.5, 0.5], [0.5; 3]),
    ]);
    assert!(close(e.total, 7.0));
    // 大きな箱の +x の面は 1 − 0.25 = 0.75 だけ見え、小さな箱の −x の面は 0
    assert!(close(e.by_direction[1], 0.75 + 0.25));
    assert!(close(e.by_part[1].1, 1.5 - 0.25));
}

/// 大きな箱の中の小さな箱は内部で、表面積に入らない（合計は大きな箱だけ 6、部品の一覧にも出ない）。
#[test]
fn a_box_inside_another_is_internal() {
    let g = ExternalGeometry::new(vec![
        part("big", [0.0; 3], [1.0; 3]),
        part("small", [0.1, 0.0, 0.0], [0.3; 3]),
    ]);
    let e = g.exposure();
    assert!(close(e.total, 6.0));
    assert_eq!(g.internal_ids(), ["small"]);
    assert_eq!(e.by_part, vec![(0, e.total)]);
}

/// 箱の面をぴったり覆うパネル（1 × 1 × 0.01 を上に重ねる）: 箱の上面は全部覆われて 5、パネルは上面 1 + 側面 4 × 0.01 = 1.04（下面は覆われる）。
#[test]
fn a_panel_covering_a_face_exactly() {
    let e = exposure_of(vec![
        part("box", [0.0; 3], [1.0; 3]),
        part("panel", [0.0, 0.0, 0.505], [1.0, 1.0, 0.01]),
    ]);
    assert!(close(e.by_part[0].1, 5.0), "{}", e.by_part[0].1);
    assert!(close(e.by_part[1].1, 1.04), "{}", e.by_part[1].1);
    assert!(close(e.total, 6.04));
}

/// 面の一部だけを覆う回した板: 0.5 × 0.5 × 0.01 の板を z まわりに 45° 回して上面の中央に載せる（輪郭の面積 0.25 は上面の中）。
/// 箱は 5 + (1 − 0.25) = 5.75、板は上面 0.25 + 側面 4 × 0.5 × 0.01 = 0.27、合計 6.02。
#[test]
fn a_rotated_plate_covers_part_of_a_face() {
    let q = Quat::from_axis_angle(Vec3::Z, std::f64::consts::FRAC_PI_4);
    let e = exposure_of(vec![
        part("box", [0.0; 3], [1.0; 3]),
        rotated("plate", [0.0, 0.0, 0.505], q, [0.5, 0.5, 0.01]),
    ]);
    assert!(close(e.by_part[0].1, 5.75), "{}", e.by_part[0].1);
    assert!(close(e.by_part[1].1, 0.27), "{}", e.by_part[1].1);
}

/// 面を貫く箱: 立方体 [−0.5,0.5]³ の +z の面 (z = 0.5) を、z まわりに 45° 回した細い柱（0.2 × 0.2、z ∈ [0.3, 0.9]）が貫く。
/// 立方体の +z の面は柱の断面（辺 0.2 の正方形を回したもの、面積 0.04）だけ覆われて、立方体は 6 − 0.04。
/// 柱の下面（z = 0.3）は立方体の断面に覆われて 0、側面 4 つは立方体の上 (z ∈ [0.5, 0.9]) の 0.2 × 0.4 = 0.08 ずつが出て 0.32、上面 0.04 → 柱は 0.36。
#[test]
fn a_box_piercing_a_face() {
    let q = Quat::from_axis_angle(Vec3::Z, std::f64::consts::FRAC_PI_4);
    let e = exposure_of(vec![
        part("cube", [0.0; 3], [1.0; 3]),
        rotated("pillar", [0.0, 0.0, 0.6], q, [0.2, 0.2, 0.6]),
    ]);
    assert!(close(e.by_part[0].1, 6.0 - 0.04), "{}", e.by_part[0].1);
    assert!(close(e.by_part[1].1, 0.36), "{}", e.by_part[1].1);
}

/// 向きの内訳: 45° 回した箱の側面は法線の成分が並ぶので、番号の小さい軸（x）に入る。面積の合計は回しても同じ（0.22）。
#[test]
fn a_rotated_box_is_binned_by_the_nearest_axis() {
    let q = Quat::from_axis_angle(Vec3::Z, std::f64::consts::FRAC_PI_4);
    let e = exposure_of(vec![rotated("A", [0.0; 3], q, [0.3, 0.2, 0.1])]);
    assert!(close(e.total, 0.22));
    // 側面 4 つ: 法線 (±.707, ±.707, 0)。x の成分が並ぶので ±X に入る（符号は x の成分）。面積は 0.2·0.1 の 2 面と 0.3·0.1 の 2 面
    assert!(close(
        e.by_direction[0] + e.by_direction[1],
        2.0 * 0.02 + 2.0 * 0.03
    ));
    assert_eq!(e.by_direction[2] + e.by_direction[3], 0.0);
    assert!(close(e.by_direction[4] + e.by_direction[5], 0.12));
    // 少し回しただけ（10°）なら、法線に近い軸に入る
    let q = Quat::from_axis_angle(Vec3::Z, 10.0_f64.to_radians());
    let e = exposure_of(vec![rotated("A", [0.0; 3], q, [0.3, 0.2, 0.1])]);
    assert!(close(e.by_direction[0] + e.by_direction[1], 2.0 * 0.02));
    assert!(close(e.by_direction[2] + e.by_direction[3], 2.0 * 0.03));
}

/// 剛体運動に対して合計は不変（回して動かしても同じ）。
#[test]
fn the_exposed_total_is_rigid_motion_invariant() {
    let make = |q: Quat, t: Vec3| {
        let place = |c: [f64; 3]| q.rotate(Vec3::new(c[0], c[1], c[2])) + t;
        let mut parts = Vec::new();
        for (id, c, qq, s) in [
            ("A", [0.0, 0.0, 0.0], Quat::IDENTITY, [0.4, 0.3, 0.2]),
            (
                "B",
                [0.0, 0.0, 0.11],
                Quat::from_axis_angle(Vec3::Z, 0.4),
                [0.2, 0.2, 0.02],
            ),
            ("C", [0.3, 0.1, 0.0], Quat::IDENTITY, [0.2, 0.1, 0.1]),
        ] {
            let p = place(c);
            parts.push(rotated(id, [p.x, p.y, p.z], q.mul(qq), s));
        }
        exposure_of(parts).total
    };
    let base = make(Quat::IDENTITY, Vec3::ZERO);
    let moved = make(
        Quat::from_axis_angle(Vec3::new(1.0, -2.0, 0.5).normalized().unwrap(), 1.1),
        Vec3::new(3.0, 1.0, -2.0),
    );
    assert!(close(base, moved), "{base} vs {moved}");
}
