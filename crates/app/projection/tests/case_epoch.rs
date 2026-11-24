//! 解析ケースの `epoch`: 時系列の解析は、ケースの軌道のその時刻から始まる。ケースが無い・`epoch` が無いときは今までと同じ。
//! 距離の最悪値を取る回線バジェットは期間全体を見るので、ケースの `epoch` で変わらない。設計はコードで書き換える（デモのデータは変えない）。

use fune_model::Design;
use fune_model::design::{Case, CentralBodyKind, OrbitSettings, OrbitSource};
use fune_model::value::{Quantity, Vector3};
use fune_projection::{Context, MissingInputs, comm, orbit, power};
use sat_analysis::comm::LinkBudget;
use sat_analysis::orbit::Geometry;
use sat_analysis::power::OrbitGeneration;
use sat_analysis::spec::Analysis;
use sat_astro::constants::{AU, R_SUN, SECONDS_PER_DAY};
use sat_astro::ephem::CentralBody;
use sat_astro::orbit::Cycle;
use sat_astro::time::Epoch;
use std::path::Path;

const EPOCH: &str = "2030-01-01T00:00:00Z";

fn demo() -> Design {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    fune_store::load(&root)
        .expect("the demo should load")
        .design
}

fn case(id: &str, epoch: Option<&str>) -> Case {
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
        sun_distance: None,
        epoch: epoch.map(str::to_owned),
    }
}

/// 太陽を回る楕円（近日点 0.7 AU・遠日点 1.5 AU。元期の平均近点角は 0 = 近日点）。ケースの β と熱条件の太陽光・アルベド・赤外は外す。
fn heliocentric(cases: &[(&str, Option<&str>)]) -> Design {
    let mut d = demo();
    let o = d.conditions.orbit.as_mut().expect("orbit");
    o.central = Some(CentralBodyKind::Sun);
    o.altitude = Some(Quantity::new(1.1 * AU - R_SUN, "m"));
    o.eccentricity = Some(0.8 / 2.2);
    o.sun_synchronous = None;
    o.ltdn = None;
    o.inclination = Some(Quantity::new(1.0, "deg"));
    o.raan = Some(Quantity::new(0.0, "deg"));
    o.epoch = Some(EPOCH.to_owned());
    for c in d
        .analysis
        .thermal
        .as_mut()
        .expect("thermal")
        .cases
        .values_mut()
    {
        c.solar_flux = None;
        c.albedo = None;
        c.planet_ir = None;
        c.beta = None;
    }
    for c in d.cases.values_mut() {
        c.beta = None;
    }
    for (id, epoch) in cases {
        d.cases.insert((*id).into(), case(id, *epoch));
    }
    d
}

fn vector(x: f64, y: f64, z: f64, unit: &str) -> Vector3 {
    Vector3 {
        x,
        y,
        z,
        unit: Some(unit.to_owned()),
        frame: None,
    }
}

/// 太陽中心・1 au の円（円速度）の状態の軌道（期間 300 日）。
fn state_orbit() -> OrbitSettings {
    let v_kms = (CentralBody::Sun.gm() / AU).sqrt() / 1e3;
    OrbitSettings {
        source: Some(OrbitSource::State),
        epoch: Some(EPOCH.to_owned()),
        central: Some(CentralBodyKind::Sun),
        position: Some(vector(AU / 1e3, 0.0, 0.0, "km")),
        velocity: Some(vector(0.0, v_kms, 0.0, "km/s")),
        span: Some(Quantity::new(300.0, "d")),
        ..OrbitSettings::default()
    }
}

/// ケース `id` で発電の解析を回したときの時系列の最初の標本の発電 [W] と、時系列の軌道の元期。
fn first_generation(d: &Design, id: Option<&str>) -> (f64, Epoch) {
    let resolved = fune_model::resolve(d);
    let ctx = Context::new(d, &resolved, id).expect("context");
    let input = power::orbit_generation(&ctx).expect("input");
    let start = input.timeline.orbit().epoch();
    let out = OrbitGeneration::run(&input).expect("run");
    let g = out
        .series
        .channels
        .iter()
        .find(|c| c.key == "generation")
        .expect("generation channel");
    (g.values[0], start)
}

/// 近日点から始まる軌道で、ケースの `epoch` を遠日点（半周後）にすると、時系列は遠日点から始まり、発電は太陽光の強さに比例する:
/// 最初の標本の発電の比は `(0.7 AU / 1.5 AU)²`（手計算。軌道は Kepler の二体で `r = a(1 ∓ e)`）。`epoch` の無いケースは軌道の元期から始まる。
#[test]
fn a_case_epoch_at_aphelion_gives_the_aphelion_flux() {
    let d0 = heliocentric(&[("plain", None)]);
    let resolved = fune_model::resolve(&d0);
    let mut m = MissingInputs::new();
    let orbit = Context::new(&d0, &resolved, None)
        .unwrap()
        .orbit(&mut m)
        .expect("orbit");
    let half = Epoch::parse(EPOCH)
        .unwrap()
        .plus_seconds(orbit.period_s() / 2.0);
    // 秒の桁までで書く（遠日点の近くの距離は時刻の 2 次でしか変わらない）
    let d = heliocentric(&[("plain", None), ("aphelion", Some(&half.to_iso()))]);
    let (g_peri, start_plain) = first_generation(&d, Some("plain"));
    let (g_aph, start_aph) = first_generation(&d, Some("aphelion"));
    assert_eq!(start_plain, Epoch::parse(EPOCH).unwrap());
    assert!(start_aph.seconds_since(half).abs() < 1.0);
    let expected = (0.7f64 / 1.5).powi(2);
    assert!(
        (g_aph / g_peri / expected - 1.0).abs() < 1e-6,
        "{}",
        g_aph / g_peri
    );
}

/// 1 回限りの軌道（状態ベクトル）: `epoch` は期間の中なら、その時刻から始まり期間は残りの長さ。期間の外・終わりは不足入力として
/// ケースの `epoch` を指し、書けない形は読めない旨を返す。
#[test]
fn a_case_epoch_must_lie_inside_a_one_off_span() {
    let mut d = heliocentric(&[]);
    d.conditions.orbit = Some(state_orbit());
    let at = |days: f64| {
        Epoch::parse(EPOCH)
            .unwrap()
            .plus_seconds(days * SECONDS_PER_DAY)
    };
    for (id, epoch) in [
        ("inside", at(100.0).to_iso()),
        ("before", at(-1.0).to_iso()),
        ("after", at(301.0).to_iso()),
        ("end", at(300.0).to_iso()),
        ("garbled", "last Tuesday".to_owned()),
    ] {
        d.cases.insert(id.into(), case(id, Some(&epoch)));
    }
    let resolved = fune_model::resolve(&d);
    let trajectory = |id: &str| {
        let ctx = Context::new(&d, &resolved, Some(id)).unwrap();
        let mut m = MissingInputs::new();
        ctx.case_trajectory(&mut m).ok_or_else(|| m.into_vec())
    };
    let t = trajectory("inside").expect("inside");
    assert_eq!(t.epoch(), at(100.0));
    assert_eq!(
        t.cycle(),
        Cycle::OneOff {
            span_s: 200.0 * SECONDS_PER_DAY
        }
    );
    // 1 au の円: 始めの距離も 1 au の近く
    let r = t.state_at(0.0).unwrap().r_eci.norm();
    assert!((r / AU - 1.0).abs() < 1e-2, "{}", r / AU);
    for (id, text) in [
        ("before", "outside the span"),
        ("after", "outside the span"),
        ("end", "end of the span"),
        ("garbled", "Cannot parse"),
    ] {
        let missing = trajectory(id).expect_err(id);
        assert_eq!(missing[0].path, format!("cases.{id}.epoch"), "{id}");
        assert!(
            missing[0].reason.contains(text),
            "{id}: {}",
            missing[0].reason
        );
    }
}

/// 状態ベクトルの軌道の回線バジェット: 距離は軌道の期間全体での地球からの最大距離（`earth_to_body`）で、ケースの `epoch` で変わらない。
/// 値は `orbit.geometry` の区間の最大（同じ幾何で詰めたもの）に、回線バジェットの標本の刻みの分（期間 420 日 / 200 = 2.1 日の半分だけ
/// ずれた標本の距離の相対誤差は ½(ω·1.05 日)² ≈ 1.6e-4、ω は地球の公転の角速度 2e-7 rad/s）の余裕を見て 1e-3 で一致する。
#[test]
fn the_link_range_of_a_state_orbit_is_the_farthest_over_the_span_whatever_the_case_epoch() {
    let mut d = heliocentric(&[]);
    let mut o = state_orbit();
    o.span = Some(Quantity::new(420.0, "d"));
    d.conditions.orbit = Some(o);
    let at = |days: f64| {
        Epoch::parse(EPOCH)
            .unwrap()
            .plus_seconds(days * SECONDS_PER_DAY)
    };
    d.cases
        .insert("early".into(), case("early", Some(&at(10.0).to_iso())));
    d.cases
        .insert("late".into(), case("late", Some(&at(300.0).to_iso())));
    let resolved = fune_model::resolve(&d);
    let range_of = |id: Option<&str>| {
        let ctx = Context::new(&d, &resolved, id).unwrap();
        let input = comm::link_budget(&ctx).expect("link budget input");
        let out = LinkBudget::run(&input).expect("link budget");
        let l = &out.links[0];
        (l.slant_range.value(), l.range_max_epoch, l.distance_model)
    };
    let (far, far_epoch, model) = range_of(None);
    assert_eq!(
        model,
        sat_analysis::comm::link_budget::LinkDistanceModel::EarthToBody
    );
    for id in ["early", "late"] {
        let (r, e, _) = range_of(Some(id));
        assert_eq!((r, e), (far, far_epoch), "{id}");
    }
    // `orbit.geometry` の同じ区間の地球距離の最大
    let ctx = Context::new(&d, &resolved, None).unwrap();
    let geometry = Geometry::run(&orbit::geometry(&ctx).unwrap()).unwrap();
    let leg = geometry
        .legs
        .iter()
        .find(|l| l.phase.as_deref() == Some("NOMINAL_OPS"))
        .unwrap_or(&geometry.legs[0]);
    let max = leg.earth_distance.max.value.value();
    assert!((far / max - 1.0).abs() < 1e-3, "{far} vs {max}");
}
