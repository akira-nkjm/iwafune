//! 太陽電池の発電の影（部品どうしの影）のテスト。影を数えた発電は、影なしの発電 × 面の日照割合（幾何は
//! `structure::external_geometry::tests::shading` で手計算している）。影を切る・影が無いときは、従来の値と厳密に等しい。

use super::*;
use crate::power::OrbitGeneration;
use crate::power::orbit_generation::OrbitGenerationInput;
use crate::power::testkit::{leo_timeline, sun_pointing_z};
use crate::spec::Analysis;
use crate::structure::interference::InterferenceBox;
use sat_astro::math::Quat;

const TOP: f64 = 0.005;

fn boxed(id: &str, c: [f64; 3], size: [f64; 3]) -> InterferenceBox {
    InterferenceBox {
        id: id.to_owned(),
        count: 1,
        position: Vec3::new(c[0], c[1], c[2]),
        orientation: Quat::IDENTITY,
        size: Vec3::new(size[0], size[1], size[2]),
        mount_ancestors: vec![],
    }
}

/// 1 m² のパネル "P"（面 +Z、板 1 m × 1 m × 10 mm の上面）。`others` は遮る箱。
fn array(others: Vec<InterferenceBox>, enabled: Option<bool>) -> SolarArray {
    let mut boxes = vec![boxed("P", [0.0, 0.0, 0.0], [1.0, 1.0, 0.01])];
    boxes.extend(others);
    SolarArray {
        panels: vec![SolarPanel {
            id: "P".to_owned(),
            area: Qty::from_raw(1.0),
            cell_efficiency: 0.3,
            packing_factor: Some(1.0),
            eol_factor: Some(1.0),
            normal: Vec3::Z,
        }],
        solar_flux: None,
        conversion_efficiency: None,
        shadowing: Some(ShadowingInput { boxes, enabled }),
    }
}

/// x ∈ [0, 0.5]（面の半分）の上、高さ 0.1〜0.2 にある箱。真上の光で日照割合 0.5。
fn half() -> InterferenceBox {
    boxed("A", [0.25, 0.0, TOP + 0.15], [0.5, 1.0, 0.1])
}

/// x ∈ [0.5, 1.5]（面の外）の、面から高さ 0.3〜0.4 の箱。+X 側 45° の光で日照割合 0.6。
fn side() -> InterferenceBox {
    boxed("S", [1.0, 0.0, TOP + 0.35], [1.0, 3.0, 0.1])
}

fn power(a: &SolarArray, sun: Vec3) -> f64 {
    let shading = a.shading();
    a.panel_powers_shaded(shading.as_ref(), sun, 1.0, a.flux(None), SolarLife::Bol)[0].value()
}

fn unshadowed(a: &SolarArray, sun: Vec3) -> f64 {
    a.panel_powers(sun, 1.0, a.flux(None), SolarLife::Bol)[0].value()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 * b.abs().max(1.0)
}

/// 手計算: 1 m² × 30 % × 1361 W/m² = 408.3 W。真上で面の半分が影 → 204.15 W。45° の光は cos 45° = 1/√2 に日照割合 0.6 を掛ける。
#[test]
fn shaded_generation_is_the_unshadowed_generation_times_the_lit_fraction() {
    let a = array(vec![half()], None);
    assert!(close(unshadowed(&a, Vec3::Z), 408.3));
    assert!(close(power(&a, Vec3::Z), 204.15));
    let a = array(vec![side()], None);
    let oblique = Vec3::new(1.0, 0.0, 1.0);
    let expect = 408.3 * std::f64::consts::FRAC_1_SQRT_2 * 0.6;
    assert!(close(power(&a, oblique), expect));
    assert!(close(power(&a, Vec3::Z), 408.3));
}

#[test]
fn switched_off_or_without_shadows_the_numbers_are_the_old_ones_exactly() {
    let dirs = [Vec3::Z, Vec3::new(1.0, 0.0, 1.0), Vec3::new(0.2, -0.4, 1.0)];
    let off = array(vec![half(), side()], Some(false));
    assert!(off.shading().is_none());
    let mut none = array(vec![half(), side()], None);
    none.shadowing = None;
    // 影を落とす箱が無い（板だけ）なら、影ありの経路でも同じ（日照割合 1 を掛けるだけ）
    let alone = array(vec![], None);
    for d in dirs {
        assert_eq!(power(&off, d), unshadowed(&off, d));
        assert_eq!(power(&none, d), unshadowed(&none, d));
        assert_eq!(power(&alone, d), unshadowed(&alone, d));
    }
}

#[test]
fn a_cell_flush_on_its_panel_is_not_shadowed() {
    // セルは大きなパネルの上面に載る。パネルの上面はセルの面の裏にあり、遮らない
    let mut a = array(vec![], None);
    a.shadowing = Some(ShadowingInput {
        boxes: vec![
            boxed("P", [0.0, 0.0, 0.001], [0.3, 0.3, 0.002]),
            boxed("BACK", [0.0, 0.0, -0.01], [2.0, 2.0, 0.02]),
        ],
        enabled: None,
    });
    for d in [Vec3::Z, Vec3::new(1.0, 0.0, 1.0)] {
        assert_eq!(power(&a, d), unshadowed(&a, d));
    }
}

fn orbit_input(a: SolarArray) -> OrbitGenerationInput {
    OrbitGenerationInput {
        timeline: leo_timeline(sun_pointing_z(), 2.0, 60.0),
        array: a,
        life: Some(SolarLife::Bol),
    }
}

/// 太陽指向（太陽は +Z で一定）の軌道: 日照中の平均発電は半分、日照割合の最小も平均も 0.5。
/// 影を切れば報告は無く、発電は影なしの値と厳密に等しい。
#[test]
fn orbit_generation_reports_the_lit_fractions_and_halves_the_generation() {
    let shaded = OrbitGeneration::run(&orbit_input(array(vec![half()], None))).unwrap();
    let mut plain_array = array(vec![half()], None);
    plain_array.shadowing = None;
    let plain = OrbitGeneration::run(&orbit_input(plain_array)).unwrap();
    assert!(close(
        shaded.sunlit_average.value(),
        0.5 * plain.sunlit_average.value()
    ));
    assert!(close(
        shaded.orbit_average.value(),
        0.5 * plain.orbit_average.value()
    ));
    let report = shaded.shadowing.expect("shadowing reported");
    assert_eq!(report.parts.len(), 1);
    let p = &report.parts[0];
    assert_eq!(p.id, "P");
    assert!(close(p.min.value(), 0.5) && close(p.mean.value(), 0.5) && p.samples > 0);
    assert!(plain.shadowing.is_none());
    let off = OrbitGeneration::run(&orbit_input(array(vec![half()], Some(false)))).unwrap();
    assert_eq!(off.orbit_average.value(), plain.orbit_average.value());
    assert!(off.shadowing.is_none());
}
