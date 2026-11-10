use super::*;
use crate::constants::{AU, C_LIGHT, GM_MOON, R_EARTH, SOLAR_CONSTANT};

fn epoch() -> Epoch {
    Epoch::parse("2027-04-01T00:00:00Z").expect("元期")
}

fn leo() -> OrbitState {
    let r = R_EARTH + 550e3;
    OrbitState {
        r_eci: Vec3::new(r * 0.6, r * 0.8, 0.0),
        v_eci: Vec3::new(-0.8 * 7.6e3, 0.6 * 7.6e3, 0.0),
    }
}

fn config(gravity: GravityModel) -> ForceConfig {
    ForceConfig::gravity_only(gravity)
}

#[test]
fn point_mass_magnitude() {
    let a = acceleration_eci(&config(GravityModel::PointMass), epoch(), &leo());
    let r = R_EARTH + 550e3;
    assert!((a.norm() - MU_EARTH / (r * r)).abs() < 1e-12);
    assert!(a.dot(leo().r_eci) < 0.0);
}

#[test]
fn j2_model_matches_egm96_zonal_degree_two() {
    // 赤道面内でない点で比べる（μ・R・J2 の定数の差 ~1e-9 以内）
    let s = OrbitState {
        r_eci: Vec3::new(3.9e6, -2.7e6, 4.9e6),
        v_eci: Vec3::ZERO,
    };
    let a_j2 = acceleration_eci(&config(GravityModel::J2), epoch(), &s);
    let a_h = acceleration_eci(
        &config(GravityModel::Harmonics {
            degree: 2,
            order: 0,
        }),
        epoch(),
        &s,
    );
    let pert_j2 = a_j2 - point_mass(MU_EARTH, s.r_eci);
    let pert_h = a_h - point_mass(MU_EARTH, s.r_eci);
    // 中心項の μ の差（7.5e-10 相対）を除いた J2 項どうしが 1e-6 相対で一致
    let mu_diff = point_mass(3.986_004_415e14 - MU_EARTH, s.r_eci);
    let rel = ((pert_h - mu_diff) - pert_j2).norm() / pert_j2.norm();
    assert!(rel < 1e-6, "{rel}");
}

#[test]
fn moon_third_body_tidal_magnitude() {
    // 月の潮汐加速度の大きさ ~ 2 μ r / d³ ≈ 1e-6 m/s²（月の方向では 2 次の項で 3 % ほど大きい）
    let r_moon = position_eci_m(Body::Moon, epoch());
    let r = r_moon.normalized().expect("非ゼロ") * (R_EARTH + 550e3);
    let a = third_body_eci(GM_MOON, r_moon, r);
    let expected = 2.0 * GM_MOON * r.norm() / r_moon.norm().powi(3);
    assert!(
        (a.norm() / expected - 1.0).abs() < 0.04,
        "{} vs {expected}",
        a.norm()
    );
}

#[test]
fn srp_points_away_from_sun_and_vanishes_in_shadow() {
    let r_sun = position_eci_m(Body::Sun, epoch());
    let s_hat = r_sun.normalized().expect("非ゼロ");
    let p = SrpParams {
        cr: 1.5,
        area_to_mass: 0.01,
    };
    let lit = s_hat * (R_EARTH + 550e3);
    let a = srp_eci(&p, lit, r_sun);
    let expected = SOLAR_CONSTANT / C_LIGHT * 1.5 * 0.01 * (AU / (r_sun - lit).norm()).powi(2);
    assert!((a.norm() / expected - 1.0).abs() < 1e-12);
    assert!(a.dot(s_hat) < 0.0);
    assert_eq!(srp_eci(&p, -lit, r_sun), Vec3::ZERO);
}

#[test]
fn drag_opposes_relative_velocity() {
    let d = DragParams {
        cd: 2.2,
        area_to_mass: 0.01,
        atmosphere: AtmosphereModel::Exponential,
    };
    let s = leo();
    let a = drag_eci(&d, epoch(), &s);
    assert!(a.dot(s.v_eci) < 0.0);
    // 550 km: ρ ≈ 3.2e-13、v_rel ≈ 7.1 km/s → |a| ≈ 0.5·ρ·Cd·A/m·v² ≈ 1.8e-7 m/s²
    assert!(a.norm() > 1e-7 && a.norm() < 3e-7, "{}", a.norm());
}

#[test]
fn drag_force_by_hand() {
    // ρ = 1e-12 kg/m³、Cd·A = 0.1 m²、v = 7.5 km/s（+x）→ F = −½·1e-12·0.1·7500² = −2.8125e-6 N（x）
    let f = drag_force(1e-12, 0.1, Vec3::new(7500.0, 0.0, 0.0));
    assert!((f.x + 2.8125e-6).abs() < 1e-18, "{f:?}");
    assert!(f.y == 0.0 && f.z == 0.0);
    assert_eq!(drag_force(1e-12, 0.1, Vec3::ZERO), Vec3::ZERO);
}

#[test]
fn j2_on_the_equator_and_over_the_pole_by_hand() {
    use crate::constants::{J2_EARTH, MU_EARTH};
    // 赤道上 r では内向きに (3/2) J2 μ R²/r⁴、極の上では外向きに 3 J2 μ R²/r⁴（7000 km で約 1.10e-2 m/s²）
    let r = 7.0e6_f64;
    let unit = J2_EARTH * MU_EARTH * R_EARTH * R_EARTH / r.powi(4);
    let eq = j2_perturbation(Vec3::new(r, 0.0, 0.0));
    assert!((eq.x / (-1.5 * unit) - 1.0).abs() < 1e-12, "{eq:?}");
    assert!((eq.x + 1.097e-2).abs() < 1e-5);
    let pole = j2_perturbation(Vec3::new(0.0, 0.0, r));
    assert!((pole.z / (3.0 * unit) - 1.0).abs() < 1e-12, "{pole:?}");
    // 地球の J2 は z 軸まわりの zonal_j2 そのもの
    let p = Vec3::new(3.0e6, -4.0e6, 5.0e6);
    assert_eq!(
        j2_perturbation(p),
        zonal_j2(MU_EARTH, R_EARTH, J2_EARTH, Vec3::Z, p)
    );
}

#[test]
fn validate_rejects_bad_configs() {
    let bad = config(GravityModel::Harmonics {
        degree: 71,
        order: 0,
    });
    assert!(validate(&bad).is_err());
    let bad_order = config(GravityModel::Harmonics {
        degree: 4,
        order: 5,
    });
    assert!(validate(&bad_order).is_err());
    let mut neg = config(GravityModel::J2);
    neg.drag = Some(DragParams {
        cd: -1.0,
        area_to_mass: 0.01,
        atmosphere: AtmosphereModel::Exponential,
    });
    assert!(validate(&neg).is_err());
    assert!(
        validate(&config(GravityModel::Harmonics {
            degree: 70,
            order: 70
        }))
        .is_ok()
    );
}

fn full_config() -> ForceConfig {
    ForceConfig {
        gravity: GravityModel::Harmonics {
            degree: 8,
            order: 8,
        },
        third_bodies: vec![Body::Sun, Body::Moon, Body::Jupiter],
        srp: Some(SrpParams {
            cr: 1.3,
            area_to_mass: 0.01,
        }),
        drag: Some(DragParams {
            cd: 2.2,
            area_to_mass: 0.01,
            atmosphere: AtmosphereModel::Nrlmsise00 {
                f107: 150.0,
                f107a: 150.0,
                ap: 4.0,
            },
        }),
        solid_tide: true,
        relativity: true,
    }
}

#[test]
fn breakdown_sums_to_total_and_lists_every_term() {
    let cfg = full_config();
    let s = leo();
    let parts = acceleration_breakdown(&cfg, epoch(), &s);
    let sum = parts.iter().fold(Vec3::ZERO, |a, (_, v)| a + *v);
    assert_eq!(sum, acceleration_eci(&cfg, epoch(), &s));
    let terms: Vec<ForceTerm> = parts.iter().map(|(t, _)| *t).collect();
    assert_eq!(
        terms,
        [
            ForceTerm::CentralBody,
            ForceTerm::NonSpherical,
            ForceTerm::ThirdBody(Body::Sun),
            ForceTerm::ThirdBody(Body::Moon),
            ForceTerm::ThirdBody(Body::Jupiter),
            ForceTerm::SolarRadiationPressure,
            ForceTerm::Drag,
            ForceTerm::SolidTide,
            ForceTerm::Relativity,
        ]
    );
    // 大きさの並び: 中心 ≫ 非球形 ≫ 月・太陽 ≫ 潮汐・相対論・SRP・抗力
    let mag = |t: ForceTerm| {
        parts
            .iter()
            .find(|(x, _)| *x == t)
            .map(|(_, v)| v.norm())
            .unwrap_or(0.0)
    };
    assert!(mag(ForceTerm::CentralBody) > 8.0);
    assert!(mag(ForceTerm::NonSpherical) > 1e-3 && mag(ForceTerm::NonSpherical) < 3e-2);
    assert!(
        mag(ForceTerm::ThirdBody(Body::Moon)) > 3e-7
            && mag(ForceTerm::ThirdBody(Body::Moon)) < 3e-6
    );
    assert!(mag(ForceTerm::ThirdBody(Body::Jupiter)) < 1e-11);
    assert!(mag(ForceTerm::Relativity) > 1e-8 && mag(ForceTerm::Relativity) < 3e-8);
    let only = acceleration_breakdown(&config(GravityModel::PointMass), epoch(), &s);
    assert_eq!(only.len(), 1);
}

/// 円軌道での相対論の補正は外向きで 3μ²/(c² r³)。
#[test]
fn relativity_matches_circular_closed_form() {
    let s = leo();
    let r = s.r_eci.norm();
    let v = (MU_EARTH / r).sqrt();
    let state = OrbitState {
        r_eci: s.r_eci,
        v_eci: s.v_eci.normalized().expect("非ゼロ") * v,
    };
    let a = relativity_eci(MU_EARTH, &state);
    let expected = 3.0 * MU_EARTH * MU_EARTH / (C_LIGHT * C_LIGHT * r.powi(3));
    assert!((a.norm() / expected - 1.0).abs() < 1e-9);
    assert!(a.dot(state.r_eci) > 0.0);
}

#[test]
fn harmonics_split_equals_full_field() {
    let s = leo();
    let g = GravityModel::Harmonics {
        degree: 20,
        order: 20,
    };
    let parts = acceleration_breakdown(&config(g), epoch(), &s);
    let total = parts[0].1 + parts[1].1;
    // 中心項は 1/r² の点質量、残りは 2 次以上（1 次の係数は 0）
    let central_expected = point_mass(3.986_004_415e14, s.r_eci);
    assert!((parts[0].1 - central_expected).norm() < 1e-12);
    assert!((total - acceleration_eci(&config(g), epoch(), &s)).norm() == 0.0);
}

#[test]
fn full_force_model_propagates() {
    use crate::orbit::{OrbitSpec, Propagator};
    use sat_units::typed::Qty;
    let mut spec = OrbitSpec::circular(
        epoch(),
        Qty::from_raw(550e3),
        Qty::from_raw(97.6_f64.to_radians()),
        Qty::from_raw(0.3),
    );
    let mut reference = spec.clone();
    reference.propagator = Propagator::Numerical {
        forces: ForceConfig::gravity_only(GravityModel::J2),
    };
    spec.propagator = Propagator::Numerical {
        forces: full_config(),
    };
    let t = 2.0 * spec.period_s();
    let s = spec.state_at(t).expect("数値伝搬");
    let j2 = reference.state_at(t).expect("J2 の数値伝搬");
    // 同じ初期状態の J2 だけの数値伝搬との差は、2 周で高次の重力場などの分（数 km 以内）。
    // 注: J2 永年項の伝搬（平均要素）と比べると、平均要素と接触要素の違いで 2 周 ~180 km ずれる。
    let d = (s.r_eci - j2.r_eci).norm();
    assert!(d > 1.0 && d < 5e3, "{d} m");
    assert!((s.r_eci.norm() - (R_EARTH + 550e3)).abs() < 30e3);
}

#[test]
fn srp_pressure_and_relative_wind() {
    // 1 AU で 1361 W/m² / c ≈ 4.54e-6 N/m²
    assert!((srp_pressure(AU) - SOLAR_CONSTANT / C_LIGHT).abs() < 1e-18);
    assert!((srp_pressure(AU) - 4.54e-6).abs() < 0.01e-6);
    // 赤道上の静止点（地球と一緒に回る）は風 0
    let r = Vec3::new(R_EARTH + 400e3, 0.0, 0.0);
    let v = Vec3::new(0.0, crate::constants::OMEGA_EARTH * r.x, 0.0);
    assert!(relative_wind_eci(r, v).norm() < 1e-9);
}
