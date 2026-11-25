//! 外面の直達日射の影のテスト。直達日射だけが日照割合倍になり、アルベドと惑星の赤外は変わらない（広がった光源は影を数えない）。

use super::*;
use crate::structure::external_geometry::ShadowingInput;
use crate::structure::interference::InterferenceBox;
use crate::thermal::network::{ThermalNetwork, ThermalPlate, ThermalSurface, tests::node};
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

/// 外面 "P"（0.01 m²、法線 +Z、α = 0.9、ε = 0.8）。箱は 1 m × 1 m の板と、その上面の半分（x ∈ [0, 0.5]）を覆う箱。
/// `cells` があれば、板 P を (u, v) の升に分ける（#340）。
fn network(enabled: Option<bool>, cells: Option<[u32; 2]>) -> ThermalNetwork {
    let frame = boxed("P", [0.0, 0.0, 0.0], [1.0, 1.0, 0.01]);
    ThermalNetwork {
        mounts: None,
        nodes: vec![node("P", None)],
        conductive: vec![],
        radiative: vec![],
        enclosure: None,
        view_factors: None,
        surfaces: vec![ThermalSurface {
            id: "P".to_owned(),
            node: "P".to_owned(),
            area: Qty::from_raw(0.01),
            normal: Vec3::Z,
            count: 1,
            absorptivity: Some(0.9),
            emissivity: Some(0.8),
            coatings: vec![],
        }],
        shadowing: Some(ShadowingInput {
            boxes: vec![
                frame.clone(),
                boxed("A", [0.25, 0.0, TOP + 0.15], [0.5, 1.0, 0.1]),
            ],
            enabled,
        }),
        plates: cells
            .map(|cells| ThermalPlate {
                part: "P".to_owned(),
                cells,
                conductivity: Qty::from_raw(100.0),
                frame,
                attachments: Vec::new(),
            })
            .into_iter()
            .collect(),
    }
}

fn plate(enabled: Option<bool>) -> Compiled {
    Compiled::new(&network(enabled, None)).unwrap()
}

/// 真上の太陽と、真上にある地球（高度 500 km。アルベドと赤外が入る）。
fn overhead(sun: Option<Vec3>) -> ThermalExposure {
    ThermalExposure::Fixed {
        sun_dir_body: sun,
        nadir_body: Some(-Vec3::Z),
        altitude: Qty::from_raw(500.0e3),
        central: CentralBody::Earth,
    }
}

/// 手計算: 太陽に正対した 0.01 m² の面（α 0.9、1361 W/m²）の直達日射は 12.249 W。半分が影なら 6.1245 W。
/// 影を数えない・数えるの差は直達日射の半分（0.9 · 1361 · 0.01 · 0.5）だけで、アルベドと赤外は同じ。
#[test]
fn only_the_direct_sun_is_multiplied_by_the_lit_fraction() {
    let env = ThermalEnvironment::default();
    let shaded = exposure_heat(&plate(None), &env, &overhead(Some(Vec3::Z))).unwrap();
    let off = exposure_heat(&plate(Some(false)), &env, &overhead(Some(Vec3::Z))).unwrap();
    let direct = 0.9 * 1361.0 * 0.01;
    assert!(
        (off.q[0] - shaded.q[0] - 0.5 * direct).abs() < 1e-12,
        "{} {}",
        off.q[0],
        shaded.q[0]
    );
    // 日陰（太陽なし）は、影を数えても数えなくても同じ（地球の赤外だけ）
    let dark = exposure_heat(&plate(None), &env, &overhead(None)).unwrap();
    let dark_off = exposure_heat(&plate(Some(false)), &env, &overhead(None)).unwrap();
    assert_eq!(dark.q[0], dark_off.q[0]);
}

#[test]
fn the_report_says_shadowing_was_applied_and_gives_the_lit_fractions() {
    let env = ThermalEnvironment::default();
    let shaded = exposure_heat(&plate(None), &env, &overhead(Some(Vec3::Z))).unwrap();
    let report = shaded.shadowing.expect("applied");
    assert_eq!(report.parts.len(), 1);
    let p = &report.parts[0];
    assert_eq!((p.id.as_str(), p.samples), ("P", 1));
    assert!((p.min.value() - 0.5).abs() < 1e-12 && (p.mean.value() - 0.5).abs() < 1e-12);
    let off = exposure_heat(&plate(Some(false)), &env, &overhead(Some(Vec3::Z))).unwrap();
    assert!(off.shadowing.is_none());
}

/// 面の部品に取り付けられた部品（面の上のセルなど。箱は面の平面に接する）の接地面は、熱の外面の日照として数える（面の表面に含める慣例）:
/// 載っているだけなら、影を数えない値と厳密に等しい。取り付けられていない部品が接しているときは、面を覆うので数えない（#446。下の覆いのテスト）。
#[test]
fn a_part_resting_on_the_face_does_not_remove_its_direct_sun() {
    let mut net = plate(None);
    // 板の上面に載る箱（x ∈ [0, 0.5]、高さ 0.1）に替える
    let mut mounted = boxed("R", [0.25, 0.0, TOP + 0.05], [0.5, 1.0, 0.1]);
    mounted.mount_ancestors = vec!["P".to_owned()];
    let resting = ShadowingInput {
        boxes: vec![boxed("P", [0.0, 0.0, 0.0], [1.0, 1.0, 0.01]), mounted],
        enabled: None,
    };
    net.shading = crate::structure::external_geometry::LitSurfaces::new(
        &resting,
        crate::structure::external_geometry::Footprint::Credited,
        [("P", Vec3::Z)],
    );
    let env = ThermalEnvironment::default();
    let shaded = exposure_heat(&net, &env, &overhead(Some(Vec3::Z))).unwrap();
    let off = exposure_heat(&plate(Some(false)), &env, &overhead(Some(Vec3::Z))).unwrap();
    assert_eq!(shaded.q[0], off.q[0]);
}

/// 影を落とす箱 "A" は外面を持たないので、遮られた日射（0.9 · 1361 · 0.01 · 0.5 = 6.1245 W）は吸収されない: 報告に部品と電力が載り、
/// 影を数えた入熱 6.1245 W に対して割合 1（> 2 %）で警告が出る。外面を持てば部品の一覧は空で警告は無い。
#[test]
fn intercepted_sunlight_of_a_part_without_a_surface_is_reported_and_warned() {
    let env = ThermalEnvironment::default();
    let exposure = ThermalExposure::Fixed {
        sun_dir_body: Some(Vec3::Z),
        nadir_body: None,
        altitude: Qty::from_raw(500.0e3),
        central: CentralBody::Earth,
    };
    let report = exposure_heat(&plate(None), &env, &exposure)
        .unwrap()
        .shadowing
        .unwrap();
    assert_eq!(report.casters, vec!["A".to_owned()]);
    let lost = report.lost.as_ref().unwrap();
    assert_eq!(lost.parts, vec!["A".to_owned()]);
    let direct = 0.9 * 1361.0 * 0.01;
    assert!((lost.power.value() - 0.5 * direct).abs() < 1e-9);
    assert!((lost.fraction.value() - 1.0).abs() < 1e-9);
    let check = shadow_loss_check(&report, "shadow_loss").unwrap();
    assert_eq!(check.verdict, crate::spec::Verdict::Warning);
    // 影を数えない: 報告も警告も無い
    let off = exposure_heat(&plate(Some(false)), &env, &exposure).unwrap();
    assert!(off.shadowing.is_none());
}

/// 板を升に分けても影は面ごとに 1 つの日照割合で、升には面積の割合で配った直達日射にそのまま掛かる（#340 との合成）。
/// 板を 2 × 1 に分けたときの直達日射の合計は、分けない面の影ありの値（0.5 · 0.9 · 1361 · 0.01 = 6.1245 W）と等しく（保存）、
/// 2 つの升は半分ずつ。影を切れば分けた板も影なしの値（12.249 W）。分けても影の報告は面ごとに 1 つ。
#[test]
fn shadowing_composes_with_plate_subdivision_by_area() {
    let env = ThermalEnvironment::default();
    let exposure = ThermalExposure::Fixed {
        sun_dir_body: Some(Vec3::Z),
        nadir_body: None,
        altitude: Qty::from_raw(500.0e3),
        central: CentralBody::Earth,
    };
    let heat = |enabled: Option<bool>, cells: Option<[u32; 2]>| {
        let net = Compiled::new(&network(enabled, cells)).unwrap();
        exposure_heat(&net, &env, &exposure).unwrap()
    };
    let (whole, split) = (heat(None, None), heat(None, Some([2, 1])));
    assert_eq!(split.q.len(), 2);
    let total: f64 = split.q.iter().sum();
    assert!((total - whole.q[0]).abs() < 1e-12);
    assert!((total - 0.5 * 0.9 * 1361.0 * 0.01).abs() < 1e-9);
    assert!((split.q[0] - split.q[1]).abs() < 1e-12);
    // 影を切る: 分けた板も分けない板も影なしの値
    let (off_whole, off_split) = (heat(Some(false), None), heat(Some(false), Some([2, 1])));
    let off_total: f64 = off_split.q.iter().sum();
    assert!((off_total - off_whole.q[0]).abs() < 1e-12);
    assert!((off_total - 0.9 * 1361.0 * 0.01).abs() < 1e-9);
    assert_eq!(split.shadowing.unwrap().parts.len(), 1);
}

/// 外面 "P" の上面（0.01 m²）の半分（x ∈ [0, 0.5]）に、箱 "R" が載る（底が面の平面。面に接する）。`lifted` は載る箱を浮かす距離 [m]、
/// `mounted` は箱を面の部品 "P" に取り付ける。箱は外面を持たない。
fn with_resting_box(enabled: Option<bool>, lifted: f64, mounted: bool) -> Compiled {
    let mut net = network(enabled, None);
    let mut resting = boxed("R", [0.25, 0.0, TOP + 0.05 + lifted], [0.5, 1.0, 0.1]);
    if mounted {
        resting.mount_ancestors = vec!["P".to_owned()];
    }
    let input = net.shadowing.as_mut().unwrap();
    input.boxes.pop(); // 影の箱 "A" は外す
    input.boxes.push(resting);
    Compiled::new(&net).unwrap()
}

/// 手計算: 面の半分を箱が覆うと、覆われた半分は宇宙へ放射せず（`ε σ A` の係数が半分: 0.8 · σ · 0.01 · 0.5）、
/// アルベドと惑星の赤外も半分しか受けない。影を切れば（`enabled = false`）覆いも数えず、従来どおり全面。
/// 直達日射は影（箱の真下は日陰）だけで半分で、覆いを重ねても 4 分の 1 にはならない（二重に数えない）。
#[test]
fn a_half_covered_face_exchanges_half_with_the_environment() {
    let covered = with_resting_box(None, 0.0, false);
    let off = with_resting_box(Some(false), 0.0, false);
    let sigma = sat_astro::constants::SIGMA_SB;
    assert!((covered.space[0] - 0.8 * sigma * 0.01 * 0.5).abs() < 1e-18);
    assert!((off.space[0] - 0.8 * sigma * 0.01).abs() < 1e-18);
    assert!((covered.surfaces[0].exposed - 0.5).abs() < 1e-12);

    let env = ThermalEnvironment::default();
    // 地球が見える向き（面の法線 +Z に対し、地球は +Z 側）で太陽なし: 惑星の赤外だけ。半分
    let earth = ThermalExposure::Fixed {
        sun_dir_body: None,
        nadir_body: Some(Vec3::Z),
        altitude: Qty::from_raw(500.0e3),
        central: CentralBody::Earth,
    };
    let ir_covered = exposure_heat(&covered, &env, &earth).unwrap().q[0];
    let ir_off = exposure_heat(&off, &env, &earth).unwrap().q[0];
    assert!(ir_off > 0.0);
    assert!((ir_covered - 0.5 * ir_off).abs() < 1e-12 * ir_off);

    // 真上の太陽（地球なし）: 直達日射は 0.9 · 1361 · 0.01 の半分（覆いと影が同じ範囲）
    let sun = ThermalExposure::Fixed {
        sun_dir_body: Some(Vec3::Z),
        nadir_body: None,
        altitude: Qty::from_raw(500.0e3),
        central: CentralBody::Earth,
    };
    let direct = 0.9 * 1361.0 * 0.01;
    let q = exposure_heat(&covered, &env, &sun).unwrap().q[0];
    assert!((q - 0.5 * direct).abs() < 1e-9);
}

/// 面の部品に取り付けられている箱（面の光学特性に含まれる）は、載っていても覆わず、向き合う面にも数えない（全面が環境に開く）。
/// 面から 1 mm 浮いている箱は接していないので覆いではないが、面の前に平行に向き合う（箱の底の面が面の半分の大きさ）ので、
/// 形態係数だけ環境から塞ぐ: 平行板の隙間が 0 の極限は面積の割合 0.5 で、1 mm の隙間では 0.49 以上 0.5 未満。覆われた面積は 0。
#[test]
fn a_mounted_box_is_ignored_and_a_floating_box_blocks_by_view_factor() {
    let sigma = sat_astro::constants::SIGMA_SB;
    let mounted = with_resting_box(None, 0.0, true);
    assert!((mounted.surfaces[0].exposed - 1.0).abs() < 1e-15);
    assert!((mounted.space[0] - 0.8 * sigma * 0.01).abs() < 1e-18);
    let floating = with_resting_box(None, 0.001, false);
    let blocked = 1.0 - floating.surfaces[0].exposed;
    assert!((0.49..0.5).contains(&blocked), "{blocked}");
    let report = exposure_heat(
        &floating,
        &ThermalEnvironment::default(),
        &overhead(Some(Vec3::Z)),
    )
    .unwrap()
    .shadowing
    .unwrap();
    let p = &report.parts[0];
    assert_eq!(p.covered_area.unwrap().value(), 0.0);
    assert!((p.facing.unwrap().value() - blocked).abs() < 1e-12);
    assert_eq!(p.facing_by, Some(vec!["R".to_owned()]));
    // 影を切れば（`enabled = false`）覆いも向き合いも数えない
    assert_eq!(
        with_resting_box(Some(false), 0.001, false).surfaces[0].exposed,
        1.0
    );
}

/// 報告に、面ごとの覆われた面積（箱の面 1 m² の半分 = 0.5 m²）と覆う部品が載る。外面を持たない覆う部品 "R" は、
/// アルベドや惑星の赤外を受ける面を覆うので、吸収されない入熱の部品に数える。影を切れば報告そのものが無い。
#[test]
fn the_report_gives_the_covered_area_and_the_covering_part() {
    let env = ThermalEnvironment::default();
    let sun = ThermalExposure::Fixed {
        sun_dir_body: Some(Vec3::Z),
        nadir_body: Some(Vec3::Z),
        altitude: Qty::from_raw(500.0e3),
        central: CentralBody::Earth,
    };
    let report = exposure_heat(&with_resting_box(None, 0.0, false), &env, &sun)
        .unwrap()
        .shadowing
        .unwrap();
    let p = &report.parts[0];
    assert!((p.covered_area.unwrap().value() - 0.5).abs() < 1e-12);
    assert!((p.covered.unwrap().value() - 0.5).abs() < 1e-12);
    assert_eq!(p.covered_by, Some(vec!["R".to_owned()]));
    assert_eq!(report.coverers, Some(vec!["R".to_owned()]));
    let lost = report.lost.unwrap();
    assert_eq!(lost.parts, vec!["R".to_owned()]);
    assert!(lost.power.value() > 0.0);
    let off = exposure_heat(&with_resting_box(Some(false), 0.0, false), &env, &sun).unwrap();
    assert!(off.shadowing.is_none());
}

/// 外面を持つ部品が面を覆うときは、その入熱は覆う部品が吸収するので、失われた入熱の部品に数えない（報告の `lost` は 0）。
#[test]
fn a_covering_part_with_its_own_surface_loses_nothing() {
    let mut net = network(None, None);
    let input = net.shadowing.as_mut().unwrap();
    input.boxes.pop();
    input
        .boxes
        .push(boxed("R", [0.25, 0.0, TOP + 0.05], [0.5, 1.0, 0.1]));
    net.surfaces.push(ThermalSurface {
        id: "R".to_owned(),
        node: "P".to_owned(),
        area: Qty::from_raw(0.01),
        normal: Vec3::Z,
        count: 1,
        absorptivity: Some(0.9),
        emissivity: Some(0.8),
        coatings: vec![],
    });
    let env = ThermalEnvironment::default();
    let sun = ThermalExposure::Fixed {
        sun_dir_body: Some(Vec3::Z),
        nadir_body: Some(Vec3::Z),
        altitude: Qty::from_raw(500.0e3),
        central: CentralBody::Earth,
    };
    let report = exposure_heat(&Compiled::new(&net).unwrap(), &env, &sun)
        .unwrap()
        .shadowing
        .unwrap();
    let lost = report.lost.unwrap();
    assert!(lost.parts.is_empty());
    assert!(lost.power.value().abs() < 1e-12);
}
