//! 外面の直達日射の影のテスト。直達日射だけが日照割合倍になり、アルベドと惑星の赤外は変わらない（広がった光源は影を数えない）。

use super::*;
use crate::structure::external_geometry::ShadowingInput;
use crate::structure::interference::InterferenceBox;
use crate::thermal::network::{ThermalNetwork, ThermalSurface, tests::node};
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
fn plate(enabled: Option<bool>) -> Compiled {
    Compiled::new(&ThermalNetwork {
        mounts: None,
        nodes: vec![node("p", None)],
        conductive: vec![],
        radiative: vec![],
        enclosure: None,
        surfaces: vec![ThermalSurface {
            id: "P".to_owned(),
            node: "p".to_owned(),
            area: Qty::from_raw(0.01),
            normal: Vec3::Z,
            count: 1,
            absorptivity: Some(0.9),
            emissivity: Some(0.8),
            coatings: vec![],
        }],
        shadowing: Some(ShadowingInput {
            boxes: vec![
                boxed("P", [0.0, 0.0, 0.0], [1.0, 1.0, 0.01]),
                boxed("A", [0.25, 0.0, TOP + 0.15], [0.5, 1.0, 0.1]),
            ],
            enabled,
        }),
    })
    .unwrap()
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

/// 面に載る部品（面の上のセルなど。箱は面の平面に接する）の接地面は、熱の外面の日照として数える（面の表面に含める慣例）:
/// 載っているだけなら、影を数えない値と厳密に等しい。
#[test]
fn a_part_resting_on_the_face_does_not_remove_its_direct_sun() {
    let mut net = plate(None);
    // 板の上面に載る箱（x ∈ [0, 0.5]、高さ 0.1）に替える
    let resting = ShadowingInput {
        boxes: vec![
            boxed("P", [0.0, 0.0, 0.0], [1.0, 1.0, 0.01]),
            boxed("R", [0.25, 0.0, TOP + 0.05], [0.5, 1.0, 0.1]),
        ],
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
