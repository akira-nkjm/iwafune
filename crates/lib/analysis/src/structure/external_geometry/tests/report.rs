//! 解析 `structure.external_geometry` の出力のテスト（入力の規則と、出力の形・数値）。

use super::*;
use crate::structure::interference::InterferenceSkipReason;

fn input_box(id: &str, center: [f64; 3], size: [f64; 3], count: u32) -> InterferenceBox {
    InterferenceBox {
        id: id.to_owned(),
        count,
        position: Vec3::new(center[0], center[1], center[2]),
        orientation: sat_astro::math::Quat::IDENTITY,
        size: Vec3::new(size[0], size[1], size[2]),
        mount_ancestors: Vec::new(),
    }
}

fn run(boxes: Vec<InterferenceBox>, unsized_parts: &[&str]) -> ExternalGeometryOutput {
    ExternalGeometryReport::run(&ExternalGeometryInput {
        boxes,
        unsized_parts: unsized_parts.iter().map(|s| (*s).to_owned()).collect(),
    })
    .unwrap()
}

/// 箱 1 つ 0.3 × 0.2 × 0.1: 軸ごとの投影は面積（x: 0.02、y: 0.03、z: 0.06）で、図心は箱の中心（原点にあれば原点、軸の成分は 0）。
/// 最小は最も小さい面 0.02（軸の上で測る。どの向きでも bc|dx| + ca|dy| + ab|dz| ≥ 最小の面）、最大は √(0.02² + 0.03² + 0.06²) = 0.07 を超えず、
/// 測った向きの中で 0.07 に近い。平均は (xy + yz + zx)/2 = 0.055。表面積は 0.22。
#[test]
fn one_box_report() {
    let out = run(vec![input_box("A", [0.0; 3], [0.3, 0.2, 0.1], 1)], &[]);
    let areas: Vec<f64> = out.projected.iter().map(|p| p.area.value()).collect();
    assert!(close(areas[0], 0.02) && close(areas[1], 0.03) && close(areas[2], 0.06));
    assert_eq!(
        out.projected.iter().map(|p| p.along).collect::<Vec<_>>(),
        [BodyAxis::X, BodyAxis::Y, BodyAxis::Z]
    );
    assert!(out.projected.iter().all(|p| p.centre.norm() < 1e-12));
    assert!(close(out.min_projected.area.value(), 0.02));
    assert!(close(out.min_projected.direction.x.abs(), 1.0));
    let exact_max = (0.02_f64.powi(2) + 0.03_f64.powi(2) + 0.06_f64.powi(2)).sqrt();
    let max = out.max_projected.area.value();
    assert!(
        max <= exact_max + 1e-12 && max > 0.95 * exact_max,
        "{max} vs {exact_max}"
    );
    assert!(close(out.mean_projected_area.value(), 0.055));
    assert!(close(out.exposed_area.value(), 0.22));
    assert_eq!(out.parts_with_box, 1);
    assert_eq!(out.directions_measured, 3 + 384);
    assert!(out.internal_parts.is_empty());
    assert_eq!(out.checks.len(), 1);
    assert_eq!(out.checks[0].verdict, Verdict::Ok);
    assert_eq!(out.exposed_by_direction.len(), 6);
    assert_eq!(out.exposed_by_direction[0].face, FaceDirection::NegX);
}

/// 最小 ≤ 平均 ≤ 最大。員数 2 の部品は 1 つ（一覧にする）、寸法が無い部品は一覧、内部の部品は一覧で表面積に入らない。
#[test]
fn lists_and_bounds() {
    let out = run(
        vec![
            input_box("big", [0.0; 3], [1.0; 3], 1),
            input_box("cells", [3.0, 0.0, 0.0], [0.5, 0.5, 0.1], 4),
            input_box("inside", [0.0; 3], [0.2; 3], 1),
        ],
        &["z_no_size"],
    );
    assert_eq!(out.parts_with_box, 3);
    assert_eq!(out.internal_parts.len(), 1);
    assert_eq!(
        (
            out.internal_parts[0].id.as_str(),
            out.internal_parts[0].inside.as_str()
        ),
        ("inside", "big")
    );
    assert_eq!(out.repeated.len(), 1);
    assert_eq!(
        (out.repeated[0].id.as_str(), out.repeated[0].count),
        ("cells", 4)
    );
    assert_eq!(out.skipped.len(), 1);
    assert_eq!(out.skipped[0].reason, InterferenceSkipReason::NoSize);
    let (min, mean, max) = (
        out.min_projected.area.value(),
        out.mean_projected_area.value(),
        out.max_projected.area.value(),
    );
    assert!(min <= mean && mean <= max);
    // 表面積: 立方体 6 + 板 2(0.25 + 0.05 + 0.05) = 0.7（内部は入らない）
    assert!(close(out.exposed_area.value(), 6.7));
    assert!(out.exposed_by_part.iter().all(|p| p.id != "inside"));
    // 軸ごとの投影は ±向きで同じなので 3 つだけ報告し、図心は軸に垂直な平面の上（その軸の成分が 0）
    assert!(out.projected[0].centre.x.abs() < 1e-12);
    assert!(out.projected[1].centre.y.abs() < 1e-12);
    assert!(out.projected[2].centre.z.abs() < 1e-12);
}

/// 寸法を持つ箱が 1 つも無ければ入力の不正（箱が無ければ 0 の面積を報告しない）。
#[test]
fn no_box_is_an_input_error() {
    let err = ExternalGeometryReport::run(&ExternalGeometryInput {
        boxes: Vec::new(),
        unsized_parts: vec!["A".to_owned()],
    })
    .unwrap_err();
    assert!(matches!(err, AnalysisError::InvalidInput(_)));
}

/// 判定文の数は `quantity_text` で整形される（0.055 m² = 550 cm²）。報告だけ（violation も warning も出さない）。
#[test]
fn the_check_text_carries_the_areas() {
    let out = run(vec![input_box("A", [0.0; 3], [0.3, 0.2, 0.1], 1)], &[]);
    let c = &out.checks[0];
    assert_eq!(c.key, "external_geometry");
    assert!(c.message.contains("550 cm²"), "{}", c.message);
    assert_eq!(c.verdict, Verdict::Ok);
    assert!(c.message.contains("box"));
}
