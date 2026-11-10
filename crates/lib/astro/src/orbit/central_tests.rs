//! 地球以外の中心天体（月）の伝搬・時系列・地上軌跡・パスのテスト。

use super::*;
use crate::attitude::AttitudeLaw;
use crate::constants::R_MOON;
use crate::ephem::orientation::pole_eci;
use crate::force::{ForceConfig, ForceTerm, GravityModel, acceleration_breakdown_about};
use crate::shadow::ShadowModel;
use crate::timeline::{TimelineSpec, build, mean_sunlit};

fn epoch() -> Epoch {
    Epoch::parse("2027-04-01T00:00:00Z").expect("元期")
}

fn lunar(inclination_deg: f64, propagator: Propagator) -> OrbitSpec {
    let mut o = OrbitSpec::circular_about(
        CentralBody::Moon,
        epoch(),
        Qty::from_raw(100e3),
        Qty::from_raw(inclination_deg.to_radians()),
        Qty::from_raw(0.4),
    );
    o.propagator = propagator;
    o
}

/// 月 100 km の円軌道の周期は ≈ 117.8 分（LRO の運用軌道と同じ）。
#[test]
fn lunar_period() {
    let p = lunar(90.0, Propagator::Kepler).period_s() / 60.0;
    assert!((p - 117.8).abs() < 0.2, "{p}");
    let s = lunar(90.0, Propagator::Kepler)
        .state_at(1234.0)
        .expect("状態");
    assert!((s.r_eci.norm() - (R_MOON + 100e3)).abs() < 1e-6);
}

/// 月の J2 は月の自転軸まわりに軌道面を回す: 軌道面法線と月の極のなす角は一定で、ECI の z とのなす角は変わる。
#[test]
fn lunar_j2_precesses_about_the_lunar_pole() {
    let o = lunar(50.0, Propagator::J2);
    let pole = pole_eci(CentralBody::Moon, o.epoch);
    let n0 = o.state_at(0.0).expect("状態").orbit_normal();
    let n1 = o.state_at(60.0 * 86_400.0).expect("状態").orbit_normal();
    let (a0, a1) = (n0.angle_to(pole), n1.angle_to(pole));
    assert!((a0 - a1).abs() < 1e-9, "{a0} vs {a1}");
    assert!((n0.angle_to(Vec3::Z) - n1.angle_to(Vec3::Z)).abs() > 1e-3);
    // Kepler なら面は動かない
    let k = lunar(50.0, Propagator::Kepler);
    let m0 = k.state_at(0.0).expect("状態").orbit_normal();
    let m1 = k.state_at(60.0 * 86_400.0).expect("状態").orbit_normal();
    assert!(m0.angle_to(m1) < 1e-12);
}

/// 月の周回軌道では地球が第三体として自動で入り、その潮汐加速度は ~2μ_E r / d³。
#[test]
fn earth_is_a_third_body_around_the_moon() {
    let o = lunar(90.0, Propagator::Kepler);
    let s = o.state_at(0.0).expect("状態");
    let parts = acceleration_breakdown_about(
        CentralBody::Moon,
        &ForceConfig::gravity_only(GravityModel::J2),
        epoch(),
        &s,
    );
    let earth = parts
        .iter()
        .find(|(t, _)| *t == ForceTerm::EarthThirdBody)
        .map(|(_, a)| a.norm())
        .expect("地球の項");
    let d = CentralBody::Moon.position_eci_m(epoch()).norm();
    let scale = 2.0 * MU_EARTH * s.r_eci.norm() / d.powi(3);
    assert!(
        earth > 0.3 * scale && earth < 1.5 * scale,
        "{earth} vs {scale}"
    );
    let central = parts[0].1.norm();
    assert!((central - CentralBody::Moon.gm() / s.r_eci.norm_squared()).abs() < 1e-12);
    // EGM96・抗力は地球だけ
    let bad = ForceConfig::gravity_only(GravityModel::Harmonics {
        degree: 4,
        order: 4,
    });
    assert!(crate::force::validate_about(CentralBody::Moon, &bad).is_err());
}

/// 数値伝搬（月の点質量＋J2＋地球）は 1 周で解析解から数 km 以内。
#[test]
fn lunar_numerical_stays_close_to_kepler() {
    let mut o = lunar(90.0, Propagator::Kepler);
    let t = o.period_s();
    let kepler = o.state_at(t).expect("Kepler");
    o.propagator = Propagator::Numerical {
        forces: ForceConfig::gravity_only(GravityModel::J2),
    };
    let num = o.state_at(t).expect("数値伝搬");
    let d = (num.r_eci - kepler.r_eci).norm();
    assert!(d > 1.0 && d < 20e3, "{d} m");
}

/// 月 100 km・β = 0 の日陰率は (1/π)·acos(√(h²+2Rh)/r) ≈ 0.394（円柱影）。
#[test]
fn lunar_eclipse_fraction_at_beta_zero() {
    let o = lunar(90.0, Propagator::Kepler);
    let span = o.period_s();
    let mut spec = TimelineSpec::new(
        o,
        AttitudeLaw::Inertial {
            q_body_to_eci: crate::math::Quat::IDENTITY,
        },
        Qty::from_raw(span),
        Qty::from_raw(5.0),
    );
    spec.shadow = ShadowModel::Cylindrical;
    spec.sampling.beta = Some(Qty::from_raw(0.0));
    let eclipse = 1.0 - mean_sunlit(&build(&spec).expect("時系列"));
    let (h, r) = (100e3, R_MOON + 100e3);
    let expected = ((h * h + 2.0 * R_MOON * h).sqrt() / r).acos() / core::f64::consts::PI;
    assert!(
        (eclipse - expected).abs() < 0.003,
        "{eclipse} vs {expected}"
    );
}

#[test]
fn lunar_ground_track_and_earth_passes() {
    use crate::ground::{GroundStation, find_passes, ground_track, look_angles};
    let o = lunar(90.0, Propagator::Kepler);
    let track = ground_track(&o, epoch(), o.period_s(), 60.0).expect("月の地上軌跡");
    let max_lat = track
        .iter()
        .fold(0.0_f64, |m, p| m.max(p.lat_rad.abs()))
        .to_degrees();
    assert!(max_lat > 60.0, "{max_lat}");
    assert!(track.iter().all(|p| (p.alt_m - 100e3).abs() < 1.0));
    let mars = OrbitSpec::circular_about(
        CentralBody::Mars,
        epoch(),
        Qty::from_raw(300e3),
        Qty::from_raw(1.0),
        Qty::from_raw(0.0),
    );
    assert!(ground_track(&mars, epoch(), 600.0, 60.0).is_err());
    // 地上局からは、月を回る衛星は月とほぼ同じ方向に見える（月が出ている間がパス）
    let gs = GroundStation {
        id: String::from("GS"),
        latitude: Qty::from_raw(35f64.to_radians()),
        longitude: Qty::from_raw(139f64.to_radians()),
        altitude: Qty::from_raw(0.0),
        min_elevation: Qty::from_raw(5f64.to_radians()),
    };
    let passes = find_passes(&gs, &o, epoch(), 2.0 * 86_400.0, 300.0).expect("パス");
    assert!(!passes.is_empty());
    let end = epoch().plus_seconds(2.0 * 86_400.0);
    for p in &passes {
        let inside = p.aos.j2000_s > epoch().j2000_s && p.los.j2000_s < end.j2000_s;
        assert!(
            !inside || p.duration.value() > 6.0 * 3600.0,
            "{}",
            p.duration.value()
        );
        let moon_el =
            look_angles(&gs, p.tca, CentralBody::Moon.position_eci_m(p.tca)).elevation_rad;
        assert!((moon_el - p.max_elevation.value()).abs().to_degrees() < 1.0);
    }
}

#[test]
fn central_defaults_to_earth_in_json() {
    let o = OrbitSpec::circular(
        epoch(),
        Qty::from_raw(550e3),
        Qty::from_raw(1.0),
        Qty::from_raw(0.0),
    );
    let mut v = serde_json::to_value(&o).expect("JSON");
    assert_eq!(v["central"], "earth");
    v.as_object_mut().expect("object").remove("central");
    let back: OrbitSpec = serde_json::from_value(v).expect("central なし");
    assert_eq!(back.central, CentralBody::Earth);
}

#[test]
fn altitude_accessors_use_the_central_radius() {
    let o = lunar(90.0, Propagator::Kepler);
    assert!((o.altitude_m() - 100e3).abs() < 1e-6);
    assert_eq!(
        semi_major_axis_from_altitude(CentralBody::Earth, 550e3),
        crate::constants::R_EARTH + 550e3
    );
    assert!((altitude_from_semi_major_axis(CentralBody::Moon, R_MOON + 5.0) - 5.0).abs() < 1e-9);
    assert!(
        (mean_motion_rad_s(MU_EARTH, 7e6) * o.period_s() / o.period_s()
            - (MU_EARTH / 7e6_f64.powi(3)).sqrt())
        .abs()
            < 1e-15
    );
}

#[test]
fn environment_models_are_zero_away_from_earth() {
    use crate::atmosphere::{AtmosphereModel, density_about_kg_m3};
    use crate::geomag::field_about_eci_t;
    let r = Vec3::new(R_MOON + 100e3, 0.0, 0.0);
    assert_eq!(field_about_eci_t(CentralBody::Moon, r, epoch()), Vec3::ZERO);
    assert_eq!(
        density_about_kg_m3(CentralBody::Moon, &AtmosphereModel::Exponential, r, epoch()),
        0.0
    );
    let leo = Vec3::new(crate::constants::R_EARTH + 400e3, 0.0, 0.0);
    assert!(field_about_eci_t(CentralBody::Earth, leo, epoch()).norm() > 1e-5);
    assert!(
        density_about_kg_m3(
            CentralBody::Earth,
            &AtmosphereModel::Exponential,
            leo,
            epoch()
        ) > 1e-13
    );
}
