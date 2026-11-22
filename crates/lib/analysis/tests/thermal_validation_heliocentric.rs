//! 熱解析の検証（5）: 太陽を中心天体にした軌道（惑星間の巡航）。太陽光だけの入熱（アルベド・天体の赤外は 0、食なし）で、
//! 太陽距離 d [m] の太陽光は `S₁ (AU/d)²`（S₁ = 1 AU の太陽光）。期待値はすべて閉じた式。
//! - 太陽光だけを受ける 1 節点の平衡温度 `T⁴ = α S A_abs / (ε σ A_rad)` は d^(-1/2) に比例する（平板・立方体）。
//! - 楕円軌道の周回平均（定常）は、時間平均 ⟨(a/r)²⟩ = 1/√(1−e²)（ケプラーの面積速度一定から）を使う。
//! - 非定常は、時定数（数分）が軌道の変化（数か月）よりずっと短いので、太陽距離に準静的に追従する。

mod common;

use common::*;
use sat_analysis::thermal::{ThermalEnvironment, ThermalExposure};
use sat_astro::attitude::AttitudeLaw;
use sat_astro::constants::{AU, SECONDS_PER_DAY, SIGMA_SB};
use sat_astro::ephem::CentralBody;
use sat_astro::math::Vec3;
use sat_astro::orbit::{OrbitElements, OrbitSpec, Propagator};
use sat_astro::time::Epoch;
use sat_astro::timeline::TimelineSpec;
use sat_units::typed::Qty;

const MASS_KG: f64 = 0.005;
const ALPHA: f64 = 0.92;
const EPS: f64 = 0.86;
const AREA: f64 = 0.01;

/// 太陽を回る軌道（長半径 `a`・離心率 `e`・近日点から開始）で太陽指向（+X が太陽）の時系列。
fn helio_timeline(a: f64, e: f64, span_s: f64, step_s: f64) -> TimelineSpec {
    let elements = OrbitElements {
        semi_major_axis: Qty::from_raw(a),
        eccentricity: e,
        inclination: Qty::from_raw(0.1),
        raan: Qty::from_raw(0.0),
        arg_perigee: Qty::from_raw(0.0),
        mean_anomaly: Qty::from_raw(0.0),
    };
    let mut orbit = OrbitSpec::new(
        Epoch::parse("2030-01-01T00:00:00Z").unwrap(),
        elements,
        Propagator::Kepler,
    );
    orbit.central = CentralBody::Sun;
    let law = AttitudeLaw::SunPointing {
        axis: Vec3::X,
        secondary: Vec3::Z,
    };
    TimelineSpec::new(orbit, law, Qty::from_raw(span_s), Qty::from_raw(step_s))
}

/// 太陽光の強さを書かない熱環境（距離から決まる）。背景温度は 0 K。
fn env_from_distance() -> ThermalEnvironment {
    ThermalEnvironment {
        space_temperature: Some(Qty::from_raw(0.0)),
        ..ThermalEnvironment::default()
    }
}

/// 太陽を向く 1 面だけが吸収する平板。
fn plate() -> sat_analysis::thermal::ThermalNetwork {
    network(
        vec![al_node("n", MASS_KG)],
        vec![surface("front", "n", AREA, Vec3::X, ALPHA, EPS)],
    )
}

/// 平衡温度 `(α S A_abs / (ε σ A_rad))^¼`（S は太陽光 [W/m²]）。
fn t_for_flux(s: f64, a_abs: f64, a_rad: f64) -> f64 {
    (ALPHA * s * a_abs / (EPS * SIGMA_SB * a_rad)).powf(0.25)
}

/// 円軌道（距離一定）の定常: 平板は `T = t_eq(1 AU)·(AU/d)^½`。0.5・1・2 AU で閉じた式と一致し、
/// 0.5 AU と 2 AU の比がちょうど 2（T ∝ d^(-1/2)）。
#[test]
fn steady_plate_follows_the_inverse_square_root_of_distance() {
    let mut temps = Vec::new();
    for d_au in [0.5_f64, 1.0, 2.0] {
        let d = d_au * AU;
        let period = helio_timeline(d, 0.0, 1.0, 1.0).orbit().period_s();
        let tl = helio_timeline(d, 0.0, period, period / 200.0);
        let case = steady(
            plate(),
            env_from_distance(),
            ThermalExposure::OrbitAverage { timeline: tl },
            vec![],
        );
        let t = case.nodes[0].temperature.value();
        let expected = t_eq(ALPHA, EPS, AREA, AREA, 0.0) / d_au.sqrt();
        assert!((t - expected).abs() < 1e-6, "{d_au} AU: {t} vs {expected}");
        temps.push(t);
    }
    assert!((temps[0] / temps[2] - 2.0).abs() < 1e-9);
}

/// 立方体（1 面だけが太陽を向く。A_abs = a²・A_rad = 6a²）も同じ d^(-1/2)。
#[test]
fn steady_cube_follows_the_inverse_square_root_of_distance() {
    let a = 0.1;
    for d_au in [0.7_f64, 1.5] {
        let d = d_au * AU;
        let period = helio_timeline(d, 0.0, 1.0, 1.0).orbit().period_s();
        let net = network(
            vec![al_node("n", MASS_KG)],
            cube_surfaces("n", a, ALPHA, EPS),
        );
        let tl = helio_timeline(d, 0.0, period, period / 200.0);
        let case = steady(
            net,
            env_from_distance(),
            ThermalExposure::OrbitAverage { timeline: tl },
            vec![],
        );
        let expected = t_eq(ALPHA, EPS, a * a, 6.0 * a * a, 0.0) / d_au.sqrt();
        let t = case.nodes[0].temperature.value();
        assert!((t - expected).abs() < 1e-6, "{d_au} AU: {t} vs {expected}");
    }
}

/// 書いたアルベド・天体の赤外・プリセットは太陽中心では使わない（太陽に反射も天体の赤外も無い）。
/// 地球周回の高温の値を書いても、結果は書かないときと同じ。
#[test]
fn written_albedo_and_planet_ir_are_not_used_about_the_sun() {
    let d = AU;
    let period = helio_timeline(d, 0.0, 1.0, 1.0).orbit().period_s();
    let run = |environment| {
        let tl = helio_timeline(d, 0.0, period, period / 200.0);
        steady(
            plate(),
            environment,
            ThermalExposure::OrbitAverage { timeline: tl },
            vec![],
        )
        .nodes[0]
            .temperature
            .value()
    };
    let plain = run(env_from_distance());
    let written = run(ThermalEnvironment {
        preset: Some(sat_analysis::thermal::ThermalPreset::Hot),
        albedo: Some(sat_units::typed::Fraction::new(0.35)),
        planet_ir: Some(Qty::from_raw(258.0)),
        ..env_from_distance()
    });
    assert_eq!(plain, written);
}

/// 楕円軌道（近日点 0.7 AU・遠日点 1.5 AU）の周回平均（定常）: 入熱の平均 ∝ ⟨(AU/r)²⟩ = (AU/a)²/√(1−e²)（時間平均）。
/// 平板の T⁴ = α S₁ (AU/a)² / (√(1−e²) ε σ)。標本は周期の関数の台形則（周期関数なのでごく小さい誤差）。
#[test]
fn steady_orbit_average_of_an_ellipse_uses_the_time_mean_of_inverse_square() {
    let (rp, ra) = (0.7 * AU, 1.5 * AU);
    let (a, e) = ((rp + ra) / 2.0, (ra - rp) / (ra + rp));
    let period = helio_timeline(a, e, 1.0, 1.0).orbit().period_s();
    let tl = helio_timeline(a, e, period, period / 400.0);
    let case = steady(
        plate(),
        env_from_distance(),
        ThermalExposure::OrbitAverage { timeline: tl },
        vec![],
    );
    let mean_flux = SOLAR * (AU / a).powi(2) / (1.0 - e * e).sqrt();
    let expected = t_for_flux(mean_flux, AREA, AREA);
    let t = case.nodes[0].temperature.value();
    assert!(
        (t - expected).abs() < 1e-3,
        "{t} vs {expected} (mean flux {mean_flux})"
    );
}

/// 非定常（円軌道）: 200 K から始めて 30 τ 後に、距離の平衡温度に収束する。0.5 AU と 2 AU で比が 2。
/// 軌道の期間（1 年）に比べて 30 τ（数時間）は短く、距離は一定（円軌道）なので誤差は積分の誤差だけ。
#[test]
fn transient_plate_converges_to_the_distance_equilibrium() {
    let tau = |t_k: f64| MASS_KG * CP_AL / (4.0 * EPS * SIGMA_SB * AREA * t_k.powi(3));
    let mut finals = Vec::new();
    for d_au in [0.5_f64, 2.0] {
        let expected = t_eq(ALPHA, EPS, AREA, AREA, 0.0) / d_au.sqrt();
        let span = 30.0 * tau(expected).max(tau(200.0));
        let tl = helio_timeline(d_au * AU, 0.0, span, 10.0);
        let out = transient(plate(), tl, env_from_distance(), vec![], uniform(200.0));
        let last = *values(&out.series, "n").last().unwrap();
        assert!(
            (last - expected).abs() < 1e-6,
            "{d_au} AU: {last} vs {expected}"
        );
        finals.push(last);
        // 環境: アルベド・赤外は 0、太陽光は距離から（書いていない）。軌道面の図は太陽中心（半径 0・日陰なし）
        assert_eq!(out.environment.albedo.value(), 0.0);
        assert_eq!(out.environment.planet_ir.map(Qty::value), Some(0.0));
        assert!(out.environment.solar_flux.is_none());
        assert_eq!(out.orbit_plane.body_radius.value(), 0.0);
        assert!(out.orbit_plane.eclipse.iter().all(|e| !e));
    }
    assert!((finals[0] / finals[1] - 2.0).abs() < 1e-6);
}

/// 非定常（楕円軌道を半年）: 時定数が軌道の変化よりずっと短いので、温度は各時刻の太陽距離の平衡温度に準静的に追従する
/// （遅れ ≈ τ·dT/dt は 1e-2 K 未満）。距離はケプラー伝搬の位置の大きさ（`OrbitSpec::state_at`）から。
#[test]
fn transient_follows_the_equilibrium_along_an_ellipse() {
    let (rp, ra) = (0.7 * AU, 1.5 * AU);
    let (a, e) = ((rp + ra) / 2.0, (ra - rp) / (ra + rp));
    let tl = helio_timeline(a, e, 120.0 * SECONDS_PER_DAY, SECONDS_PER_DAY);
    let orbit = tl.orbit().clone();
    let equilibrium = |t: f64| {
        let r = orbit.state_at(t).unwrap().r_eci.norm();
        t_eq(ALPHA, EPS, AREA, AREA, 0.0) * (AU / r).sqrt()
    };
    let out = transient(
        plate(),
        tl,
        env_from_distance(),
        vec![],
        uniform(equilibrium(0.0)),
    );
    let temps = values(&out.series, "n");
    let times = &out.series.t;
    assert!(temps.len() > 100);
    for (t, temp) in times.iter().zip(temps) {
        let want = equilibrium(*t);
        assert!((temp - want).abs() < 1e-2, "t = {t} s: {temp} vs {want}");
    }
    // 近日点（0.7 AU）から離れるにつれ温度が下がる
    assert!(temps[0] > *temps.last().unwrap() + 20.0);
}

/// 太陽中心でも、定常解析の固定の向き（最悪ケース）は太陽光の強さを書かないと入力エラー（1 AU の慣例値を黙って使わない）。
#[test]
fn fixed_exposure_about_the_sun_needs_the_flux() {
    use sat_analysis::spec::Analysis;
    let exposure = ThermalExposure::Fixed {
        sun_dir_body: Some(Vec3::X),
        nadir_body: None,
        altitude: Qty::from_raw(1.0e9),
        central: CentralBody::Sun,
    };
    let input = sat_analysis::thermal::ThermalSteadyInput {
        network: plate(),
        cases: vec![sat_analysis::thermal::steady::ThermalSteadyCase {
            id: "c".to_owned(),
            label: String::new(),
            environment: env_from_distance(),
            exposure,
            heaters: Vec::new(),
            loads: Vec::new(),
            mode_loads: None,
        }],
        margin: sat_analysis::thermal::ThermalMarginPolicy::default(),
    };
    assert!(sat_analysis::thermal::ThermalSteady::run(&input).is_err());
}
