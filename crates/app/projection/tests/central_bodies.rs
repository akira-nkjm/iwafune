//! 地球以外の中心天体: デモの機体を月の高度 100 km の極軌道に置き（設計はコードで書き換える。デモのデータは変えない）、
//! 熱（定常・非定常）と β・蝕の季節解析が月の環境の値で動くこと。既定値の無い天体（火星）は環境の値を不足入力にする。

use fune_model::Design;
use fune_model::design::CentralBodyKind;
use fune_model::value::Quantity;
use fune_projection::{Context, orbit, power, thermal};
use sat_analysis::orbit::EclipseSeason;
use sat_analysis::spec::Analysis;
use sat_analysis::thermal::{ThermalSteady, ThermalTransient};
use sat_astro::ephem::CentralBody;
use std::path::Path;

fn demo() -> Design {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    fune_store::load(&root).expect("デモを読めること").design
}

/// 中心天体 `body` の高度 100 km の極軌道（太陽同期・LTDN は外す）。熱の環境条件の値は書かず、プリセットだけにする。
fn about(body: CentralBodyKind) -> Design {
    let mut d = demo();
    let o = d.conditions.orbit.as_mut().expect("軌道");
    o.central = Some(body);
    o.altitude = Some(Quantity::new(100.0, "km"));
    o.sun_synchronous = None;
    o.ltdn = None;
    o.inclination = Some(Quantity::new(90.0, "deg"));
    o.raan = Some(Quantity::new(0.0, "deg"));
    o.duration = Some(Quantity::new(30.0, "d"));
    let t = d.analysis.thermal.as_mut().expect("熱の設定");
    for c in t.cases.values_mut() {
        c.solar_flux = None;
        c.albedo = None;
        c.planet_ir = None;
    }
    d
}

/// 月周回: 熱の定常・非定常が月のアルベド 0.12 と地方時で変わる赤外（夜側 ≈ 5.4 W/m²）で動き、高温のケースは
/// 低温のケースより平均でも最高でも暖かい。季節解析の最長の蝕は高度 100 km の月の影（β = 0 で ≈ 46 分）以下。
#[test]
fn lunar_orbit_thermal_and_eclipse_use_lunar_environment() {
    let design = about(CentralBodyKind::Moon);
    let resolved = fune_model::resolve(&design);
    let ctx = Context::new(&design, &resolved, Some("hot_imaging")).unwrap();
    let steady = thermal::steady(&ctx).unwrap();
    let out = ThermalSteady::run(&steady).unwrap();
    let case = |id: &str| out.cases.iter().find(|c| c.id == id).unwrap();
    let (hot, cold) = (case("hot"), case("cold"));
    for c in [hot, cold] {
        assert_eq!(c.environment.albedo.value(), 0.12);
        assert!(c.environment.planet_ir.is_none());
        assert!((c.environment.night_ir.unwrap().value() - 5.387).abs() < 1e-2);
    }
    assert_eq!(hot.environment.solar_flux.unwrap().value(), 1414.0);
    // 高温（β 60°・発熱の多いモード）は低温（β 0・セーフモード）より平均で暖かい
    let mean = |c: &sat_analysis::thermal::steady::ThermalSteadyCaseResult| {
        c.nodes.iter().map(|n| n.temperature.value()).sum::<f64>() / c.nodes.len() as f64
    };
    let max = |c: &sat_analysis::thermal::steady::ThermalSteadyCaseResult| {
        c.nodes
            .iter()
            .map(|n| n.temperature.value())
            .fold(0.0, f64::max)
    };
    assert!(mean(hot) > mean(cold), "{} {}", mean(hot), mean(cold));
    assert!(max(hot) > max(cold));
    let transient = thermal::transient(&ctx).unwrap();
    assert_eq!(transient.timeline.orbit().central, CentralBody::Moon);
    let t = ThermalTransient::run(&transient).unwrap();
    assert!(t.environment.night_ir.is_some());

    let season = orbit::eclipse_season(&ctx).unwrap();
    assert_eq!(season.orbit.central, CentralBody::Moon);
    let s = EclipseSeason::run(&season).unwrap();
    let longest = s.max_eclipse_duration.value();
    assert!(longest > 0.0 && longest < 47.0 * 60.0, "{longest}");
}

/// 既定値の無い天体（火星）: 熱の環境条件のアルベド・赤外と、プリセットの 1 AU の太陽光の代わりの太陽光の強さ、
/// 電源の逆算の太陽距離を不足入力にする。
#[test]
fn mars_orbit_requires_environment_values() {
    let design = about(CentralBodyKind::Mars);
    let resolved = fune_model::resolve(&design);
    let ctx = Context::new(&design, &resolved, Some("hot_imaging")).unwrap();
    let missing: Vec<String> = thermal::steady(&ctx)
        .unwrap_err()
        .into_vec()
        .iter()
        .filter_map(|m| m.target.as_ref().map(ToString::to_string))
        .collect();
    for key in ["albedo", "planet_ir", "solar_flux"] {
        let path = format!("analysis.thermal.case.hot.{key}");
        assert!(missing.contains(&path), "{path}: {missing:?}");
    }
    let sizing: Vec<String> = power::sizing(&ctx)
        .unwrap_err()
        .into_vec()
        .iter()
        .filter_map(|m| m.target.as_ref().map(ToString::to_string))
        .collect();
    assert!(
        sizing.contains(&"analysis.power.sizing.sun_distance".to_owned()),
        "{sizing:?}"
    );
}
