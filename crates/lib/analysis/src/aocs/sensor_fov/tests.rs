use super::*;
use crate::power::testkit::{leo_timeline, nadir_pointing_z, sun_pointing_z};
use sat_astro::constants::R_EARTH;
use sat_astro::timeline::time_mean;
use sat_units::typed::Fraction;

#[test]
fn earth_limb_angle_by_hand() {
    // 高度 500 km: 地球の見込み半角 = asin(6378.137 / 6878.137) = 68.0°
    // 視線を天頂（地心と逆）に向けると、縁までは 180° − 68.0° = 112.0°
    let r = R_EARTH + 500e3;
    let g = Geometry {
        sun_body: Vec3::X,
        sun_visible: true,
        nadir_body: -Vec3::Z,
        earth_radius_rad: (R_EARTH / r).asin(),
        moon_body: Vec3::Y,
        moon_visible: false,
    };
    let (a, _) = g.angle(SensorFovTarget::EarthLimb, Vec3::Z);
    let expect = std::f64::consts::PI - (R_EARTH / r).asin();
    assert!((a - expect).abs() < 1e-12);
    assert!((a.to_degrees() - 111.97).abs() < 0.05);
    let (_, visible) = g.angle(SensorFovTarget::Moon, Vec3::Y);
    assert!(!visible);
}

#[test]
fn verdict_rules() {
    let sensor = SensorFovSensor {
        id: "STT".to_owned(),
        boresight: Vec3::Z,
        fov_half_angle: None,
        exclusion_sun: Some(Qty::from_raw(0.5)),
        exclusion_earth: None,
        exclusion_moon: None,
        max_intrusion_fraction: Some(Fraction::new(0.1)),
    };
    let mut r = SensorFovResult {
        id: "STT".to_owned(),
        fov_half_angle: None,
        targets: Vec::new(),
        intrusion_fraction: Fraction::new(0.05),
        intrusions: Vec::new(),
    };
    assert_eq!(verdict(&sensor, &r).verdict, Verdict::Ok);
    r.intrusion_fraction = Fraction::new(0.2);
    assert_eq!(verdict(&sensor, &r).verdict, Verdict::Violation);
}

fn sensor(id: &str, boresight: Vec3) -> SensorFovSensor {
    SensorFovSensor {
        id: id.to_owned(),
        boresight,
        fov_half_angle: Some(Qty::from_raw(10f64.to_radians())),
        exclusion_sun: Some(Qty::from_raw(30f64.to_radians())),
        exclusion_earth: Some(Qty::from_raw(20f64.to_radians())),
        exclusion_moon: None,
        max_intrusion_fraction: None,
    }
}

#[test]
fn sun_pointing_sensor_sees_sun_only_when_lit() {
    // +Z を太陽指向: +Z のセンサは日照中ずっと太陽が視線上（角 0）→ 侵入の割合 = 日照の割合
    let timeline = leo_timeline(sun_pointing_z(), 1.0, 10.0);
    let samples = timeline::build(&timeline).unwrap();
    // 日照の割合（台形則の時間平均）
    let t: Vec<f64> = samples.iter().map(|s| s.t_s).collect();
    let flags: Vec<f64> = samples
        .iter()
        .map(|s| f64::from(u8::from(is_sunlit(s))))
        .collect();
    let lit = time_mean(&t, &flags);
    let input = SensorFovInput {
        timeline,
        sensors: vec![sensor("PZ", Vec3::Z), sensor("MZ", -Vec3::Z)],
    };
    let out = SensorFov::run(&input).unwrap();
    let sun = |i: usize| {
        out.sensors[i]
            .targets
            .iter()
            .find(|t| t.target == SensorFovTarget::Sun)
            .unwrap()
            .clone()
    };
    assert!((sun(0).intrusion_fraction.value() - lit).abs() < 1e-12);
    assert!(sun(0).min_angle.value() < 1e-6);
    assert_eq!(sun(1).intrusion_fraction.value(), 0.0);
    assert_eq!(out.series.channels.len(), 6);
    assert_eq!(out.checks[0].verdict, Verdict::Warning);
}

#[test]
fn nadir_pointing_sensor_sees_earth() {
    // +Z を地心指向: +Z の視線は地球の円盤の中（縁までの角 = −asin(R/r)）、−Z は天頂（π − asin(R/r)）
    let input = SensorFovInput {
        timeline: leo_timeline(nadir_pointing_z(), 0.5, 30.0),
        sensors: vec![sensor("DOWN", Vec3::Z), sensor("UP", -Vec3::Z)],
    };
    let out = SensorFov::run(&input).unwrap();
    let earth = |i: usize| {
        out.sensors[i]
            .targets
            .iter()
            .find(|t| t.target == SensorFovTarget::EarthLimb)
            .unwrap()
            .clone()
    };
    assert_eq!(earth(0).intrusion_fraction.value(), 1.0);
    assert_eq!(earth(1).intrusion_fraction.value(), 0.0);
    let rho = (R_EARTH / input.timeline.orbit().elements.semi_major_axis.value()).asin();
    assert!((earth(0).min_angle.value() + rho).abs() < 1e-3);
    assert_eq!(earth(0).windows.len(), 1);
}
