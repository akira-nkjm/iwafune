//! 太陽を中心天体にした軌道（惑星間の巡航）と、ケースの太陽距離（`sun_distance`）: デモの機体を 0.7〜1.5 AU の楕円に置き
//! （設計はコードで書き換える。デモのデータは変えない）、熱と電源が太陽距離のケースで動き、距離で結果が変わること。
//! 太陽を中心天体にしない軌道でケースが太陽距離を持つと、不足入力として名指しする。

use fune_model::Design;
use fune_model::design::{Case, CentralBodyKind};
use fune_model::value::Quantity;
use fune_projection::{Context, orbit, power, thermal};
use sat_analysis::orbit::sun_distance_extremes;
use sat_analysis::power::OrbitGeneration;
use sat_analysis::spec::Analysis;
use sat_analysis::thermal::ThermalSteady;
use sat_astro::constants::{AU, R_SUN};
use std::path::Path;

fn demo() -> Design {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    fune_store::load(&root)
        .expect("the demo should load")
        .design
}

/// 太陽を回る楕円（近日点 ≈ 0.7 AU・遠日点 ≈ 1.5 AU。長半径は太陽の半径を引いた高度で書く）。ケースの β と
/// 熱条件の太陽光・アルベド・赤外は外し（太陽中心では β を持てない）、近日点・遠日点のケースを足す。
fn cruise() -> Design {
    let mut d = demo();
    let o = d.conditions.orbit.as_mut().expect("orbit");
    o.central = Some(CentralBodyKind::Sun);
    o.altitude = Some(Quantity::new(1.1 * AU - R_SUN, "m"));
    o.eccentricity = Some(0.8 / 2.2);
    o.sun_synchronous = None;
    o.ltdn = None;
    o.inclination = Some(Quantity::new(1.0, "deg"));
    o.raan = Some(Quantity::new(0.0, "deg"));
    o.duration = Some(Quantity::new(1.0, "yr"));
    let t = d.analysis.thermal.as_mut().expect("thermal settings");
    for c in t.cases.values_mut() {
        c.solar_flux = None;
        c.albedo = None;
        c.planet_ir = None;
        c.beta = None;
    }
    for c in d.cases.values_mut() {
        c.beta = None;
    }
    for (id, au) in [("perihelion", 0.7), ("aphelion", 1.5)] {
        d.cases.insert(
            id.into(),
            Case {
                id: id.into(),
                label: None,
                description: None,
                mode: Some("NOMINAL".into()),
                phase: None,
                side: None,
                thermal_case: None,
                beta: None,
                orbits: None,
                sun_distance: Some(Quantity::new(au, "AU")),
            },
        );
    }
    d
}

/// ケースの太陽距離は軌道を、その半径の円軌道に置き換える（傾斜角・元期はそのまま）。ケースが無ければ楕円のまま。
#[test]
fn case_sun_distance_replaces_the_orbit_by_a_circle_at_that_distance() {
    let design = cruise();
    let resolved = fune_model::resolve(&design);
    let mut m = fune_projection::MissingInputs::new();
    let plain = Context::new(&design, &resolved, None)
        .unwrap()
        .orbit(&mut m)
        .unwrap();
    assert!(plain.elements.eccentricity > 0.3);
    let mut at = |id: &str| {
        let ctx = Context::new(&design, &resolved, Some(id)).unwrap();
        ctx.orbit(&mut m).unwrap()
    };
    let peri = at("perihelion");
    assert_eq!(peri.elements.eccentricity, 0.0);
    assert!((peri.elements.semi_major_axis.value() / (0.7 * AU) - 1.0).abs() < 1e-12);
    assert_eq!(
        peri.elements.inclination.value(),
        plain.elements.inclination.value()
    );
    assert_eq!(peri.epoch.to_iso(), plain.epoch.to_iso());
}

/// 近日点（0.7 AU）のケースは遠日点（1.5 AU）のケースより、熱は暖かく（太陽を向く面は ≈ 4.6 倍の太陽光）、
/// 電源の発電は (1.5/0.7)² 倍。ケースの距離で太陽光が決まる（環境の太陽光の強さは書かない）。
#[test]
fn thermal_and_power_differ_with_the_case_distance() {
    let design = cruise();
    let resolved = fune_model::resolve(&design);
    let run_thermal = |id: &str| {
        let ctx = Context::new(&design, &resolved, Some(id)).unwrap();
        let out = ThermalSteady::run(&thermal::steady(&ctx).unwrap()).unwrap();
        let c = &out.cases[0];
        assert_eq!(c.environment.albedo.value(), 0.0);
        assert!(c.environment.solar_flux.is_none());
        c.nodes
            .iter()
            .map(|n| n.temperature.value())
            .fold(f64::MIN, f64::max)
    };
    let (hot, cold) = (run_thermal("perihelion"), run_thermal("aphelion"));
    assert!(hot > cold + 20.0, "{hot} vs {cold}");
    let generation = |id: &str| {
        let ctx = Context::new(&design, &resolved, Some(id)).unwrap();
        let input = power::orbit_generation(&ctx).unwrap();
        OrbitGeneration::run(&input).unwrap().sunlit_average.value()
    };
    let ratio = generation("perihelion") / generation("aphelion");
    assert!(
        (ratio / (1.5f64 / 0.7).powi(2) - 1.0).abs() < 1e-6,
        "generation ratio {ratio}"
    );
}

/// 近日点・遠日点は軌道条件とミッション期間から求める（`orbit::sun_distance` → `sun_distance_extremes`）。1 年で 1 周が入る。
#[test]
fn sun_distance_extremes_from_the_design() {
    let design = cruise();
    let resolved = fune_model::resolve(&design);
    let ctx = Context::new(&design, &resolved, None).unwrap();
    let x = sun_distance_extremes(&orbit::sun_distance(&ctx).unwrap()).unwrap();
    // 長半径 = 1.1 AU（高度 + 太陽の半径）、e = 0.8/2.2 → a(1∓e) = 0.7・1.5 AU（周期は 1.15 年なので遠日点は期間の外のことがある）
    assert!((x.min.distance.value() / (0.7 * AU) - 1.0).abs() < 1e-3);
    assert!(x.max.distance.value() > x.min.distance.value());
}

/// 太陽を中心天体にしない軌道（地球周回）でケースが太陽距離を持つと、不足入力としてケースの `sun_distance` を指す。
#[test]
fn sun_distance_on_an_earth_orbit_is_reported() {
    let mut design = demo();
    design
        .cases
        .values_mut()
        .next()
        .expect("a case")
        .sun_distance = Some(Quantity::new(1.0, "AU"));
    let resolved = fune_model::resolve(&design);
    let id = design.cases.keys().next().unwrap().to_string();
    let ctx = Context::new(&design, &resolved, Some(&id)).unwrap();
    let err = power::orbit_generation(&ctx).unwrap_err();
    let paths: Vec<String> = err.into_vec().into_iter().map(|m| m.path).collect();
    assert!(
        paths.iter().any(|p| p.ends_with("sun_distance")),
        "{paths:?}"
    );
}
