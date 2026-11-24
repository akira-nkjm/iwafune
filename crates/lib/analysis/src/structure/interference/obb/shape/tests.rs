//! 箱の形のテスト。閉じた式（立方体の輪郭の面積 1・√2・√3、箱 a×b×c の投影面積 bc|dx| + ca|dy| + ab|dz|）と比べる。
//! 厳密な幾何なので許容は相対 1e-12（丸め誤差の積み重ねの上限）。

use super::*;
use sat_astro::math::Quat;

fn boxed(center: [f64; 3], q: Quat, size: [f64; 3]) -> OrientedBox {
    OrientedBox::from_size(
        Vec3::new(center[0], center[1], center[2]),
        q,
        Vec3::new(size[0], size[1], size[2]),
    )
}

fn unit_cube() -> OrientedBox {
    boxed([0.0; 3], Quat::IDENTITY, [1.0; 3])
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 * b.abs().max(1.0)
}

fn unit(x: f64, y: f64, z: f64) -> Vec3 {
    Vec3::new(x, y, z).normalized().unwrap()
}

/// 固定の種の擬似乱数（[0, 1)）。
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 11) as f64 / (1_u64 << 53) as f64
    }
}

/// 単位立方体: 軸に沿って 1、面の対角線に沿って √2（長方形 1 × √2）、体対角線に沿って √3（正六角形、辺 √(2/3)、面積 (3√3/2)·(2/3) = √3）。
#[test]
fn unit_cube_silhouettes() {
    let c = unit_cube();
    for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
        let s = c.silhouette(axis);
        assert!(close(s.area(), 1.0), "{}", s.area());
        assert_eq!(s.vertices().len(), 4);
    }
    let face_diagonal = c.silhouette(unit(1.0, 1.0, 0.0));
    assert!(close(face_diagonal.area(), 2.0_f64.sqrt()));
    let body_diagonal = c.silhouette(unit(1.0, 1.0, 1.0));
    assert!(close(body_diagonal.area(), 3.0_f64.sqrt()));
    assert_eq!(body_diagonal.vertices().len(), 6);
}

/// 箱 a × b × c を任意の向きで見た輪郭の面積は、`bc|dx| + ca|dy| + ab|dz|`（d は箱の軸での成分。凸体の投影面積の厳密な式）。
/// 回した箱と、乱数の向き 200 本で確かめる。
#[test]
fn silhouette_area_of_a_box_is_the_closed_form() {
    let (a, b, c) = (0.3, 0.2, 0.1);
    let q = Quat::from_axis_angle(unit(1.0, 2.0, 3.0), 0.7);
    let shape = boxed([0.4, -0.2, 0.9], q, [a, b, c]);
    let mut rng = Lcg(7);
    for _ in 0..200 {
        let d = unit(rng.next() - 0.5, rng.next() - 0.5, rng.next() - 0.5 + 1e-3);
        let local = q.rotate_inverse(d);
        let want = b * c * local.x.abs() + c * a * local.y.abs() + a * b * local.z.abs();
        let got = shape.silhouette(d).area();
        assert!(close(got, want), "{got} vs {want} for {d:?}");
    }
}

/// 厚さのある板を真横から見る: 辺に沿って平行に見れば面積 0（線分）。
#[test]
fn a_plate_seen_edge_on_has_no_area() {
    let plate = boxed([0.0; 3], Quat::IDENTITY, [1.0, 0.5, 0.002]);
    let edge_on = plate.silhouette(Vec3::X);
    assert!(close(edge_on.area(), 0.5 * 0.002));
    // 面の法線に垂直で、短い辺に沿う向き（板の厚みの向き z は面積 0.5 の長方形、y に沿う向きは 1 × 0.002）
    assert!(close(plate.silhouette(Vec3::Y).area(), 0.002));
    // 回して、板の面に平行な向きにちょうど合わせる: z 軸まわりに 30° 回した板を (cos 30°, sin 30°, 0) の向きから見る
    let turned = boxed(
        [0.0; 3],
        Quat::from_axis_angle(Vec3::Z, 30.0_f64.to_radians()),
        [1.0, 0.5, 0.002],
    );
    let along_long_edge = turned.silhouette(unit(
        30.0_f64.to_radians().cos(),
        30.0_f64.to_radians().sin(),
        0.0,
    ));
    assert!(close(along_long_edge.area(), 0.002 * 0.5));
}

/// 面: 6 つ、法線は外向き、`u × v = normal`、面積の和は 2(ab + bc + ca)、中心は箱の中心から法線方向に半幅だけ。回した箱でも同じ。
#[test]
fn faces_are_outward_rectangles() {
    let (a, b, c) = (0.3, 0.2, 0.1);
    let q = Quat::from_axis_angle(unit(1.0, -1.0, 0.5), 1.1);
    let shape = boxed([1.0, 2.0, 3.0], q, [a, b, c]);
    let faces = shape.faces();
    let total: f64 = faces.iter().map(BoxFace::area).sum();
    assert!(close(total, 2.0 * (a * b + b * c + c * a)));
    for f in &faces {
        assert!((f.u.cross(f.v) - f.normal).norm() < 1e-12);
        assert!(f.normal.dot(f.center - shape.center) > 0.0);
        assert!(close(f.normal.norm(), 1.0));
        // 面の 4 頂点は箱の表面の上（箱の中で、法線方向の端）
        assert!(shape.contains_point(f.center, 1e-12));
        let [u, v] = f.local(f.center + f.u.scale(f.half_u) + f.v.scale(f.half_v));
        assert!(close(u, f.half_u) && close(v, f.half_v));
    }
}

/// 包含: 小さな箱は大きな箱の中、はみ出せば外、面に接しているだけなら（許容の内で）中、回した箱で一辺が出れば外。
#[test]
fn containment_uses_the_eight_corners_with_a_tolerance() {
    let big = boxed([0.0; 3], Quat::IDENTITY, [2.0, 2.0, 2.0]);
    let small = boxed([0.2, 0.0, 0.0], Quat::IDENTITY, [0.5, 0.5, 0.5]);
    assert!(big.contains_box(&small, 0.0));
    let flush = boxed([0.75, 0.0, 0.0], Quat::IDENTITY, [0.5, 0.5, 0.5]);
    assert!(big.contains_box(&flush, 1e-9));
    let out = boxed([0.8, 0.0, 0.0], Quat::IDENTITY, [0.5, 0.5, 0.5]);
    assert!(!big.contains_box(&out, 1e-9));
    assert!(big.contains_box(&out, 0.06));
    // 一辺 1.5 の立方体を z まわりに 45° 回すと、x 方向の張りが 1.5·√2 / 2 = 1.06 で、半幅 1 を超える
    let turned = boxed(
        [0.0; 3],
        Quat::from_axis_angle(Vec3::Z, std::f64::consts::FRAC_PI_4),
        [1.5, 1.5, 0.5],
    );
    assert!(!big.contains_box(&turned, 1e-9));
    assert!(big.contains_box(&turned, 0.07));
    assert!(big.contains_box(&big, 0.0));
}

/// 外接: 2 つの軸平行な箱 [-1,1]³ と中心 (3,0,0) の一辺 1 の箱 → x は [-1, 3.5]、y・z は [-1, 1]。箱が無ければ `None`。
#[test]
fn axis_aligned_extents_of_boxes() {
    let a = boxed([0.0; 3], Quat::IDENTITY, [2.0; 3]);
    let b = boxed([3.0, 0.0, 0.0], Quat::IDENTITY, [1.0; 3]);
    let (lo, hi) = axis_aligned_extents(&[a, b]).unwrap();
    assert_eq!((lo.x, lo.y, lo.z), (-1.0, -1.0, -1.0));
    assert_eq!((hi.x, hi.y, hi.z), (3.5, 1.0, 1.0));
    assert!(axis_aligned_extents(&[]).is_none());
}

fn section_area(shape: &OrientedBox, n: Vec3, offset: f64, tol: f64) -> Option<f64> {
    let points = shape.plane_section(n, offset, tol)?;
    let (e1, e2) = plane_basis(n);
    let p2: Vec<[f64; 2]> = points.iter().map(|p| [p.dot(e1), p.dot(e2)]).collect();
    Some(ConvexPolygon::hull(&p2).area())
}

/// 断面: 立方体を中ほどで z に垂直に切れば面積 1、体対角線に垂直な原点を通る平面で切れば正六角形（辺 √2/2、面積 3√3/4）。
/// 離れた平面は `None`、面に接する平面（許容の内）は面そのもの（面積 1）、辺に接する平面は面積 0。
#[test]
fn plane_sections_of_a_unit_cube() {
    let c = unit_cube();
    assert!(close(section_area(&c, Vec3::Z, 0.2, 0.0).unwrap(), 1.0));
    // 体対角線に垂直な中央の断面: 辺 √2/2 の正六角形、面積 (3√3/2)(√2/2)² = 3√3/4
    let hexagon = section_area(&c, unit(1.0, 1.0, 1.0), 0.0, 0.0).unwrap();
    assert!(close(hexagon, 3.0 * 3.0_f64.sqrt() / 4.0), "{hexagon}");
    assert!(c.plane_section(Vec3::Z, 0.6, 0.0).is_none());
    assert!(c.plane_section(Vec3::Z, 0.5 + 1e-4, 1e-3).is_some());
    assert!(close(
        section_area(&c, Vec3::Z, 0.5 + 1e-4, 1e-3).unwrap(),
        1.0
    ));
    assert!(close(section_area(&c, Vec3::Z, -0.5, 0.0).unwrap(), 1.0));
    // 辺 (x = 0.5, z = 0.5) だけに接する平面（法線 (1,0,1)/√2、原点から 0.5·√2）: 接触は線分で面積 0（丸めの分だけ許容を取る）
    let edge = section_area(&c, unit(1.0, 0.0, 1.0), 0.5 * 2.0_f64.sqrt(), 1e-9).unwrap();
    assert!(edge.abs() < 1e-12);
}
