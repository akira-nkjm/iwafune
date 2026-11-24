//! 軌道の出どころ `state`（状態ベクトル）: 書いた元期・原点・位置・速度（単位どおりに SI へ）・期間から、1 回限りの軌道ができること。
//! `orbit.geometry` がその軌道で走り、ケプラー要素が要る解析は「対象外」を理由つきの不足入力で返し、足りない欄・次元の違う単位は
//! 欄を指す不足入力になること。設計はコードで書き換える（デモのデータは変えない）。

use fune_model::Design;
use fune_model::design::{CentralBodyKind, OrbitSettings, OrbitSource, Phase};
use fune_model::value::{Quantity, Vector3};
use fune_projection::{Context, MissingInput, MissingInputs, orbit};
use sat_analysis::orbit::Geometry;
use sat_analysis::spec::Analysis;
use sat_astro::constants::{AU, C_LIGHT};
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

fn vector(x: f64, y: f64, z: f64, unit: &str) -> Vector3 {
    Vector3 {
        x,
        y,
        z,
        unit: Some(unit.to_owned()),
        frame: None,
    }
}

/// 太陽中心・1 au・円速度（太陽の点質量の GM から）の状態（位置は km、速度は km/s で書く）。
fn circular_state() -> OrbitSettings {
    let v_kms = (CentralBody::Sun.gm() / AU).sqrt() / 1e3;
    OrbitSettings {
        source: Some(OrbitSource::State),
        epoch: Some(EPOCH.to_owned()),
        central: Some(CentralBodyKind::Sun),
        position: Some(vector(AU / 1e3, 0.0, 0.0, "km")),
        velocity: Some(vector(0.0, v_kms, 0.0, "km/s")),
        span: Some(Quantity::new(365.0, "d")),
        ..OrbitSettings::default()
    }
}

/// デモの設計の軌道を状態にして、フェーズを外す（`orbit.geometry` は設計の軌道 1 つの区間になる）。
fn with_state(o: OrbitSettings) -> Design {
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

/// 書いた値が単位どおりに SI の状態になる: 元期・中心天体・期間（日 → 秒、1 回限り）・位置（km → m）・速度（km/s → m/s）。
/// 時刻 0 の状態は書いた状態そのもの。
#[test]
fn the_state_orbit_is_a_one_off_trajectory_in_si() {
    let t = trajectory(&with_state(circular_state())).expect("trajectory");
    assert_eq!(t.epoch(), Epoch::parse(EPOCH).unwrap());
    assert_eq!(t.central(), CentralBody::Sun);
    assert_eq!(
        t.cycle(),
        Cycle::OneOff {
            span_s: 365.0 * 86_400.0
        }
    );
    assert!(t.elements().is_none() && t.altitude_m().is_none());
    let s = t.state_at(0.0).expect("state");
    let v = (CentralBody::Sun.gm() / AU).sqrt();
    assert!((s.r_eci.x - AU).abs() < 1e-3 && s.r_eci.y == 0.0 && s.r_eci.z == 0.0);
    assert!((s.v_eci.y - v).abs() < 1e-9 * v && s.v_eci.x == 0.0);
}

/// 状態は多体の力（太陽と惑星の点質量の既定）で伝搬する: 1 au の円軌道は 1 年のあいだ 1 au の近くにある。
/// 許容は 1e-2 au: 惑星の直接の加速度の和の上限は、木星（太陽との距離 ≥ 4.2 au）で `GM_J/d² ≈ 7e-8`、ほかの惑星を足しても 3.5e-7 m/s² 以下で、
/// 1 年の変位は `½ a t² ≈ 1.7e8 m ≈ 1.2e-3 au` 以下（太陽の加速度の差になる潮汐の項はこれより小さい）。
#[test]
fn a_heliocentric_state_stays_near_its_radius_under_the_default_forces() {
    let t = trajectory(&with_state(circular_state())).expect("trajectory");
    for day in [0.0, 90.0, 180.0, 270.0, 365.0] {
        let r = t.state_at(day * 86_400.0).expect("state").r_eci.norm();
        assert!((r / AU - 1.0).abs() < 1e-2, "day {day}: {} au", r / AU);
    }
}

/// `orbit.geometry` が状態の軌道で走る: 区間は 1 つ（中心は太陽、期間は書いた期間の `OneOff`）。太陽距離の範囲は 1 au の近く
/// （上のテストと同じ許容）、片道の光の伝搬時間の最大は、地球距離の最大 / c（同じ評価を別の量で詰めたもの）に 1e-6 の相対誤差で一致する。
#[test]
fn orbit_geometry_runs_on_a_state_orbit() {
    let d = with_state(circular_state());
    let resolved = fune_model::resolve(&d);
    let ctx = Context::new(&d, &resolved, None).expect("context");
    let input = orbit::geometry(&ctx).expect("geometry input");
    let out = Geometry::run(&input).expect("geometry");
    assert_eq!(out.legs.len(), 1);
    let leg = &out.legs[0];
    assert_eq!(leg.central, CentralBody::Sun);
    assert_eq!(leg.span.value(), 365.0 * 86_400.0);
    assert_eq!(
        leg.span_basis,
        sat_analysis::orbit::GeometrySpanBasis::OneOff
    );
    for r in [leg.sun_distance.min.value, leg.sun_distance.max.value] {
        assert!((r.value() / AU - 1.0).abs() < 1e-2, "{} au", r.value() / AU);
    }
    let (light, far) = (
        leg.light_time_max.value.value(),
        leg.earth_distance.max.value.value(),
    );
    assert!(
        (light - far / C_LIGHT).abs() < 1e-6 * light,
        "{light} s vs {far} m"
    );
}

/// 欄が足りなければ、その欄を指す不足入力（直す先は `conditions.orbit.<欄>`。フェーズの軌道なら `phases.<ID>.orbit.<欄>`）。
#[test]
fn missing_keys_are_missing_inputs_naming_the_key() {
    for key in ["epoch", "position", "velocity", "span"] {
        let mut o = circular_state();
        match key {
            "epoch" => o.epoch = None,
            "position" => o.position = None,
            "velocity" => o.velocity = None,
            _ => o.span = None,
        }
        let m = trajectory(&with_state(o.clone())).unwrap_err();
        let paths: Vec<&str> = m.iter().map(|x| x.path.as_str()).collect();
        assert_eq!(paths, [format!("conditions.orbit.{key}")], "{key}");
        // フェーズの軌道
        let mut d = demo();
        d.phases.insert(
            "CRUISE".into(),
            Phase {
                id: "CRUISE".into(),
                orbit: Some(o),
                ..Phase::default()
            },
        );
        let resolved = fune_model::resolve(&d);
        let ctx = Context::new(&d, &resolved, None).unwrap();
        let in_phase = ctx.in_phase(&d.phases["CRUISE"]);
        let mut missing = MissingInputs::new();
        assert!(in_phase.trajectory(&mut missing).is_none());
        let paths: Vec<String> = missing.into_vec().into_iter().map(|x| x.path).collect();
        assert_eq!(paths, [format!("phases.CRUISE.orbit.{key}")], "{key}");
    }
}

/// 単位の次元が違えば、欄を指す不足入力（理由に次元の違いが入る）。値の検査（`check`）の誤りと同じ表。
#[test]
fn a_unit_of_the_wrong_dimension_is_a_missing_input_for_that_key() {
    let mut o = circular_state();
    o.velocity = Some(vector(0.0, 29.78, 0.0, "km"));
    let m = trajectory(&with_state(o)).unwrap_err();
    assert_eq!(m.len(), 1, "{m:?}");
    assert_eq!(m[0].path, "conditions.orbit.velocity");
}

/// ケプラー要素が要る解析は、状態の軌道では対象外を軌道の `source` を対象にした理由つきの不足入力で返す。
#[test]
fn kepler_only_analyses_report_not_applicable_for_a_state_orbit() {
    let d = with_state(circular_state());
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
                && r.contains("state")),
            "{got:?}"
        );
    }
}

/// ケースの `sun_distance`・`beta` は要素の軌道の置き換えで、状態の軌道には当てられない。
#[test]
fn case_overrides_do_not_apply_to_a_state_orbit() {
    let mut d = with_state(circular_state());
    d.cases.get_mut("hot_imaging").unwrap().beta = Some(Quantity::new(30.0, "deg"));
    let m = trajectory_for_case(&d, "hot_imaging").unwrap_err();
    assert!(
        m.iter()
            .any(|x| x.path == "cases.hot_imaging.beta" && x.reason.contains("Not applicable")),
        "{m:?}"
    );
}

fn trajectory_for_case(d: &Design, case: &str) -> Result<Trajectory, Vec<MissingInput>> {
    let resolved = fune_model::resolve(d);
    let ctx = Context::new(d, &resolved, Some(case)).expect("context");
    let mut m = MissingInputs::new();
    ctx.trajectory(&mut m).ok_or_else(|| m.into_vec())
}
