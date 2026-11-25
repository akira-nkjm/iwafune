//! 面の部品に取り付けられた部品が自分の外面を持つときの、持ち主の面の扱い（#472）のテスト。
//! 取り付けられた部品が外面を持つなら、持ち主の面はその下を覆われたものとして数え、同じ面積を二度（吸収も放射も）数えない。
//! 外面を持たないなら、これまでどおり持ち主の表面に含める（接地面は日照のまま。覆いにも数えない）。

use super::shading_tests::{TOP, boxed, network, overhead};
use super::*;
use crate::thermal::network::{ThermalSurface, tests::node};

/// 外面 "P"（0.01 m²、法線 +Z、α = 0.9、ε = 0.8）の上面の半分（x ∈ [0, 0.5]）に、P に取り付けられた箱 "R" が載る（底が面の平面から `lifted` 浮く）。
/// `own_surface` なら R は自分の外面（0.005 m²、法線 +Z、α = 0.7、ε = 0.6、節点 "R"）を持つ。
fn with_mounted_box(enabled: Option<bool>, lifted: f64, own_surface: bool) -> Compiled {
    let mut net = network(enabled, None);
    let mut resting = boxed("R", [0.25, 0.0, TOP + 0.05 + lifted], [0.5, 1.0, 0.1]);
    resting.mount_ancestors = vec!["P".to_owned()];
    let input = net.shadowing.as_mut().unwrap();
    input.boxes.pop(); // 影の箱 "A" は外す
    input.boxes.push(resting);
    if own_surface {
        net.nodes.push(node("R", None));
        net.surfaces.push(ThermalSurface {
            id: "R".to_owned(),
            node: "R".to_owned(),
            area: Qty::from_raw(0.005),
            normal: Vec3::Z,
            count: 1,
            absorptivity: Some(0.7),
            emissivity: Some(0.6),
            coatings: vec![],
        });
    }
    Compiled::new(&net).unwrap()
}

/// 真上の太陽だけ（地球なし）。
fn sun_only() -> ThermalExposure {
    ThermalExposure::Fixed {
        sun_dir_body: Some(Vec3::Z),
        nadir_body: None,
        altitude: Qty::from_raw(500.0e3),
        central: CentralBody::Earth,
    }
}

/// 地球が真上に見える向きで太陽なし（惑星の赤外だけ）。
fn earth_only() -> ThermalExposure {
    ThermalExposure::Fixed {
        sun_dir_body: None,
        nadir_body: Some(Vec3::Z),
        altitude: Qty::from_raw(500.0e3),
        central: CentralBody::Earth,
    }
}

/// 手計算: 面 P（0.01 m²）の半分を、P に取り付けられ自分の外面を持つ R（0.005 m²）が覆う。
/// 持ち主 P は覆われた半分で宇宙へ放射せず（`ε σ A` の係数が半分: 0.8 · σ · 0.01 · 0.5）、R は自分の面積で放射する（0.6 · σ · 0.005）。
/// 直達日射: P は R の下が日陰で 0.9 · 1361 · 0.01 · 0.5、R は 0.7 · 1361 · 0.005（面積は一度ずつ）。惑星の赤外は P が半分。
#[test]
fn a_surfaced_mounted_part_covers_its_owner_and_exchanges_its_own_area() {
    let sigma = sat_astro::constants::SIGMA_SB;
    let net = with_mounted_box(None, 0.0, true);
    assert!((net.surfaces[0].exposed - 0.5).abs() < 1e-12);
    assert!((net.surfaces[1].exposed - 1.0).abs() < 1e-12);
    assert!((net.space[0] - 0.8 * sigma * 0.01 * 0.5).abs() < 1e-18);
    assert!((net.space[1] - 0.6 * sigma * 0.005).abs() < 1e-18);

    let env = ThermalEnvironment::default();
    let q = exposure_heat(&net, &env, &sun_only()).unwrap().q;
    assert!((q[0] - 0.9 * 1361.0 * 0.01 * 0.5).abs() < 1e-9);
    assert!((q[1] - 0.7 * 1361.0 * 0.005).abs() < 1e-9);

    let off = with_mounted_box(Some(false), 0.0, true);
    let ir = exposure_heat(&net, &env, &earth_only()).unwrap().q[0];
    let ir_off = exposure_heat(&off, &env, &earth_only()).unwrap().q[0];
    assert!(ir_off > 0.0 && (ir - 0.5 * ir_off).abs() < 1e-12 * ir_off);

    // 報告: 覆われた面積は箱の面 1 m² の半分、覆うのは R。R が自分の外面を持つので、吸収されない入熱は無い
    let report = exposure_heat(&net, &env, &sun_only())
        .unwrap()
        .shadowing
        .unwrap();
    assert_eq!(report.parts[0].covered_by, Some(vec!["R".to_owned()]));
    assert!((report.parts[0].covered_area.unwrap().value() - 0.5).abs() < 1e-12);
    assert!(report.lost.unwrap().parts.is_empty());
}

/// 面から 0.45 mm 浮いて取り付けられた（接していない）外面を持つ部品も、面の前に平行に向き合うので、持ち主の面を形態係数だけ塞ぐ
/// （平行板の隙間が 0 の極限は面積の割合 0.5 で、0.45 mm の隙間では 0.49 以上 0.5 未満）。直達日射は R の影で半分。
#[test]
fn a_surfaced_mounted_part_standing_off_blocks_its_owner_by_view_factor() {
    let net = with_mounted_box(None, 0.00045, true);
    let blocked = 1.0 - net.surfaces[0].exposed;
    assert!((0.49..0.5).contains(&blocked), "{blocked}");
    let q = exposure_heat(&net, &ThermalEnvironment::default(), &sun_only())
        .unwrap()
        .q;
    assert!((q[0] - 0.9 * 1361.0 * 0.01 * 0.5).abs() < 1e-9);
}

/// 自分の外面を持たない取り付けられた部品（センサーなど）は、これまでどおり持ち主の表面に含まれる:
/// 覆いにも向き合いにも数えず、直達日射も持ち主の面積のまま。影を数えない値と厳密に等しい。
#[test]
fn an_unsurfaced_mounted_part_stays_lumped_into_its_owner() {
    let sigma = sat_astro::constants::SIGMA_SB;
    let lumped = with_mounted_box(None, 0.0, false);
    let off = with_mounted_box(Some(false), 0.0, false);
    assert!((lumped.surfaces[0].exposed - 1.0).abs() < 1e-15);
    assert!((lumped.space[0] - 0.8 * sigma * 0.01).abs() < 1e-18);
    let env = ThermalEnvironment::default();
    for exposure in [sun_only(), earth_only(), overhead(Some(Vec3::Z))] {
        let a = exposure_heat(&lumped, &env, &exposure).unwrap().q[0];
        let b = exposure_heat(&off, &env, &exposure).unwrap().q[0];
        assert_eq!(a, b);
    }
}

/// 影を切れば（`enabled = false`）、外面を持つ取り付けられた部品があっても何も覆わず、持ち主は全面で放射・吸収する（従来の数値）。
#[test]
fn with_shadowing_off_a_surfaced_mounted_part_covers_nothing() {
    let sigma = sat_astro::constants::SIGMA_SB;
    let off = with_mounted_box(Some(false), 0.0, true);
    assert_eq!(off.surfaces[0].exposed, 1.0);
    assert!((off.space[0] - 0.8 * sigma * 0.01).abs() < 1e-18);
    let env = ThermalEnvironment::default();
    let q = exposure_heat(&off, &env, &sun_only()).unwrap();
    assert!(q.shadowing.is_none());
    assert!((q.q[0] - 0.9 * 1361.0 * 0.01).abs() < 1e-9);
}
