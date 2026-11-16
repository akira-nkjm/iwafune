//! 地球以外の中心天体: デモの機体を月の高度 100 km の極軌道に置き（設計はコードで書き換える。デモのデータは変えない）、
//! 熱（定常・非定常）と β・蝕の季節解析が月の環境の値で動くこと。既定値の無い天体（火星）は環境の値を不足入力にする。

use fune_model::Design;
use fune_model::design::CentralBodyKind;
use fune_model::value::Quantity;
use fune_projection::{Context, aocs, orbit, power, thermal};
use sat_analysis::aocs::unload_time::{MtqFieldGap, MtqFieldReason};
use sat_analysis::aocs::{RwUnloading, UnloadTime};
use sat_analysis::orbit::EclipseSeason;
use sat_analysis::spec::{Analysis, Verdict};
use sat_analysis::thermal::{ThermalSteady, ThermalTransient};
use sat_astro::ephem::CentralBody;
use std::path::Path;

fn demo() -> Design {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    fune_store::load(&root)
        .expect("the demo should load")
        .design
}

/// 中心天体 `body` の高度 100 km の極軌道（太陽同期・LTDN は外す）。熱の環境条件の値は書かず、プリセットだけにする。
fn about(body: CentralBodyKind) -> Design {
    let mut d = demo();
    let o = d.conditions.orbit.as_mut().expect("orbit");
    o.central = Some(body);
    o.altitude = Some(Quantity::new(100.0, "km"));
    o.sun_synchronous = None;
    o.ltdn = None;
    o.inclination = Some(Quantity::new(90.0, "deg"));
    o.raan = Some(Quantity::new(0.0, "deg"));
    o.duration = Some(Quantity::new(30.0, "d"));
    let t = d.analysis.thermal.as_mut().expect("thermal settings");
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

/// 地球以外の中心天体で `sun_synchronous = true`・`ltdn` を使うと、地球の値を黙って入れず、
/// `sun_synchronous`・`ltdn` を不足入力として名指しする。地球は従来どおり値が入る。
#[test]
fn non_earth_sun_synchronous_and_ltdn_are_reported_not_earths_value() {
    let paths = |body: CentralBodyKind| -> Vec<String> {
        let mut d = about(body);
        let o = d.conditions.orbit.as_mut().expect("orbit");
        o.sun_synchronous = Some(true);
        o.inclination = None;
        o.raan = None;
        o.ltdn = Some("10:30".into());
        let resolved = fune_model::resolve(&d);
        let ctx = Context::new(&d, &resolved, Some("hot_imaging")).unwrap();
        let mut m = fune_projection::MissingInputs::new();
        let spec = ctx.orbit(&mut m);
        assert_eq!(spec.is_some(), m.is_empty());
        m.into_vec().iter().map(|x| x.path.clone()).collect()
    };
    for body in [CentralBodyKind::Moon, CentralBodyKind::Mars] {
        let missing = paths(body);
        for key in ["sun_synchronous", "ltdn"] {
            let path = format!("conditions.orbit.{key}");
            assert!(missing.contains(&path), "{body:?}: {missing:?}");
        }
    }
    assert!(paths(CentralBodyKind::Earth).is_empty());
}

/// 月・火星のまわりで RW アンロード頻度を解くと、空力・磁気トルクが 0 になる理由が判定に出る
/// （月は大気・磁場が無いので `Ok`、火星は大気のモデルが無いので `Warning`）。地球のデモでは判定を足さない。
#[test]
fn rw_unloading_about_other_bodies_states_the_environment_models() {
    let run = |body: CentralBodyKind| {
        let design = about(body);
        let resolved = fune_model::resolve(&design);
        let ctx = Context::new(&design, &resolved, Some("hot_imaging")).unwrap();
        let input = aocs::rw_unloading(&ctx).unwrap();
        RwUnloading::run(&input).unwrap()
    };
    let note = |o: &sat_analysis::aocs::RwUnloadingOutput| {
        o.checks
            .iter()
            .find(|c| c.key == "environment_models")
            .cloned()
    };
    let moon = note(&run(CentralBodyKind::Moon)).expect("note for the Moon");
    assert_eq!(moon.verdict, Verdict::Ok);
    assert!(
        moon.message
            .contains("Aerodynamic torque is 0 for this body"),
        "{}",
        moon.message
    );
    let mars = note(&run(CentralBodyKind::Mars)).expect("note for Mars");
    assert_eq!(mars.verdict, Verdict::Warning);
    assert!(
        mars.message
            .contains("this body has atmosphere but there is no model for it"),
        "{}",
        mars.message
    );
    assert!(note(&run(CentralBodyKind::Earth)).is_none());
}

/// 地球以外で磁場を軌道から求める MTQ のアンロード時間は、磁場が 0 の「トルクが 0」とせず、解析を失敗させずに
/// MTQ の結果を外して理由（`mtq_field_gap`）を返す（月は磁場が無い）。
#[test]
fn unload_time_about_other_bodies_leaves_out_the_mtq_with_a_reason() {
    let run = |body| {
        let design = about(body);
        let resolved = fune_model::resolve(&design);
        let ctx = Context::new(&design, &resolved, Some("hot_imaging")).unwrap();
        let input = aocs::unload_time(&ctx).unwrap();
        UnloadTime::run(&input).unwrap()
    };
    let moon = run(CentralBodyKind::Moon);
    assert!(moon.mtq.is_none());
    // デモにスラスタは無いので、RCS で足りない
    assert_eq!(
        moon.mtq_field_gap,
        Some(MtqFieldGap {
            reason: MtqFieldReason::Absent,
            covered_by_rcs: false,
        })
    );
    let earth = run(CentralBodyKind::Earth);
    assert!(earth.mtq.is_some() && earth.mtq_field_gap.is_none());
}

/// 太陽が中心天体の軌道は、β と蝕の季節解析が（`beta_angle_rad` の前に）エラーで断る。
#[test]
fn eclipse_season_about_the_sun_is_an_error() {
    let design = about(CentralBodyKind::Sun);
    let resolved = fune_model::resolve(&design);
    let ctx = Context::new(&design, &resolved, Some("hot_imaging")).unwrap();
    let input = orbit::eclipse_season(&ctx).unwrap();
    let err = EclipseSeason::run(&input).unwrap_err().to_string();
    assert!(err.contains("An orbit around the Sun"), "{err}");
}
