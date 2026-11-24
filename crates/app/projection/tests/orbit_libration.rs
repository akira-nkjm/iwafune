//! 軌道の出どころ `libration`（ラグランジュ点・ハロー軌道）: 書いた天体・点・振幅・枝・元期から、周期的な回転座標系の軌跡ができること
//! （原点は副天体、固定点の繰り返しは公転周期、ハローは補正した周期）。`orbit.geometry` が走り、太陽–地球 L2 では
//! 太陽–探査機–地球の角が 0°・太陽–地球–探査機の角が 180° になること（手で導ける）、ケプラー要素が要る解析は「対象外」を返し、
//! 足りない欄・天体の組の誤り・次元の違う単位は欄を指す不足入力になること。設計はコードで書き換える（デモのデータは変えない）。

use fune_model::Design;
use fune_model::design::{
    CentralBodyKind, HaloBranchKind, LibrationPointKind, OrbitSettings, OrbitSource,
};
use fune_model::value::Quantity;
use fune_projection::{Context, MissingInput, MissingInputs, orbit};
use sat_analysis::orbit::Geometry;
use sat_analysis::spec::Analysis;
use sat_astro::ephem::CentralBody;
use sat_astro::orbit::{Cycle, Trajectory};
use sat_astro::time::Epoch;
use std::path::Path;

const EPOCH: &str = "2030-01-01T00:00:00Z";

fn demo() -> Design {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    fune_store::load(&root)
        .expect("the demo should load")
        .design
}

fn l2() -> OrbitSettings {
    OrbitSettings {
        source: Some(OrbitSource::Libration),
        epoch: Some(EPOCH.to_owned()),
        primary: Some(CentralBodyKind::Sun),
        secondary: Some(CentralBodyKind::Earth),
        point: Some(LibrationPointKind::L2),
        ..OrbitSettings::default()
    }
}

fn halo() -> OrbitSettings {
    OrbitSettings {
        amplitude_z: Some(Quantity::new(120_000.0, "km")),
        branch: Some(HaloBranchKind::North),
        ..l2()
    }
}

fn with_orbit(o: OrbitSettings) -> Design {
    let mut d = demo();
    d.conditions.orbit = Some(o);
    d.phases.clear();
    d
}

fn trajectory(d: &Design) -> Result<Trajectory, Vec<MissingInput>> {
    let resolved = fune_model::resolve(d);
    let ctx = Context::new(d, &resolved, None).expect("context");
    let mut m = MissingInputs::new();
    ctx.trajectory(&mut m).ok_or_else(|| m.into_vec())
}

/// 固定点は副天体（地球）が原点・周期的（公転周期、約 365 日）、ハローは約半年の周期。どちらも要素を持たず高度も無い。
#[test]
fn the_libration_orbit_is_a_periodic_trajectory_about_the_secondary() {
    let fixed = trajectory(&with_orbit(l2())).expect("fixed");
    assert_eq!(fixed.epoch(), Epoch::parse(EPOCH).unwrap());
    assert_eq!(fixed.central(), CentralBody::Earth);
    let Cycle::Periodic { period_s } = fixed.cycle() else {
        panic!("periodic")
    };
    assert!((period_s / 86_400.0 - 365.25).abs() < 1.0);
    assert!(fixed.elements().is_none() && fixed.altitude_m().is_none());
    // 地球から約 150 万 km
    let r = fixed.state_at(0.0).expect("state").r_eci.norm();
    assert!((1.4e9..1.6e9).contains(&r), "{r}");
    let h = trajectory(&with_orbit(halo())).expect("halo");
    let Cycle::Periodic { period_s } = h.cycle() else {
        panic!("periodic")
    };
    assert!((170.0..190.0).contains(&(period_s / 86_400.0)));
}

/// `orbit.geometry` が走り、太陽–地球 L2 の固定点では SPE = 0°、SEP = 180°（太陽・地球が探査機から同じ側）。
#[test]
fn orbit_geometry_runs_on_the_libration_orbit() {
    let d = with_orbit(l2());
    let resolved = fune_model::resolve(&d);
    let ctx = Context::new(&d, &resolved, None).expect("context");
    let input = orbit::geometry(&ctx).expect("geometry input");
    let out = Geometry::run(&input).expect("geometry");
    let leg = &out.legs[0];
    assert_eq!(leg.central, CentralBody::Earth);
    // 最小の SEP が 180° に近い（全期間で ≥ 179°）
    assert!(leg.sep_min.value.value().to_degrees() > 179.0);
    let spe = leg
        .series
        .channels
        .iter()
        .find(|c| c.key == "spe")
        .expect("spe");
    assert!(spe.values.iter().all(|v| v.to_degrees() < 1.0));
}

/// 欄が足りなければその欄を指す不足入力（`primary`・`secondary`・`point`・`epoch`。振幅と枝は省略できる）。
#[test]
fn missing_keys_are_missing_inputs_naming_the_key() {
    for key in ["epoch", "primary", "secondary", "point"] {
        let mut o = l2();
        match key {
            "epoch" => o.epoch = None,
            "primary" => o.primary = None,
            "secondary" => o.secondary = None,
            _ => o.point = None,
        }
        let m = trajectory(&with_orbit(o)).unwrap_err();
        let paths: Vec<&str> = m.iter().map(|x| x.path.as_str()).collect();
        assert_eq!(paths, [format!("conditions.orbit.{key}")], "{key}");
    }
    let mut no_branch = halo();
    no_branch.branch = None;
    assert!(trajectory(&with_orbit(no_branch)).is_ok());
}

/// 天体の組が暦に無い・ハローが作れない（L4 に振幅）場合は、理由つきの不足入力。単位の次元が違えば欄を指す。
#[test]
fn bad_pairs_and_amplitudes_are_missing_inputs_with_a_reason() {
    let mut pair = l2();
    pair.secondary = Some(CentralBodyKind::Moon);
    let m = trajectory(&with_orbit(pair)).unwrap_err();
    assert!(m[0].reason.contains("not a primary"), "{m:?}");
    let mut l4 = halo();
    l4.point = Some(LibrationPointKind::L4);
    let m = trajectory(&with_orbit(l4)).unwrap_err();
    assert!(m[0].reason.contains("only about L1 and L2"), "{m:?}");
    let mut unit = halo();
    unit.amplitude_z = Some(Quantity::new(1.0, "kg"));
    let m = trajectory(&with_orbit(unit)).unwrap_err();
    assert_eq!(m[0].path, "conditions.orbit.amplitude_z");
}

#[test]
fn kepler_only_analyses_report_not_applicable() {
    let d = with_orbit(halo());
    let resolved = fune_model::resolve(&d);
    let ctx = Context::new(&d, &resolved, None).unwrap();
    for err in [
        orbit::eclipse_season(&ctx).unwrap_err(),
        orbit::decay(&ctx).unwrap_err(),
        orbit::sun_distance(&ctx).unwrap_err(),
    ] {
        let got: Vec<(String, String)> = err
            .into_vec()
            .into_iter()
            .map(|m| (m.path, m.reason))
            .collect();
        assert!(
            got.iter().any(|(p, r)| p == "conditions.orbit.source"
                && r.contains("Not applicable")
                && r.contains("libration")),
            "{got:?}"
        );
    }
}

/// 中心天体が地球でも高度を持たない軌道（太陽–地球 L2）の回線バジェットは、地球までの距離の時系列で距離を測る
/// （高度から決まる斜距離ではない）: 距離のモデルは `EarthToBody`、距離は L2 の 1.2〜1.7 百万 km（ハローの広がりを含む）、光の伝搬時間は距離 / c。
#[test]
fn the_link_budget_measures_the_distance_from_the_earth() {
    use sat_analysis::comm::link_budget::{LinkBudget, LinkDistanceModel};
    for o in [l2(), halo()] {
        let d = with_orbit(o);
        let resolved = fune_model::resolve(&d);
        let ctx = Context::new(&d, &resolved, None).expect("context");
        let input = fune_projection::comm::link_budget(&ctx).expect("input");
        assert!(
            input.orbit.is_some(),
            "the orbit is passed for the Earth range"
        );
        let out = LinkBudget::run(&input).expect("link budget");
        assert!(!out.links.is_empty());
        for l in &out.links {
            assert_eq!(l.distance_model, LinkDistanceModel::EarthToBody);
            let (min, max) = (l.range_min.value(), l.slant_range.value());
            assert!(
                (1.2e9..1.7e9).contains(&min) && (min..1.7e9).contains(&max),
                "{min} {max}"
            );
            let light = max / sat_astro::constants::C_LIGHT;
            assert!((l.light_time.value() - light).abs() < 1e-9 * light);
        }
    }
}
