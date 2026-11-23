//! 軌道の出どころ `transfer`（日付つきの遷移の巡航の軌道）: 地球 → 火星の遷移のマヌーバから、元期が出発の日・中心が太陽・期間が
//! 飛行時間の軌道ができ、両端の天体の位置を通ること。時系列・回線の距離がその軌道で動き、ケプラー要素が要る解析は
//! 「対象外」を理由つきの不足入力として返すこと。設計はコードで書き換える（デモのデータは変えない）。

use fune_model::Design;
use fune_model::design::{
    Case, CentralBodyKind, Maneuver, ManeuverCategory, ManeuverConstraints, ManeuverMethod,
    ManeuverTarget, ManeuverTransfer, OrbitSettings, OrbitSource, Phase,
};
use fune_model::value::Quantity;
use fune_projection::{
    Context, MissingInput, MissingInputs, comm, orbit, power, propulsion, timeline,
};
use sat_analysis::comm::LinkBudget;
use sat_analysis::power::OrbitGeneration;
use sat_analysis::spec::Analysis;
use sat_astro::ephem::{Body, CentralBody, position_eci_m};
use sat_astro::orbit::{Cycle, Trajectory};
use sat_astro::time::Epoch;
use sat_astro::timeline::build;
use std::path::Path;

const DEPARTURE: &str = "2028-12-10T00:00:00Z";
const ARRIVAL: &str = "2029-09-25T00:00:00Z";

fn demo() -> Design {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    fune_store::load(&root)
        .expect("the demo should load")
        .design
}

fn mars_transfer(departure: &str, arrival: &str) -> Maneuver {
    Maneuver {
        id: "MARS_TRANSFER".into(),
        label: None,
        category: ManeuverCategory::Insertion,
        method: ManeuverMethod::Impulsive,
        thruster: None,
        after: None,
        via_altitude: None,
        target: ManeuverTarget {
            body: Some(CentralBodyKind::Mars),
            perigee_altitude: Some(Quantity::new(300.0, "km")),
            apogee_altitude: Some(Quantity::new(33_000.0, "km")),
            ..ManeuverTarget::default()
        },
        constraints: ManeuverConstraints::default(),
        transfer: Some(ManeuverTransfer {
            departure: Some(departure.to_owned()),
            arrival: Some(arrival.to_owned()),
        }),
    }
}

fn transfer_orbit(maneuver: Option<&str>) -> OrbitSettings {
    OrbitSettings {
        source: Some(OrbitSource::Transfer),
        maneuver: maneuver.map(str::to_owned),
        ..OrbitSettings::default()
    }
}

/// デモに、地球 → 火星の遷移のマヌーバ・巡航のフェーズ（`source = "transfer"`）・それを選ぶケースを足す。
/// 熱の条件の β は外す（日付から決まる軌道には RAAN の置き換えが当てられない）。
fn with_cruise() -> Design {
    let mut d = demo();
    d.maneuvers
        .insert("MARS_TRANSFER".into(), mars_transfer(DEPARTURE, ARRIVAL));
    d.phases.insert(
        "CRUISE".into(),
        Phase {
            id: "CRUISE".into(),
            orbit: Some(transfer_orbit(Some("MARS_TRANSFER"))),
            ..Phase::default()
        },
    );
    for c in d
        .analysis
        .thermal
        .as_mut()
        .expect("thermal")
        .cases
        .values_mut()
    {
        c.beta = None;
    }
    for c in d.cases.values_mut() {
        c.beta = None;
    }
    d.cases.insert(
        "cruise".into(),
        Case {
            id: "cruise".into(),
            label: None,
            description: None,
            mode: Some("NOMINAL".into()),
            phase: Some("CRUISE".into()),
            side: None,
            thermal_case: Some("cold_nominal".into()),
            beta: None,
            orbits: None,
            sun_distance: None,
        },
    );
    d
}

fn epoch(text: &str) -> Epoch {
    Epoch::parse(text).expect("epoch")
}

fn trajectory(d: &Design, case: Option<&str>) -> Result<Trajectory, Vec<MissingInput>> {
    let resolved = fune_model::resolve(d);
    let ctx = Context::new(d, &resolved, case).expect("context");
    let mut m = MissingInputs::new();
    let t = ctx.trajectory(&mut m);
    t.ok_or_else(|| m.into_vec())
}

/// 巡航の軌道は影響圏の外の区間: 元期は出発の天体（地球）の影響圏を出る時刻（出発の日より後）、中心は太陽、期間は到着の天体
/// （火星）の影響圏に入るまで（`OneOff`）。同じ楕円を元期の前後へ延ばすと、出発の日の地球の位置と到着の日の火星の位置を通る
/// （二体の軌道。位置は暦の天体の位置と比べる）。
///
/// 許容は 1 m: Lambert の二分法の幅 `1e-13` による飛行時間の誤差は 0.1 m 未満の位置の誤差になり（速さ 30 km/s）、
/// 接触要素への往復と Kepler 方程式の反復の丸めはそれより小さい（`sat_astro::transfer` のテストと同じ根拠）。
#[test]
fn the_cruise_orbit_runs_between_the_spheres_of_influence_on_the_arc_through_both_bodies() {
    let d = with_cruise();
    let t = trajectory(&d, Some("cruise")).expect("trajectory");
    let (departure, arrival) = (epoch(DEPARTURE), epoch(ARRIVAL));
    let tof = arrival.seconds_since(departure);
    let leave = t.epoch().seconds_since(departure);
    let Cycle::OneOff { span_s } = t.cycle() else {
        panic!("a one-off arc is expected");
    };
    assert_eq!(t.central(), CentralBody::Sun);
    assert!(leave > 0.0 && leave + span_s < tof);
    let earth = CentralBody::Earth
        .state_about_parent(departure)
        .expect("earth");
    let mars = CentralBody::Mars.state_about_parent(arrival).expect("mars");
    assert!((t.state_at(-leave).unwrap().r_eci - earth.r_eci).norm() < 1.0);
    assert!((t.state_at(tof - leave).unwrap().r_eci - mars.r_eci).norm() < 1.0);
}

/// 時系列（電源・熱が使う）は、周回数を使わず巡航の区間（影響圏の外）そのものを解き、太陽距離は軌道の近日点と遠日点のあいだで、日陰は無い。
/// 両端は影響圏の境。
#[test]
fn the_timeline_covers_the_cruise_leg_with_the_sun_always_visible() {
    let d = with_cruise();
    let resolved = fune_model::resolve(&d);
    let ctx = Context::new(&d, &resolved, Some("cruise")).unwrap();
    let mut m = MissingInputs::new();
    let spec = timeline::timeline(&ctx, Some("NOMINAL"), Some(7), &mut m).expect("timeline");
    assert!(m.is_empty());
    let Cycle::OneOff { span_s } = spec.orbit().cycle() else {
        panic!("a one-off arc is expected");
    };
    assert_eq!(spec.sampling.span.value(), span_s);
    let samples = build(&spec).expect("samples");
    assert!(samples.iter().all(|s| s.sunlit == 1.0));
    // 軌道の近日点・遠日点（二体の楕円なので、距離はそのあいだ）
    let Trajectory::Spanned { orbit, .. } = spec.orbit() else {
        panic!("a one-off orbit is expected");
    };
    let Trajectory::Elements(o) = orbit.as_ref() else {
        panic!("the arc is a two-body orbit");
    };
    let (a, e) = (o.elements.semi_major_axis.value(), o.elements.eccentricity);
    for s in &samples {
        let r = s.sun_distance_m;
        assert!(r >= a * (1.0 - e) * (1.0 - 1e-9) && r <= a * (1.0 + e) * (1.0 + 1e-9));
        assert!((r - s.r_eci.norm()).abs() < 1.0);
    }
    // 両端は影響圏の境: 探査機と天体の距離が影響圏の半径に等しい（許容 100 m。`sat_astro::transfer` のテストと同じ根拠）
    let (first, last) = (
        samples.first().expect("first"),
        samples.last().expect("last"),
    );
    let from_planet = |body: CentralBody, s: &sat_astro::timeline::TimelineSample| {
        (s.r_eci - body.state_about_parent(s.epoch).expect("state").r_eci).norm()
    };
    let soi = |body: CentralBody| body.soi_radius_m().expect("soi");
    assert!((from_planet(CentralBody::Earth, first) - soi(CentralBody::Earth)).abs() < 100.0);
    assert!((from_planet(CentralBody::Mars, last) - soi(CentralBody::Mars)).abs() < 100.0);
}

/// 発電の解析は 1 回限りの軌道で走り、出力に周回が無いことの注意が付く。日照率は 1。
#[test]
fn power_generation_runs_over_the_arc_and_says_it_is_one_off() {
    let d = with_cruise();
    let resolved = fune_model::resolve(&d);
    let ctx = Context::new(&d, &resolved, Some("cruise")).unwrap();
    let out = OrbitGeneration::run(&power::orbit_generation(&ctx).unwrap()).unwrap();
    assert_eq!(out.sunlit_fraction.value(), 1.0);
    assert!(out.eclipses.is_empty());
    let spec = timeline::timeline(&ctx, Some("NOMINAL"), None, &mut MissingInputs::new()).unwrap();
    assert_eq!(out.period.value(), spec.sampling.span.value());
    assert_eq!(out.complete_orbits, 1);
    assert_eq!(
        out.span_fallback,
        Some(sat_analysis::spec::SpanFallback::OneOffSpan)
    );
}

/// 地球からの距離は、区間の始め（t = 0）で地球の影響圏の半径に等しく（期待値は `soi_radius_m` から求める）、その後は増える。
/// 回線バジェットの距離（`earth_range`）の最小はその始めの距離で、最大は区間中の最遠。許容は 100 m（区間の境の時刻を
/// 二分法で 1 ms まで詰めるため。`sat_astro::transfer` のテストと同じ根拠）。
#[test]
fn the_earth_distance_starts_at_the_sphere_of_influence_and_grows() {
    let d = with_cruise();
    let t = trajectory(&d, Some("cruise")).expect("trajectory");
    let soi = CentralBody::Earth.soi_radius_m().expect("soi");
    let distance = |s: f64| {
        let state = t.state_at(s).expect("state");
        (position_eci_m(Body::Sun, t.epoch().plus_seconds(s)) + state.r_eci).norm()
    };
    let day = 86_400.0;
    assert!((distance(0.0) - soi).abs() < 100.0, "{}", distance(0.0));
    assert!(distance(day) > distance(0.0));
    assert!(distance(10.0 * day) > distance(day));
    assert!(distance(100.0 * day) > distance(10.0 * day));

    let resolved = fune_model::resolve(&d);
    let ctx = Context::new(&d, &resolved, Some("cruise")).unwrap();
    let input = comm::link_budget(&ctx).expect("link budget input");
    let far = input
        .orbit
        .as_ref()
        .expect("the Earth range needs the orbit");
    assert_eq!(far.cycle(), t.cycle());
    let out = LinkBudget::run(&input).expect("link budget");
    let link = &out.links[0];
    assert!(
        (link.range_min.value() - soi).abs() < 100.0,
        "{}",
        link.range_min.value()
    );
    assert!(link.slant_range.value() > 1.0e11);
    // 回線の余裕と PFD は有限（距離 0 の特異点が無い）
    for l in &out.links {
        assert!(l.range_min.value() > 1.0e8);
    }
    assert!(
        out.checks
            .iter()
            .all(|c| !c.message.contains("NaN") && !c.message.contains("inf"))
    );
}

/// ケプラー要素が要る解析は、対象外を軌道の `source` を対象にした理由つきの不足入力で返す（ほかの不足は混ぜない）。
#[test]
fn kepler_only_analyses_report_not_applicable_naming_the_source() {
    let d = with_cruise();
    let resolved = fune_model::resolve(&d);
    let ctx = Context::new(&d, &resolved, Some("cruise")).unwrap();
    let paths = |e: MissingInputs| -> Vec<(String, String)> {
        e.into_vec()
            .into_iter()
            .map(|m| (m.path, m.reason))
            .collect()
    };
    let not_applicable = |got: Vec<(String, String)>| {
        assert!(
            got.iter().any(|(p, r)| p == "phases.CRUISE.orbit.source"
                && r.contains("Not applicable")
                && r.contains("transfer")),
            "{got:?}"
        );
    };
    not_applicable(paths(orbit::eclipse_season(&ctx).unwrap_err()));
    not_applicable(paths(orbit::decay(&ctx).unwrap_err()));
    not_applicable(paths(orbit::sun_distance(&ctx).unwrap_err()));
    not_applicable(paths(propulsion::maneuvers(&ctx).unwrap_err()));
    not_applicable(paths(power::sizing(&ctx).unwrap_err()));
    // 直し方の候補は付かない（候補を決める service が、この対象に何も返さない。`fixes.rs` のテスト）
    let none = Context::new(&d, &resolved, None).unwrap();
    assert!(orbit::eclipse_season(&none).is_ok());
}

/// 遷移の弧が楕円でないとき（飛行時間が短い）は、理由つきの不足入力（軌道の `maneuver`）にする。
#[test]
fn a_hyperbolic_arc_is_reported_with_its_reason() {
    let mut d = with_cruise();
    d.maneuvers.insert(
        "MARS_TRANSFER".into(),
        mars_transfer(DEPARTURE, "2029-01-20T00:00:00Z"),
    );
    let m = trajectory(&d, Some("cruise")).unwrap_err();
    assert_eq!(m.len(), 1, "{m:?}");
    assert_eq!(m[0].path, "phases.CRUISE.orbit.maneuver");
    assert!(m[0].reason.contains("not an ellipse"), "{}", m[0].reason);
}

/// `maneuver` が無い・存在しない・日付が無いときの不足入力（軌道の `maneuver` を指す）。
#[test]
fn a_missing_or_unusable_maneuver_is_a_missing_input() {
    let cases = [
        (None, "Maneuver with a dated"),
        (Some("NOPE"), "not found"),
        (Some("PLAIN"), "no dated [transfer]"),
    ];
    for (maneuver, reason) in cases {
        let mut d = with_cruise();
        let mut plain = mars_transfer(DEPARTURE, ARRIVAL);
        plain.id = "PLAIN".into();
        plain.transfer = None;
        d.maneuvers.insert("PLAIN".into(), plain);
        d.phases.get_mut("CRUISE").unwrap().orbit = Some(transfer_orbit(maneuver));
        let m = trajectory(&d, Some("cruise")).unwrap_err();
        assert_eq!(m.len(), 1, "{m:?}");
        assert_eq!(m[0].path, "phases.CRUISE.orbit.maneuver");
        assert!(m[0].reason.contains(reason), "{}", m[0].reason);
    }
}

/// ケースの `sun_distance`・`beta` は要素の軌道の置き換えで、巡航の軌道には当てられない（ケースのキーを指す不足入力）。
#[test]
fn case_overrides_do_not_apply_to_a_transfer_orbit() {
    let mut d = with_cruise();
    let case = d.cases.get_mut("cruise").unwrap();
    case.sun_distance = Some(Quantity::new(1.2, "AU"));
    case.beta = Some(Quantity::new(30.0, "deg"));
    let m = trajectory(&d, Some("cruise")).unwrap_err();
    let paths: Vec<&str> = m.iter().map(|x| x.path.as_str()).collect();
    assert_eq!(paths, ["cases.cruise.beta", "cases.cruise.sun_distance"]);
    assert!(m.iter().all(|x| x.reason.contains("Not applicable")));
    // 要素の軌道（ケースが巡航のフェーズを指さない）では今までどおり
    let mut elements = demo();
    elements.cases.get_mut("hot_imaging").unwrap().beta = Some(Quantity::new(30.0, "deg"));
    assert!(trajectory(&elements, Some("hot_imaging")).is_ok());
}

/// 設計の軌道が `transfer` でも同じ（フェーズが無くても）。マヌーバの列の始めの軌道の中心天体は、巡航の軌道が何も語らないので既定（地球）のまま
/// （循環させない）。
#[test]
fn the_design_orbit_can_be_a_transfer_too() {
    let mut d = demo();
    d.maneuvers
        .insert("MARS_TRANSFER".into(), mars_transfer(DEPARTURE, ARRIVAL));
    assert_eq!(
        fune_projection::maneuver_dates::central_before(&d, &d.maneuvers["MARS_TRANSFER"]),
        CentralBody::Earth
    );
    d.conditions.orbit = Some(transfer_orbit(Some("MARS_TRANSFER")));
    let t = trajectory(&d, None).expect("trajectory");
    assert_eq!(t.central(), CentralBody::Sun);
    assert_eq!(
        fune_projection::maneuver_dates::central_before(&d, &d.maneuvers["MARS_TRANSFER"]),
        CentralBody::Earth
    );
}

/// 熱の `time_step`（デモは 30 s）は周回の分解能の指定で、1 回限りの軌道（巡航）ではその期間の既定の刻み（`期間 / 200`）より細かくしない。
/// 周期的な軌道（巡航のフェーズを指さないケース）では書いたとおり。
#[test]
fn the_thermal_time_step_is_not_finer_than_the_one_off_default() {
    let d = with_cruise();
    let resolved = fune_model::resolve(&d);
    let ctx = Context::new(&d, &resolved, Some("cruise")).unwrap();
    let input = fune_projection::thermal::transient(&ctx).expect("thermal input");
    let span = input.timeline.sampling.span.value();
    assert_eq!(
        input.timeline.orbit().cycle(),
        Cycle::OneOff { span_s: span }
    );
    assert_eq!(input.timeline.sampling.step.value(), span / 200.0);
    let ctx = Context::new(&d, &resolved, Some("cold_safe")).unwrap();
    let input = fune_projection::thermal::transient(&ctx).expect("thermal input");
    assert_eq!(input.timeline.sampling.step.value(), 30.0);
}
