//! `structure.external_geometry` の射影: 部品の箱を取付の連鎖から機体座標に置き、形態の配置に追従すること、
//! 寸法を持つ部品が無ければ入力の不足になること。デモは、デモ自身の箱から測の中で計算した値と比べる（値は貼らない）。

use fune_model::Design;
use fune_projection::{Context, ProjectionError, structure};
use fune_store::FileSet;
use fune_store::read::design_of;
use sat_analysis::spec::{Analysis, Verdict};
use sat_analysis::structure::external_geometry::geometry::{ExternalGeometry, PartBox};
use sat_analysis::structure::interference::obb::OrientedBox;
use sat_analysis::structure::{ExternalGeometryOutput, ExternalGeometryReport};
use sat_astro::math::Vec3;
use std::path::Path;

/// 一辺 0.2 m の本体。
const FRAME: &str = "[attr]\nsize = { x = 0.2, y = 0.2, z = 0.2 }\n";
/// 0.18 × 0.18 × 0.002 m の板。しまった配置では本体の中（原点）で、本体に含まれる（内部の部品）。
const PADDLE: &str =
    "[mount]\non = \"FRAME\"\n\n[attr]\nsize = { x = 0.18, y = 0.18, z = 0.002 }\n";
/// 寸法の無い部品
const D: &str = "[mount]\non = \"FRAME\"\n";
/// 板を x = 0.3 m に開く形態（本体の外。x の範囲 0.21〜0.39 m）
const OPEN: &str = "label = \"Open\"\n\n[pose.PADDLE]\nposition = { x = 0.3, y = 0.0, z = 0.0 }\n";

fn design(extra: &[(&str, &str)]) -> Design {
    let mut set = FileSet::new();
    set.insert("fune.toml", "[project]\nname = \"t\"\n");
    for (path, text) in [
        ("parts/FRAME.toml", FRAME),
        ("parts/PADDLE.toml", PADDLE),
        ("parts/D.toml", D),
    ] {
        set.insert(path, text);
    }
    for (path, text) in extra {
        set.insert(*path, *text);
    }
    design_of(&set)
}

fn run(d: &Design, configuration: Option<&str>) -> ExternalGeometryOutput {
    let resolved = fune_model::resolve(d);
    let ctx = Context::bare(d, resolved.in_configuration(configuration));
    let input = structure::external_geometry(&ctx).expect("a design with sized parts is complete");
    ExternalGeometryReport::run(&input).expect("the analysis runs on boxes")
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 * b.abs().max(1.0)
}

/// しまった配置: 板は本体の中（内部）で、投影は本体だけ（z で 0.04 m²）、表面積は本体の 6 面 0.24 m²。寸法の無い D は一覧。
/// 開いた配置: 板は本体の外 (x = 0.3) に離れ、z の投影は 0.04 + 0.18² = 0.0724 m²、図心の x = 0.0324·0.3/0.0724、
/// 表面積は 0.24 + 板の 2·0.18² + 4·0.18·0.002 = 0.30624。同じ設計で形態だけが違う。
#[test]
fn the_analysis_follows_the_configuration_in_effect() {
    let d = design(&[("configurations/OPEN.toml", OPEN)]);
    let stowed = run(&d, None);
    assert_eq!(stowed.internal_parts.len(), 1);
    assert_eq!(
        (
            stowed.internal_parts[0].id.as_str(),
            stowed.internal_parts[0].inside.as_str()
        ),
        ("PADDLE", "FRAME")
    );
    assert_eq!(stowed.skipped.len(), 1);
    assert_eq!(stowed.skipped[0].id, "D");
    assert!(close(stowed.projected[2].area.value(), 0.04));
    assert!(close(stowed.exposed_area.value(), 0.24));
    let open = run(&d, Some("OPEN"));
    assert!(open.internal_parts.is_empty());
    assert!(close(open.projected[2].area.value(), 0.0724));
    assert!(close(open.projected[2].centre.x, 0.0324 * 0.3 / 0.0724));
    assert!(close(open.exposed_area.value(), 0.30624));
    assert_eq!(open.checks[0].verdict, Verdict::Ok);
}

/// 寸法を持つ部品が無ければ、外形は出せない: 入力の不足（部品の無い設計も同じ）。
#[test]
fn no_sized_part_is_missing_input() {
    let mut set = FileSet::new();
    set.insert("fune.toml", "[project]\nname = \"t\"\n");
    set.insert("parts/D.toml", "");
    let d = design_of(&set);
    let resolved = fune_model::resolve(&d);
    let ctx = Context::bare(&d, &resolved);
    assert!(structure::external_geometry(&ctx).is_err());
    let json = fune_projection::project_json("structure.external_geometry", &ctx);
    assert!(matches!(json, Err(ProjectionError::Missing(_))));
}

fn demo_shapes() -> (Vec<(String, OrientedBox)>, ExternalGeometryOutput) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    let design = fune_store::load(&root)
        .expect("the demo should load")
        .design;
    let resolved = fune_model::resolve(&design);
    let ctx = Context::bare(&design, &resolved);
    let shapes = structure::boxes(&ctx)
        .boxes
        .iter()
        .map(|b| {
            (
                b.id.clone(),
                OrientedBox::from_size(b.position, b.orientation, b.size),
            )
        })
        .collect();
    (shapes, run(&design, None))
}

/// デモ: 性質で確かめる（デモの値は貼らない。デモの配置は別に変わる）。基準はデモの箱から測の中で計算する。
/// - 軸ごとの投影は、最大の箱（本体の枠）のその向きの輪郭の面積以上、全部の箱の輪郭の面積の和以下。
/// - 平均は、最大の箱 1 つの平均（(xy + yz + zx)/2）以上、全部の箱 1 つずつの平均の和以下。
/// - 最小 ≤ 軸の面積 ≤ 最大、最小 ≤ 平均 ≤ 最大。
/// - 内部の部品は、ほかの箱に丸ごと含まれる。表面積は、内部でない箱の表面積の和以下。
#[test]
fn the_demo_satisfies_the_geometric_bounds() {
    let (shapes, out) = demo_shapes();
    assert!(out.parts_with_box as usize >= shapes.len());
    let volume = |s: &OrientedBox| s.half().iter().product::<f64>();
    let body = shapes
        .iter()
        .max_by(|a, b| volume(&a.1).total_cmp(&volume(&b.1)))
        .expect("the demo has boxes");
    for (axis, direction) in out.projected.iter().zip([Vec3::X, Vec3::Y, Vec3::Z]) {
        let own = body.1.silhouette(direction).area();
        let sum: f64 = shapes
            .iter()
            .map(|(_, s)| s.silhouette(direction).area())
            .sum();
        let got = axis.area.value();
        assert!(
            got >= own - 1e-12 && got <= sum + 1e-12,
            "{got} not in [{own}, {sum}]"
        );
        assert!(
            out.min_projected.area.value() <= got + 1e-12
                && got <= out.max_projected.area.value() + 1e-12
        );
    }
    let single_mean = |s: &OrientedBox| {
        let g = ExternalGeometry::new(vec![PartBox {
            id: "x".into(),
            shape: *s,
        }]);
        g.mean_projected_area()
    };
    let mean = out.mean_projected_area.value();
    let own: f64 = single_mean(&body.1);
    let sum: f64 = shapes.iter().map(|(_, s)| single_mean(s)).sum();
    assert!(
        mean >= own - 1e-12 && mean <= sum + 1e-12,
        "{mean} not in [{own}, {sum}]"
    );
    assert!(out.min_projected.area.value() <= mean && mean <= out.max_projected.area.value());
    for part in &out.internal_parts {
        let find = |id: &str| {
            &shapes
                .iter()
                .find(|(i, _)| i == id)
                .expect("a listed part has a box")
                .1
        };
        assert!(
            find(&part.inside).contains_box(find(&part.id), 1e-4),
            "{} is not inside {}",
            part.id,
            part.inside
        );
    }
    let surface: f64 = shapes
        .iter()
        .filter(|(id, _)| !out.internal_parts.iter().any(|p| &p.id == id))
        .map(|(_, s)| s.faces().iter().map(|f| f.area()).sum::<f64>())
        .sum();
    let exposed = out.exposed_area.value();
    assert!(
        exposed > 0.0 && exposed <= surface + 1e-12,
        "{exposed} vs {surface}"
    );
    let by_direction: f64 = out
        .exposed_by_direction
        .iter()
        .map(|d| d.area.value())
        .sum();
    let by_part: f64 = out.exposed_by_part.iter().map(|p| p.area.value()).sum();
    assert!(close(by_direction, exposed) && close(by_part, exposed));
}
