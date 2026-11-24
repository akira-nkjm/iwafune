//! 面積と腕の手計算の確認（箱は軸に沿って置く。厳密な幾何なので相対 1e-12 で合う）。

use super::*;
use sat_astro::math::Quat;

fn cuboid(id: &str, at: [f64; 3], size: [f64; 3]) -> InterferenceBox {
    InterferenceBox {
        id: id.to_owned(),
        count: 1,
        position: Vec3::new(at[0], at[1], at[2]),
        orientation: Quat::IDENTITY,
        size: Vec3::new(size[0], size[1], size[2]),
        mount_ancestors: Vec::new(),
    }
}

fn geometry(boxes: Vec<InterferenceBox>, com: [f64; 3]) -> DisturbanceGeometry {
    DisturbanceGeometry {
        boxes,
        center_of_mass: Vec3::new(com[0], com[1], com[2]),
    }
}

/// 面積も圧力中心も書かない面（係数 1）。
fn derived() -> DisturbanceSurface {
    DisturbanceSurface {
        coefficient: 1.0,
        area: None,
        cp_offset: None,
    }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 * b.abs().max(1.0)
}

/// 箱 1 つ（0.1 × 0.2 × 0.3 m、中心 (0.01, 0.02, 0.03)）を z から見る: 面積 = x·y = 0.02、
/// 図心 = 箱の中心の xy。重心が原点なら腕 = (0.01, 0.02, 0)（向きに沿う z 成分は重心が z にずれても効かない）。
/// x から見れば面積 = y·z = 0.06、腕 = (0, 0.02, 0.03)。
#[test]
fn one_box_gives_its_face_and_the_centre_minus_the_centre_of_mass() {
    let boxes = vec![cuboid("B", [0.01, 0.02, 0.03], [0.1, 0.2, 0.3])];
    let model = SurfaceModel::new(Some(&geometry(boxes.clone(), [0.0, 0.0, 0.0])));
    let z = model.evaluate(&derived(), Vec3::Z);
    assert!(close(z.area, 0.02), "{}", z.area);
    assert!((z.cp_offset - Vec3::new(0.01, 0.02, 0.0)).norm() < 1e-12);
    assert_eq!(z.area_source, SurfaceValueSource::Geometry);
    assert_eq!(z.cp_source, SurfaceValueSource::Geometry);
    // 逆向きでも同じ面積と腕
    let back = model.evaluate(&derived(), -Vec3::Z);
    assert!(close(back.area, 0.02) && (back.cp_offset - z.cp_offset).norm() < 1e-12);
    let x = model.evaluate(&derived(), Vec3::X);
    assert!(close(x.area, 0.06), "{}", x.area);
    assert!((x.cp_offset - Vec3::new(0.0, 0.02, 0.03)).norm() < 1e-12);
    // 重心が (0.005, 0, 0.5) にあれば、z から見た腕は (0.005, 0.02, 0)（z 成分は捨てる）
    let moved = SurfaceModel::new(Some(&geometry(boxes, [0.005, 0.0, 0.5])));
    let z = moved.evaluate(&derived(), Vec3::Z);
    assert!((z.cp_offset - Vec3::new(0.005, 0.02, 0.0)).norm() < 1e-12);
}

/// 離れた 2 つの立方体（辺 1 m、中心 x = 0 と 3 m）を z から見る: 面積 2 m²、図心 x = 1.5 m。
/// y から見ても同じ。x から見ると 2 つが重なり、面積は 1 m²（重なりを 1 度だけ数える）、図心は箱の中心の yz。
#[test]
fn two_disjoint_boxes_have_the_union_area_and_the_centroid_between_them() {
    let boxes = vec![
        cuboid("A", [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
        cuboid("B", [3.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
    ];
    let model = SurfaceModel::new(Some(&geometry(boxes, [0.0, 0.0, 0.0])));
    let z = model.evaluate(&derived(), Vec3::Z);
    assert!(close(z.area, 2.0));
    assert!((z.cp_offset - Vec3::new(1.5, 0.0, 0.0)).norm() < 1e-12);
    let x = model.evaluate(&derived(), Vec3::X);
    assert!(close(x.area, 1.0));
    assert!(x.cp_offset.norm() < 1e-12);
}

/// 板（1 × 1 × 0.002 m）を正面から見れば 1 m²、真横から見れば 1 × 0.002 = 0.002 m²（厚さ 2 mm でも粗くならない）。
#[test]
fn a_plate_seen_edge_on_has_the_thickness_area() {
    let model = SurfaceModel::new(Some(&geometry(
        vec![cuboid("P", [0.0, 0.0, 0.0], [1.0, 1.0, 0.002])],
        [0.0; 3],
    )));
    assert!(close(model.evaluate(&derived(), Vec3::Z).area, 1.0));
    assert!(close(model.evaluate(&derived(), Vec3::X).area, 0.002));
}

/// 書かれた面積は箱の面積に勝ち（圧力中心は箱から）、書かれた圧力中心は箱の腕に勝つ（面積は箱から）。
/// 出どころがそれぞれ分かる。
#[test]
fn written_values_replace_the_geometry_and_say_so() {
    let model = SurfaceModel::new(Some(&geometry(
        vec![cuboid("B", [0.01, 0.02, 0.03], [0.1, 0.2, 0.3])],
        [0.0; 3],
    )));
    let area_written = DisturbanceSurface {
        area: Some(Qty::from_raw(0.5)),
        ..derived()
    };
    let s = model.evaluate(&area_written, Vec3::Z);
    assert_eq!(s.area, 0.5);
    assert_eq!(s.area_source, SurfaceValueSource::Written);
    assert_eq!(s.cp_source, SurfaceValueSource::Geometry);
    assert!((s.cp_offset - Vec3::new(0.01, 0.02, 0.0)).norm() < 1e-12);
    let cp = Vec3::new(0.0, 0.0, 0.07);
    let cp_written = DisturbanceSurface {
        cp_offset: Some(cp),
        ..derived()
    };
    let s = model.evaluate(&cp_written, Vec3::Z);
    assert!(close(s.area, 0.02));
    assert_eq!(s.area_source, SurfaceValueSource::Geometry);
    assert_eq!(s.cp_source, SurfaceValueSource::Written);
    assert_eq!(s.cp_offset, cp);
}

/// 箱が無く圧力中心も書かれていなければ、重心に置く（トルク 0。従来の既定）。面積が書かれていればそのまま使える。
#[test]
fn without_boxes_the_centre_of_pressure_defaults_to_the_centre_of_mass() {
    let model = SurfaceModel::new(None);
    assert!(!model.has_geometry());
    let s = model.evaluate(
        &DisturbanceSurface {
            area: Some(Qty::from_raw(0.1)),
            ..derived()
        },
        Vec3::X,
    );
    assert_eq!(s.area, 0.1);
    assert_eq!(s.cp_offset, DEFAULT_CP_OFFSET);
    assert_eq!(s.cp_source, SurfaceValueSource::CentreOfMass);
}

/// ほぼ同じ向き（1e-15 の違い）は同じ投影を使い回し、結果は同じ。向きが違えば別に解く。
#[test]
fn repeated_directions_are_cached_without_changing_the_result() {
    let model = SurfaceModel::new(Some(&geometry(
        vec![cuboid("B", [0.01, 0.02, 0.03], [0.1, 0.2, 0.3])],
        [0.0; 3],
    )));
    let a = model.evaluate(&derived(), Vec3::new(1e-15, -1e-15, 1.0));
    let b = model.evaluate(&derived(), Vec3::Z);
    // 使い回した結果は最初の向きで解いたもの（差は 1e-15 の傾きの分で、浮動小数点の雑音の大きさ）
    assert!(close(a.area, b.area) && (a.cp_offset - b.cp_offset).norm() < 1e-12);
    assert_eq!(model.cache.borrow().len(), 1);
    model.evaluate(&derived(), Vec3::X);
    assert_eq!(model.cache.borrow().len(), 2);
}
