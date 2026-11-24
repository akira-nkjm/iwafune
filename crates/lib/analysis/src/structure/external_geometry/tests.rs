//! 外形の幾何のテスト。閉じた式と手計算で確かめる（厳密な幾何は相対 1e-12。求積は測った精度を理由にして許容を置く）。
//! デモの値は貼らない（デモの配置は別に変わる）。

mod exposure;
mod report;

use super::*;
use sat_astro::math::Quat;

pub(super) fn part(id: &str, center: [f64; 3], size: [f64; 3]) -> PartBox {
    rotated(id, center, Quat::IDENTITY, size)
}

pub(super) fn rotated(id: &str, center: [f64; 3], q: Quat, size: [f64; 3]) -> PartBox {
    PartBox {
        id: id.to_owned(),
        shape: OrientedBox::from_size(
            Vec3::new(center[0], center[1], center[2]),
            q,
            Vec3::new(size[0], size[1], size[2]),
        ),
    }
}

pub(super) fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 * b.abs().max(1.0)
}

fn area(g: &ExternalGeometry, d: Vec3) -> f64 {
    g.projected(d).unwrap().area
}

/// 一辺の長さの積から出る箱 1 つの平均の投影面積（凸体の平均の投影面積は表面積の 1/4 = (xy + yz + zx)/2）。
fn convex_mean(s: [f64; 3]) -> f64 {
    (s[0] * s[1] + s[1] * s[2] + s[2] * s[0]) / 2.0
}

fn unit(x: f64, y: f64, z: f64) -> Vec3 {
    Vec3::new(x, y, z).normalized().unwrap()
}

/// 向きの組: 重みの和 1、向きは単位ベクトルで上半球、そして多項式の平均が厳密（`⟨|dz|⟩ = 1/2`、`⟨dx²⟩ = 1/3`、`⟨|dx|⟩ = 1/2`）。
#[test]
fn the_direction_set_integrates_simple_functions_exactly() {
    let set = DirectionSet::standard();
    assert_eq!(
        set.nodes().len(),
        directions::POLAR_NODES * 4 * directions::AZIMUTH_NODES_PER_QUADRANT
    );
    let mean = |f: fn(Vec3) -> f64| -> f64 { set.nodes().iter().map(|(d, w)| w * f(*d)).sum() };
    assert!(close(mean(|_| 1.0), 1.0));
    assert!(
        set.nodes()
            .iter()
            .all(|(d, _)| (d.norm() - 1.0).abs() < 1e-14 && d.z > 0.0)
    );
    assert!(close(mean(|d| d.z.abs()), 0.5));
    assert!(close(mean(|d| d.x * d.x), 1.0 / 3.0));
    assert!(close(mean(|d| d.x.abs()), 0.5));
}

/// 箱 1 つの平均の投影面積は (xy + yz + zx)/2。軸に平行なら求積が解析的で 1e-12 で合う（測った誤差は 1e-15 の程度）。
/// 任意に回した箱は折れ目が象限の境目に無いので、測った最悪 1e-3（40 通りの乱数。向きの組の説明を参照）の 2 倍の 2e-3 を許す。
#[test]
fn mean_projected_area_of_one_box_matches_the_closed_form() {
    let s = [0.3, 0.2, 0.1];
    let aligned = ExternalGeometry::new(vec![part("A", [0.1, 0.2, 0.3], s)]);
    let got = aligned.mean_projected_area();
    assert!(close(got, convex_mean(s)), "{got} vs {}", convex_mean(s));
    let q = Quat::from_axis_angle(unit(1.0, 2.0, 3.0), 0.7);
    let turned = ExternalGeometry::new(vec![rotated("A", [0.0; 3], q, s)]);
    let got = turned.mean_projected_area();
    assert!(
        (got / convex_mean(s) - 1.0).abs() < 2e-3,
        "{got} vs {}",
        convex_mean(s)
    );
}

/// 隣り合う 2 つの箱（x で接する）: A = [0,1]³、B = x∈[1,2]、y・z∈[0,0.5]。
/// x に沿って見れば B の輪郭 (0.5 × 0.5) は A の輪郭 (1 × 1) の中で、面積は大きいほう = 1。
/// y・z に沿って見れば 2 つの輪郭は離れていて（x の範囲が [0,1] と [1,2]）、面積は和: y では A 1 + B (1 × 0.5) = 1.5、z も 1.5。
/// z 方向の輪郭の図心は、A の中心 (0.5, 0.5)・面積 1 と B の中心 (1.5, 0.25)・面積 0.5 の面積つき平均 = (1.25/1.5, 0.625/1.5)。
#[test]
fn two_boxes_side_by_side() {
    let g = ExternalGeometry::new(vec![
        part("A", [0.5, 0.5, 0.5], [1.0, 1.0, 1.0]),
        part("B", [1.5, 0.25, 0.25], [1.0, 0.5, 0.5]),
    ]);
    assert!(close(area(&g, Vec3::X), 1.0));
    assert!(close(area(&g, Vec3::Y), 1.5));
    let z = g.projected(Vec3::Z).unwrap();
    assert!(close(z.area, 1.5));
    let c = z.centre.unwrap();
    assert!(close(c.x, 1.25 / 1.5) && close(c.y, 0.625 / 1.5) && c.z.abs() < 1e-12);
    // 逆向きでも同じ面積と同じ点
    let back = g.projected(-Vec3::Z).unwrap();
    assert!(close(back.area, 1.5));
    assert!((back.centre.unwrap() - c).norm() < 1e-12);
}

/// 1 つの箱が別の箱を隠す: 大きな板 [0,2]×[0,2]×[0,1] の後ろ (z∈[3,4]) に小さな箱 (x・y ∈ [0.5,1.5])。
/// z に沿って見れば小さな箱は完全に隠れて面積 4（内部の部品ではない: 箱の中にはない）。x に沿って見れば離れていて 2·1 + 1·1 = 3。
#[test]
fn one_box_hides_another() {
    let g = ExternalGeometry::new(vec![
        part("big", [1.0, 1.0, 0.5], [2.0, 2.0, 1.0]),
        part("small", [1.0, 1.0, 3.5], [1.0, 1.0, 1.0]),
    ]);
    assert!(close(area(&g, Vec3::Z), 4.0));
    assert!(close(area(&g, Vec3::X), 3.0));
    assert!(g.internal_ids().is_empty());
}

/// 展開したパドルのような板を箱の辺に付ける。本体 0.2 m 立方体（原点）、板は x∈[0.1,0.5]・y∈[−0.1,0.1]・z∈[0.099,0.101]（0.4 × 0.2 × 0.002、上の辺に付く）。
/// z: 本体 0.04 + 板 0.08（x の範囲が接するだけ）= 0.12、図心の x = 0.08·0.3/0.12 = 0.2。
/// x: 本体 0.04 + 板の輪郭 (0.2 × 0.002) のうち本体の上に出る z∈[0.1,0.101] の 0.2 × 0.001 = 0.0402、図心の z = 0.0002·0.1005/0.0402。
/// y: 本体 0.04 + 板 (0.4 × 0.002 = 0.0008、x の範囲が離れている) = 0.0408、図心 (x, z) = (0.0008·0.3, 0.0008·0.1)/0.0408。
#[test]
fn a_plate_on_the_edge_like_a_deployed_paddle() {
    let g = ExternalGeometry::new(vec![
        part("body", [0.0; 3], [0.2; 3]),
        part("paddle", [0.3, 0.0, 0.1], [0.4, 0.2, 0.002]),
    ]);
    let z = g.projected(Vec3::Z).unwrap();
    assert!(close(z.area, 0.12));
    assert!((z.centre.unwrap() - Vec3::new(0.2, 0.0, 0.0)).norm() < 1e-12);
    let x = g.projected(Vec3::X).unwrap();
    assert!(close(x.area, 0.0402));
    let cx = x.centre.unwrap();
    assert!(cx.x.abs() < 1e-12 && cx.y.abs() < 1e-12 && close(cx.z, 0.0002 * 0.1005 / 0.0402));
    let y = g.projected(Vec3::Y).unwrap();
    assert!(close(y.area, 0.0408));
    let cy = y.centre.unwrap();
    assert!(close(cy.x, 0.0008 * 0.3 / 0.0408) && close(cy.z, 0.0008 * 0.1 / 0.0408));
    assert!(cy.y.abs() < 1e-12);
}

/// 遠く離れた同じ立方体 2 つの平均は、和ではない（立方体を結ぶ向きの近くでは重なって隠し合う）。
/// 範囲: 1 つの平均 1.5 以上、和 3.0 以下。さらに、細かい向きの組（24 × 4 × 16）の平均と、既定の組が 1e-3 で合う（求積の収束）。
/// 近さ（中心の距離 2、隙間 1）は、隠し合いが数値の誤差より十分大きく出るように決めた物理の設定。
#[test]
fn the_mean_of_two_cubes_is_not_the_sum() {
    let g = ExternalGeometry::new(vec![
        part("A", [0.0; 3], [1.0; 3]),
        part("B", [2.0, 0.0, 0.0], [1.0; 3]),
    ]);
    let mean = g.mean_projected_area();
    assert!(mean > 1.5 && mean < 3.0, "{mean}");
    assert!(3.0 - mean > 0.05, "{mean}");
    let fine = g.mean_projected_area_with(&DirectionSet::hemisphere(24, 16));
    assert!((mean / fine - 1.0).abs() < 1e-3, "{mean} vs {fine}");
}

/// 剛体運動に対して不変: 箱を丸ごと回して動かし、向きも同じ回転にすれば、投影面積は同じ（1e-12）で、図心は回転し、
/// 動かした分のうち向きに垂直な成分だけずれる。
#[test]
fn rigid_motion_invariance() {
    let boxes = |q: Quat, t: Vec3| -> ExternalGeometry {
        let place = |c: [f64; 3]| q.rotate(Vec3::new(c[0], c[1], c[2])) + t;
        let mut parts = Vec::new();
        for (id, c, qq, s) in [
            ("A", [0.0, 0.0, 0.0], Quat::IDENTITY, [0.4, 0.3, 0.2]),
            (
                "B",
                [0.3, 0.1, 0.0],
                Quat::from_axis_angle(Vec3::Z, 0.4),
                [0.2, 0.2, 0.2],
            ),
            (
                "C",
                [0.1, -0.3, 0.2],
                Quat::from_axis_angle(Vec3::X, 0.9),
                [0.3, 0.05, 0.1],
            ),
        ] {
            let p = place(c);
            parts.push(rotated(id, [p.x, p.y, p.z], q.mul(qq), s));
        }
        ExternalGeometry::new(parts)
    };
    let base = boxes(Quat::IDENTITY, Vec3::ZERO);
    let q = Quat::from_axis_angle(unit(1.0, -2.0, 0.5), 1.234);
    let t = Vec3::new(0.7, -0.2, 0.4);
    let moved = boxes(q, t);
    for d in [unit(1.0, 0.3, -0.2), unit(0.1, 1.0, 0.5), Vec3::Z] {
        let (a, b) = (
            base.projected(d).unwrap(),
            moved.projected(q.rotate(d)).unwrap(),
        );
        assert!(close(a.area, b.area), "{} vs {}", a.area, b.area);
        let d2 = q.rotate(d);
        let want = q.rotate(a.centre.unwrap()) + (t - d2.scale(t.dot(d2)));
        assert!((b.centre.unwrap() - want).norm() < 1e-12);
    }
}

/// 内部の部品: 大きな箱の中の小さな箱は内部、許容の内で重なる同じ箱は後のほうだけが内部、外にはみ出す箱は内部でない。
#[test]
fn the_internal_rule() {
    let g = ExternalGeometry::new(vec![
        part("a_big", [0.0; 3], [2.0; 3]),
        part("b_inside", [0.2, 0.0, 0.0], [0.5; 3]),
        part("c_flush", [0.75, 0.0, 0.0], [0.5; 3]),
        part("d_poking", [0.9, 0.0, 0.0], [0.5; 3]),
        part("e_twin", [0.0; 3], [2.0; 3]),
    ]);
    assert_eq!(g.internal_ids(), ["b_inside", "c_flush", "e_twin"]);
    // 内部の部品は投影を変えない（和集合が隠す）: 外にはみ出す d_poking の分だけ x の向きの輪郭が大きい
    let without = ExternalGeometry::new(vec![part("a_big", [0.0; 3], [2.0; 3])]);
    assert!(close(
        area(&g, Vec3::Z),
        area(&without, Vec3::Z) + 0.5 * 0.15
    ));
    assert!(close(area(&g, Vec3::X), area(&without, Vec3::X)));
}

/// 不正な向き（零・非有限）は `None`。
#[test]
fn an_invalid_direction_has_no_projection() {
    let g = ExternalGeometry::new(vec![part("A", [0.0; 3], [1.0; 3])]);
    assert!(g.projected(Vec3::ZERO).is_none());
    assert!(g.projected(Vec3::new(f64::NAN, 0.0, 1.0)).is_none());
    assert!(
        ExternalGeometry::new(Vec::new())
            .projected(Vec3::X)
            .unwrap()
            .centre
            .is_none()
    );
}

/// 箱 41 個（リリースビルドで測る。デバッグビルドでは遅い）の費用の記録: 1 向きの投影と、全方向の平均。
#[test]
#[ignore = "slow in debug builds: records the cost on about 40 boxes (cargo test --release -- --ignored --nocapture)"]
fn cost_of_the_projection_on_forty_boxes() {
    let mut state = 99_u64;
    let mut rnd = || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 11) as f64 / (1_u64 << 53) as f64
    };
    let mut parts = vec![part("p00", [0.0; 3], [0.226, 0.1, 0.366])];
    for i in 1..9 {
        let c = [
            (rnd() - 0.5) * 0.4,
            (rnd() - 0.5) * 0.4,
            (rnd() - 0.5) * 0.3,
        ];
        let s = [0.05 + 0.1 * rnd(), 0.05 + 0.1 * rnd(), 0.05 + 0.1 * rnd()];
        parts.push(rotated(
            &format!("p{i:02}"),
            c,
            Quat::from_axis_angle(Vec3::Z, rnd()),
            s,
        ));
    }
    for i in 9..40 {
        let c = [
            (rnd() - 0.5) * 0.1,
            (rnd() - 0.5) * 0.05,
            (rnd() - 0.5) * 0.3,
        ];
        let s = [
            0.03 + 0.05 * rnd(),
            0.03 + 0.04 * rnd(),
            0.02 + 0.05 * rnd(),
        ];
        parts.push(part(&format!("p{i:02}"), c, s));
    }
    parts.push(rotated(
        "p40",
        [0.0; 3],
        Quat::from_axis_angle(Vec3::X, 0.3),
        [0.002, 0.3, 0.4],
    ));
    let g = ExternalGeometry::new(parts);
    let start = std::time::Instant::now();
    let mean = g.mean_projected_area();
    let elapsed = start.elapsed();
    println!(
        "{} boxes ({} internal): mean {mean} m² in {elapsed:?} over {} directions",
        g.parts().len(),
        g.internal_ids().len(),
        DirectionSet::standard().nodes().len()
    );
    let start = std::time::Instant::now();
    let e = g.exposure();
    println!("exposure {} m² in {:?}", e.total, start.elapsed());
    assert!(elapsed.as_secs_f64() < 5.0);
}
