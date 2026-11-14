use super::simulate::{self, Model};
use super::*;
use crate::propulsion::{
    DvCategory, ManeuverInput, OrbitTarget, PropellantTank, PropulsionThruster, ThrustConstraints,
    TransferMethod,
};
use crate::spec::Verdict;
use sat_astro::atmosphere::{AtmosphereModel, density_exponential_kg_m3};
use sat_astro::constants::{MU_EARTH, OMEGA_EARTH, R_EARTH};
use sat_astro::orbit::{OrbitSpec, Propagator, sso_inclination_rad};
use sat_astro::time::Epoch;

fn epoch() -> Epoch {
    Epoch::parse("2027-04-01T00:00:00Z").unwrap()
}

fn circular(altitude_m: f64, inclination_deg: f64) -> OrbitSpec {
    OrbitSpec::circular(
        epoch(),
        Qty::from_raw(altitude_m),
        Qty::from_raw(inclination_deg.to_radians()),
        Qty::ZERO,
    )
}

/// 3U CubeSat（4 kg、10 × 10 × 34 cm を向きでならした面積 0.039 m²、Cd 2.2）。
fn cubesat_3u(altitude_m: f64) -> OrbitDecayInput {
    OrbitDecayInput {
        orbit: circular(altitude_m, 97.4),
        mass: Qty::from_raw(4.0),
        drag_area: None,
        body_size: Some(Vec3::new(0.1, 0.1, 0.34)),
        drag_coefficient: None,
        mission_duration: None,
        span: None,
        reentry_altitude: None,
        band: None,
        max_lifetime: None,
        propulsion: None,
        dated_weather: None,
    }
}

fn case<'a>(o: &'a OrbitDecayOutput, id: &str) -> &'a DecayCase {
    o.cases.iter().find(|c| c.id == id).unwrap()
}

fn years(q: Option<Qty<Time>>) -> f64 {
    q.map_or(f64::INFINITY, |l| l.value() / YEAR_S)
}

/// 赤道の円軌道・指数大気（高度だけで決まり、時刻で変わらない）では、周回平均の式は手計算の
/// `da/dt = −ρ (Cd·A/m) √(μa) (1 − ω_E a / v)²`（King-Hele の式に共回転大気の補正）、
/// 打ち消す推力は `½ ρ (Cd·A/m) (v − ω_E a)²` になる。
#[test]
fn equatorial_rates_match_king_hele_by_hand() {
    let h = 400e3;
    let orbit = circular(h, 0.0);
    let b = 0.01;
    let model = Model {
        base: &orbit,
        cd_a_over_m: b,
        weather: Weather::Constant(AtmosphereModel::Exponential),
    };
    let a = R_EARTH + h;
    let m = model.mean(0.0, a, 0.0).unwrap();
    let rho = density_exponential_kg_m3(h);
    let v = (MU_EARTH / a).sqrt();
    let wind = v - OMEGA_EARTH * a;
    let a_dot = -rho * b * (MU_EARTH * a).sqrt() * (wind / v).powi(2);
    // 密度は WGS84 の測地高度から引くので、赤道でも球の高度とわずかに違う。
    assert!(
        (m.density / rho - 1.0).abs() < 1e-5,
        "{} vs {rho}",
        m.density
    );
    assert!(
        (m.a_dot / a_dot - 1.0).abs() < 1e-5,
        "{} vs {a_dot}",
        m.a_dot
    );
    assert!((m.dv_rate / (0.5 * rho * b * wind * wind) - 1.0).abs() < 1e-5);
    // 1 年保つ ΔV は率 × 期間（大気が時刻でほとんど変わらないので。ECI の赤道と地球の赤道の歳差のずれだけ）。
    let held = simulate::hold(&model, a, YEAR_S, None).unwrap();
    let r = held.delta_v / (m.dv_rate * YEAR_S);
    assert!((r - 1.0).abs() < 1e-3, "{r}");
    // 帯を付けても 1 年の ΔV はほぼ同じ（帯の中の密度の差だけ増える）。
    let banded = simulate::hold(&model, a, YEAR_S, Some(2e3)).unwrap();
    assert!(banded.burns.unwrap() > 10);
    let ratio = banded.delta_v / held.delta_v;
    assert!((1.0..1.1).contains(&ratio), "{ratio}");
}

/// 500 km の 3U CubeSat の寿命は中程度の太陽活動で数年（Oltrogge & Leveque, "An Evaluation of CubeSat
/// Orbital Decay", SSC11-VII-2, 2011 の図: 500 km の 3U はおよそ 2〜10 年。NASA の CubeSat 101（2017）も
/// 「400 km は 1 年前後、500 km は数年、600 km は 10 年以上」）。太陽活動が高いほど短い。
#[test]
fn cubesat_lifetime_at_500_km_is_a_few_years() {
    let o = OrbitDecay::run(&cubesat_3u(500e3)).unwrap();
    let (low, moderate, high) = (
        years(case(&o, "low").lifetime),
        years(case(&o, "moderate").lifetime),
        years(case(&o, "high").lifetime),
    );
    assert!((1.5..10.0).contains(&moderate), "moderate {moderate} yr");
    assert!(high < moderate && moderate < low, "{high} {moderate} {low}");
    assert!(high > 0.3, "high {high} yr");
    // 400 km は中程度で 1 年前後（数か月〜2 年）
    let o = OrbitDecay::run(&cubesat_3u(400e3)).unwrap();
    let m400 = years(case(&o, "moderate").lifetime);
    assert!((0.2..2.0).contains(&m400), "400 km moderate {m400} yr");
}

/// ISS 級（m/(Cd·A) ≈ 130 kg/m²、高度 410 km、傾斜角 51.6°）の高度低下は月に 2 km 程度
/// （NASA の ISS の軌道上昇の説明: 1 か月に平均 2 km ほど下がる。太陽活動が高い時期はその数倍）。
#[test]
fn iss_like_decay_is_kilometres_per_month() {
    let input = OrbitDecayInput {
        orbit: circular(410e3, 51.6),
        mass: Qty::from_raw(420_000.0),
        drag_area: Some(Qty::from_raw(1_470.0)),
        drag_coefficient: Some(2.2),
        body_size: None,
        span: Some(Qty::from_raw(30.0 * SECONDS_PER_DAY)),
        ..cubesat_3u(410e3)
    };
    let o = OrbitDecay::run(&input).unwrap();
    let per_month = |id: &str| case(&o, id).decay_per_day.value() * 30.0 / 1e3;
    let (moderate, high) = (per_month("moderate"), per_month("high"));
    assert!(
        (0.8..6.0).contains(&moderate),
        "moderate {moderate} km/month"
    );
    assert!((2.0..20.0).contains(&high), "high {high} km/month");
    assert!(per_month("low") < moderate && moderate < high);
}

/// 日付で引いた列がプリセットと同じ値なら、結果もプリセットの条件と同じ。出所の数と、プリセットを使い始めた日を返す。
#[test]
fn dated_weather_matches_the_same_preset() {
    let mut input = cubesat_3u(450e3);
    input.span = Some(Qty::from_raw(2.0 * YEAR_S));
    input.mission_duration = Some(Qty::from_raw(0.5 * YEAR_S));
    let cutoff = epoch().plus_seconds(YEAR_S);
    let schedule = dated_schedule(epoch(), input.span_s(), SolarActivity::Moderate, |e| {
        let (f, fa, ap) = SolarActivity::Moderate.indices();
        if e.seconds_since(cutoff) < 0.0 {
            DatedSpaceWeather::manual(f, fa, ap)
        } else {
            DatedSpaceWeather::preset(SolarActivity::Moderate)
        }
    })
    .unwrap();
    input.dated_weather = Some(schedule);
    let o = OrbitDecay::run(&input).unwrap();
    let (dated, moderate) = (case(&o, "dated"), case(&o, "moderate"));
    assert_eq!(
        dated.condition,
        DecayCondition::Dated {
            fallback: SolarActivity::Moderate
        }
    );
    assert_eq!(dated.lifetime.is_some(), moderate.lifetime.is_some());
    assert!((dated.final_altitude.value() - moderate.final_altitude.value()).abs() < 1.0);
    // 標本の境目で刻みを切るぶんだけ積分の刻みが違う（1 周の平均は世界時の 1 日の変化を拾うので、刻みの違いで 1% ほど揺れる）。
    let dv = |c: &DecayCase| c.station_keeping.delta_v_total.value();
    let r = dv(dated) / dv(moderate);
    assert!((r - 1.0).abs() < 2e-2, "{r}");
    let s = dated.weather_sources.unwrap();
    for o in [s.f107, s.ap] {
        assert_eq!(s.samples, o.manual + o.preset);
        assert!(o.manual > 0 && o.preset > 0);
        assert!(o.preset_from.unwrap().seconds_since(cutoff) >= 0.0);
        assert!(o.predicted_from.is_none());
    }
    assert!(case(&o, "low").weather_sources.is_none());
}

/// 4 kg の機体の推進系: 化学推進 1 N・220 s、500 → 520 km のインパルスの上昇 1 つ、1 L のタンク。
fn propulsion(orbit: &OrbitSpec, dry_kg: f64) -> PropulsionInput {
    PropulsionInput {
        initial_orbit: orbit.clone(),
        dry_mass: Qty::from_raw(dry_kg),
        thrusters: vec![PropulsionThruster {
            id: "PROP".into(),
            thrust: Qty::from_raw(1.0),
            isp: Qty::from_raw(220.0),
            propellant: None,
            min_impulse_bit: None,
            rated_total_impulse: None,
            rated_firing_time: None,
            rated_firings: None,
            firing_power: None,
        }],
        tanks: vec![PropellantTank {
            id: "TANK".into(),
            volume: Qty::from_raw(1e-3),
            propellant: None,
            density: Some(Qty::from_raw(1000.0)),
        }],
        maneuvers: vec![ManeuverInput {
            id: "RAISE".into(),
            category: DvCategory::Insertion,
            method: TransferMethod::Impulsive,
            thruster: Some("PROP".into()),
            target: OrbitTarget {
                altitude: Some(Qty::from_raw(520e3)),
                ..OrbitTarget::default()
            },
            via_altitude: None,
            ignored_constraints: Vec::new(),
            thrust: ThrustConstraints::default(),
        }],
        dv_margin: None,
        residual_fraction: None,
        unloading: None,
        unloading_note: None,
    }
}

/// 軌道維持の推薬は推進系の解析（`propulsion.propellant`）の使える推薬。噴射前の質量を推薬を積んだ質量にすると、
/// 使える推薬を使い切った ΔV は `v_e ln(m_wet / (m_wet − m_usable)) = 1.1 × ホーマンの ΔV`（設計の ΔV。マージン 10 %）に戻る。
/// 推薬の寿命はそれを 1 年の ΔV で割ったもの。判定はミッション期間と比べる。
#[test]
fn propellant_life_and_checks() {
    let mut input = cubesat_3u(500e3);
    input.span = Some(Qty::from_raw(3.0 * YEAR_S));
    input.mission_duration = Some(Qty::from_raw(2.0 * YEAR_S));
    input.max_lifetime = Some(Qty::from_raw(25.0 * YEAR_S));
    let p = propulsion(&input.orbit, 4.0);
    let usable = crate::propulsion::Propellant::run(&p)
        .unwrap()
        .usable_propellant
        .value();
    input.propulsion = Some(p);
    let hohmann = sat_astro::transfer::hohmann(MU_EARTH, R_EARTH + 500e3, R_EARTH + 520e3)
        .unwrap()
        .total_m_s();
    let capacity = 1.1 * hohmann;
    let o = OrbitDecay::run(&input).unwrap();
    assert!((o.used.propulsion_delta_v.unwrap().value() - capacity).abs() < 1e-6);
    assert!((o.used.propellant.unwrap().value() - usable).abs() < 1e-12);
    assert_eq!(o.used.thruster.as_deref(), Some("PROP"));
    for c in &o.cases {
        let sk = &c.station_keeping;
        let life = sk.propellant_life.unwrap().value();
        assert!((life * sk.delta_v_per_year.value() / YEAR_S - capacity).abs() < 1e-6);
        assert!(sk.burns.is_none());
    }
    let verdict = |key: &str| o.checks.iter().find(|c| c.key == key).unwrap().verdict;
    // 高い太陽活動では 2 年もたない → 軌道維持が要る（注意）
    assert_eq!(verdict("lifetime_vs_mission"), Verdict::Warning);
    assert!(o.checks.iter().any(|c| c.key == "propellant_life"));
    // 代表（中程度）は 3 年以内に落ちるので 25 年の上限を満たす
    assert_eq!(verdict("max_lifetime"), Verdict::Ok);
    // 上限を 1 年にすると違反、積分を 1 年にして上限を 25 年にすると確かめられない（注意）
    input.max_lifetime = Some(Qty::from_raw(YEAR_S));
    let o1 = OrbitDecay::run(&input).unwrap();
    let v1 = o1.checks.iter().find(|c| c.key == "max_lifetime").unwrap();
    assert_eq!(v1.verdict, Verdict::Violation);
    input.max_lifetime = Some(Qty::from_raw(25.0 * YEAR_S));
    input.span = Some(Qty::from_raw(YEAR_S));
    let o2 = OrbitDecay::run(&input).unwrap();
    let v2 = o2.checks.iter().find(|c| c.key == "max_lifetime").unwrap();
    assert_eq!(v2.verdict, Verdict::Warning);
    assert!(o.checks.iter().all(|c| c.key != "near_circular"));
}

#[test]
fn tumbling_area_is_a_quarter_of_the_surface() {
    // 立方体の辺 1 m: 表面積 6 m² → 1.5 m²
    assert_eq!(tumbling_area(Vec3::new(1.0, 1.0, 1.0)), 1.5);
    let i = cubesat_3u(500e3);
    let u = used_values(&i).unwrap();
    assert!((u.drag_area.value() - 0.039).abs() < 1e-12);
    assert_eq!(u.area_source, DragAreaSource::TumblingBox);
    assert_eq!(u.drag_coefficient, DEFAULT_DRAG_COEFFICIENT);
}

#[test]
fn invalid_inputs_are_errors() {
    let mut i = cubesat_3u(100e3);
    assert!(OrbitDecay::run(&i).is_err());
    i = cubesat_3u(500e3);
    i.body_size = None;
    assert!(OrbitDecay::run(&i).is_err());
    // 推進系の解析が通らなければ（比推力が 0）高度低下も誤り
    i = cubesat_3u(500e3);
    let mut p = propulsion(&i.orbit, 4.0);
    p.thrusters[0].isp = Qty::ZERO;
    i.propulsion = Some(p);
    assert!(OrbitDecay::run(&i).is_err());
}

/// 太陽同期軌道（J2 の歳差あり）でも RAAN の歳差を追い、離心率が大きければ注意を出す。
#[test]
fn sun_synchronous_orbit_and_eccentricity_warning() {
    let a = R_EARTH + 550e3;
    let mut orbit = circular(550e3, sso_inclination_rad(a, 0.0).to_degrees());
    assert_eq!(orbit.propagator, Propagator::J2);
    orbit.elements.eccentricity = 0.02;
    let input = OrbitDecayInput {
        orbit,
        span: Some(Qty::from_raw(YEAR_S)),
        ..cubesat_3u(550e3)
    };
    let o = OrbitDecay::run(&input).unwrap();
    assert!(o.checks.iter().any(|c| c.key == "near_circular"));
    let high = case(&o, "high");
    assert!(high.decay_first_year.value() > 0.0);
    assert!(high.series.t.len() > 10);
    assert_eq!(high.series.channels[0].kind, ChannelKind::Altitude);
}
