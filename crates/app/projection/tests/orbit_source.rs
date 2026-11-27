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
        delta_v: None,
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
            departure_by: None,
            flybys: Vec::new(),
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
            epoch: None,
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

const SWING_DEPARTURE: &str = "2028-10-01T00:00:00Z";
const SWING_FLYBY: &str = "2028-12-30T00:00:00Z";
const SWING_ARRIVAL: &str = "2030-02-23T00:00:00Z";

fn venus_flyby(date: &str) -> fune_model::design::ManeuverFlyby {
    fune_model::design::ManeuverFlyby {
        body: CentralBodyKind::Venus,
        date: date.to_owned(),
        min_altitude: None,
        powered: None,
    }
}

/// `with_cruise` の遷移を、地球 → 金星（通過）→ 火星（日付は解析式の暦で成り立つ組）にする。
fn with_swing_by() -> Design {
    let mut d = with_cruise();
    let mut m = mars_transfer(SWING_DEPARTURE, SWING_ARRIVAL);
    m.transfer
        .as_mut()
        .expect("transfer")
        .flybys
        .push(venus_flyby(SWING_FLYBY));
    d.maneuvers.insert("MARS_TRANSFER".into(), m);
    d
}

/// 途中のスイングバイのある遷移は、通過の日を境にした 2 本の弧の連鎖になる（もう断らない）: 元期は出発の天体の影響圏を出る時刻、
/// 中心は太陽、境は通過の日、その日に両方の弧が金星の位置を通る（許容 1 m。連鎖の検査の許容と同じ）。繰り返しは全体で 1 回限り。
/// 通過を外すと、同じ設計で弧 1 本（従来と同じ形）になる。
#[test]
fn a_swing_by_transfer_resolves_to_a_chain_of_arcs() {
    let mut d = with_swing_by();
    let t = trajectory(&d, Some("cruise")).expect("trajectory");
    let Trajectory::Arcs(chain) = &t else {
        panic!("a chain of arcs is expected: {t:?}");
    };
    assert_eq!(chain.arcs().len(), 2);
    assert_eq!(t.central(), CentralBody::Sun);
    let (departure, flyby, arrival) = (
        epoch(SWING_DEPARTURE),
        epoch(SWING_FLYBY),
        epoch(SWING_ARRIVAL),
    );
    assert!(t.epoch().seconds_since(departure) > 0.0);
    let at = t.breakpoints_s();
    assert_eq!(at.len(), 1);
    assert!((at[0] - flyby.seconds_since(t.epoch())).abs() < 1e-6);
    let venus = CentralBody::Venus.state_about_parent(flyby).expect("venus");
    let end_of_first = &chain.arcs()[0];
    let incoming = end_of_first
        .state_at(end_of_first.span_s().expect("span"))
        .expect("state");
    assert!((incoming.r_eci - venus.r_eci).norm() < 1.0);
    assert!((t.state_at(at[0]).unwrap().r_eci - venus.r_eci).norm() < 1.0);
    // 全体は到着の天体（火星）の影響圏に入るまで（到着の日より前）
    let Cycle::OneOff { span_s } = t.cycle() else {
        panic!("a one-off chain is expected");
    };
    assert!(t.epoch().seconds_since(departure) + span_s < arrival.seconds_since(departure));
    assert!(t.elements().is_none());
    // 通過を外すと弧 1 本
    d.maneuvers
        .get_mut("MARS_TRANSFER")
        .and_then(|m| m.transfer.as_mut())
        .expect("transfer")
        .flybys
        .clear();
    let single = trajectory(&d, Some("cruise")).expect("trajectory");
    assert!(matches!(single, Trajectory::Spanned { .. }));
}

/// 通過の日が読めない・通過の天体が隣と同じなら、理由つきの不足入力（軌道の `maneuver`、または通過の欄）で、黙って 1 脚だけにしない。
#[test]
fn an_unreadable_or_invalid_flyby_is_a_missing_input() {
    let mut d = with_swing_by();
    d.maneuvers
        .get_mut("MARS_TRANSFER")
        .and_then(|m| m.transfer.as_mut())
        .expect("transfer")
        .flybys[0]
        .date = "soon".to_owned();
    let m = trajectory(&d, Some("cruise")).expect_err("must refuse");
    assert_eq!(m.len(), 1, "{m:?}");
    assert!(m[0].reason.contains("Cannot parse the date of flyby 1"));
    let mut d = with_swing_by();
    d.maneuvers
        .get_mut("MARS_TRANSFER")
        .and_then(|m| m.transfer.as_mut())
        .expect("transfer")
        .flybys[0]
        .body = CentralBodyKind::Mars;
    let m = trajectory(&d, Some("cruise")).expect_err("must refuse");
    assert_eq!(m.len(), 1, "{m:?}");
    assert_eq!(m[0].path, "phases.CRUISE.orbit.maneuver");
    assert!(m[0].reason.contains("must differ"), "{}", m[0].reason);
}

/// ケースの `epoch` が 2 本目の弧の中なら、そこから始まる（残りの弧 1 本。境の前の弧は落ちる）。境ちょうどは出ていく弧（後の弧）。
/// 最初の弧の中なら、その弧の残りと後の弧の連鎖。始めの状態は元の軌道のその時刻の状態と同じ。
#[test]
fn a_case_epoch_starts_in_the_arc_that_contains_it() {
    let mut d = with_swing_by();
    let full = trajectory(&d, Some("cruise")).expect("trajectory");
    let start_of = |d: &Design, text: &str| -> Trajectory {
        let mut d = d.clone();
        d.cases.get_mut("cruise").unwrap().epoch = Some(text.to_owned());
        let resolved = fune_model::resolve(&d);
        let ctx = Context::new(&d, &resolved, Some("cruise")).unwrap();
        let mut m = MissingInputs::new();
        let t = ctx.case_trajectory(&mut m).expect("case trajectory");
        assert!(m.is_empty());
        t
    };
    let same_state = |t: &Trajectory, at: &str| {
        let dt = epoch(at).seconds_since(full.epoch());
        let (got, want) = (t.state_at(0.0).unwrap(), full.state_at(dt).unwrap());
        assert!((got.r_eci - want.r_eci).norm() < 1e-3, "{at}");
        assert!((got.v_eci - want.v_eci).norm() < 1e-6, "{at}");
    };
    // 2 本目の弧の中（金星の通過の後）
    let in_second = start_of(&d, "2029-06-01T00:00:00Z");
    assert_eq!(in_second.epoch(), epoch("2029-06-01T00:00:00Z"));
    assert!(matches!(in_second, Trajectory::Spanned { .. }));
    same_state(&in_second, "2029-06-01T00:00:00Z");
    // 境ちょうど: 出ていく弧の全体（始めの速度は後の弧のもの）
    let at_break = start_of(&d, SWING_FLYBY);
    assert!(matches!(at_break, Trajectory::Spanned { .. }));
    assert_eq!(at_break.epoch(), epoch(SWING_FLYBY));
    same_state(&at_break, SWING_FLYBY);
    // 最初の弧の中
    let in_first = start_of(&d, "2028-11-15T00:00:00Z");
    assert_eq!(in_first.breakpoints_s().len(), 1);
    same_state(&in_first, "2028-11-15T00:00:00Z");
    // 全体の外は、理由つきの不足入力としてケースの epoch を指す
    d.cases.get_mut("cruise").unwrap().epoch = Some("2031-01-01T00:00:00Z".to_owned());
    let resolved = fune_model::resolve(&d);
    let ctx = Context::new(&d, &resolved, Some("cruise")).unwrap();
    let mut m = MissingInputs::new();
    assert!(ctx.case_trajectory(&mut m).is_none());
    assert_eq!(m.into_vec()[0].path, "cases.cruise.epoch");
}

/// 時系列（電源が使う）は連鎖の全体を 1 回限りの軌道として解く: 日照率は 1、太陽距離は境でも連続（境の前後のサンプルの差は、
/// サンプルの間隔の間の距離の変化の範囲）で、境の日の太陽距離は金星の日心距離（暦）に等しい。
#[test]
fn the_timeline_and_power_run_across_the_flyby() {
    let d = with_swing_by();
    let resolved = fune_model::resolve(&d);
    let ctx = Context::new(&d, &resolved, Some("cruise")).unwrap();
    let mut m = MissingInputs::new();
    let spec = timeline::timeline(&ctx, Some("NOMINAL"), Some(7), &mut m).expect("timeline");
    assert!(m.is_empty());
    let samples = build(&spec).expect("samples");
    assert!(samples.iter().all(|s| s.sunlit == 1.0));
    // サンプルの太陽距離は、軌道の位置の大きさそのもの（太陽中心）
    assert!(
        samples
            .iter()
            .all(|s| (s.sun_distance_m - s.r_eci.norm()).abs() < 1.0)
    );
    let t0 = spec.orbit().epoch();
    let flyby = epoch(SWING_FLYBY);
    let venus_distance = CentralBody::Venus
        .state_about_parent(flyby)
        .expect("venus")
        .r_eci
        .norm();
    let state = spec
        .orbit()
        .state_at(flyby.seconds_since(t0))
        .expect("state");
    assert!((state.r_eci.norm() - venus_distance).abs() < 1.0);
    let out = OrbitGeneration::run(&power::orbit_generation(&ctx).unwrap()).unwrap();
    assert_eq!(out.sunlit_fraction.value(), 1.0);
    assert_eq!(
        out.span_fallback,
        Some(sat_analysis::spec::SpanFallback::OneOffSpan)
    );
}

/// 設計の軌道が通過のある遷移でも、`orbit.geometry` が連鎖の全体で走る: 区間は 1 つ（中心は太陽、期間は連鎖の全体の `OneOff`）。
/// 通過の日の太陽距離は金星の日心距離（暦）で、境をまたぐサンプルの地球距離は連続（光の伝搬時間は地球距離 / c）。
#[test]
fn orbit_geometry_runs_across_the_flyby() {
    let mut d = with_swing_by();
    d.conditions.orbit = Some(transfer_orbit(Some("MARS_TRANSFER")));
    d.phases.clear();
    let resolved = fune_model::resolve(&d);
    let ctx = Context::new(&d, &resolved, None).expect("context");
    let input = orbit::geometry(&ctx).expect("geometry input");
    let leg = &input.legs[0];
    assert_eq!(leg.orbit.breakpoints_s().len(), 1);
    let out = sat_analysis::orbit::Geometry::run(&input).expect("geometry");
    let g = &out.legs[0];
    assert_eq!(g.central, CentralBody::Sun);
    assert_eq!(g.span_basis, sat_analysis::orbit::GeometrySpanBasis::OneOff);
    assert_eq!(g.span.value(), leg.orbit.cycle().length_s());
    // 太陽距離の最小は、金星の通過の日の距離（金星の日心距離）以下、最大は両端の距離以上（極値はサンプル・詰めた値で、端と境を含む）
    let (lo, hi) = (
        g.sun_distance.min.value.value(),
        g.sun_distance.max.value.value(),
    );
    let venus = CentralBody::Venus
        .state_about_parent(epoch(SWING_FLYBY))
        .expect("venus")
        .r_eci
        .norm();
    assert!(lo <= venus + 1.0, "{lo} vs {venus}");
    let span = leg.orbit.cycle().length_s();
    for t in [0.0, span] {
        let r = leg.orbit.state_at(t).expect("state").r_eci.norm();
        assert!(hi >= r - 1.0, "{hi} vs {r} at {t}");
    }
}

/// 回線バジェットの地球からの距離は連鎖の全体で求まり（最小は始めの地球の影響圏の境、最大は区間中の最遠）、
/// ケプラー要素が要る解析は連鎖でも「対象外」を理由つきの不足入力で返す。
#[test]
fn the_link_budget_and_kepler_only_analyses_handle_a_chain() {
    let d = with_swing_by();
    let resolved = fune_model::resolve(&d);
    let ctx = Context::new(&d, &resolved, Some("cruise")).unwrap();
    let input = comm::link_budget(&ctx).expect("link budget input");
    let out = LinkBudget::run(&input).expect("link budget");
    let link = &out.links[0];
    let soi = CentralBody::Earth.soi_radius_m().expect("soi");
    assert!((link.range_min.value() - soi).abs() < 100.0);
    assert!(link.slant_range.value() > link.range_min.value());
    for m in [
        orbit::eclipse_season(&ctx).unwrap_err(),
        orbit::decay(&ctx).unwrap_err(),
        power::sizing(&ctx).unwrap_err(),
    ] {
        let got: Vec<(String, String)> = m
            .into_vec()
            .into_iter()
            .map(|m| (m.path, m.reason))
            .collect();
        assert!(
            got.iter().any(|(p, r)| p == "phases.CRUISE.orbit.source"
                && r.contains("Not applicable")
                && r.contains("transfer")),
            "{got:?}"
        );
    }
}
