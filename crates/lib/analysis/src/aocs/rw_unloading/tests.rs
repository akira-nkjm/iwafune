use super::trace::{integrate_momentum, secular_and_cyclic};
use super::*;
use crate::aocs::disturbance::DisturbanceSurface;
use crate::power::testkit::{leo_timeline, nadir_pointing_z, sun_pointing_z};
use sat_astro::constants::C_LIGHT;
use sat_astro::math::Mat3;

#[test]
fn constant_torque_integrates_linearly() {
    // 一定の 1 µN·m を 1000 s → 1 mN·m·s
    let t: Vec<f64> = (0..=100).map(|k| f64::from(k) * 10.0).collect();
    let tau = vec![Vec3::new(1e-6, 0.0, 0.0); t.len()];
    let h = integrate_momentum(&t, &tau);
    assert!((h[100].x - 1e-3).abs() < 1e-15);
}

#[test]
fn secular_and_cyclic_by_hand() {
    // H = a t + b sin(2πt/T): 1 周の増分 a T、周期成分の振幅はほぼ b
    let (a, b, period) = (1e-7, 2e-4, 100.0);
    let t: Vec<f64> = (0..=200).map(f64::from).collect();
    let h: Vec<Vec3> = t
        .iter()
        .map(|&x| {
            Vec3::new(
                a * x + b * (std::f64::consts::TAU * x / period).sin(),
                0.0,
                0.0,
            )
        })
        .collect();
    let ranges = orbit_ranges(&t, period);
    let (sec, cyc) = secular_and_cyclic(&t, &h, &ranges, period);
    assert!((sec.x - a * period).abs() < 1e-15);
    assert!((cyc - b).abs() < 1e-6);
}

#[test]
fn unload_interval_by_hand() {
    // 容量 16 mN·m·s、周期成分 2、永年 0.5 mN·m·s/周、周期 5600 s → 28 周 = 156 800 s
    let i = unload_interval(0.5e-3, 2e-3, 16e-3, 5600.0).unwrap();
    assert!((i - 28.0 * 5600.0).abs() < 1e-6);
    assert_eq!(unload_interval(0.5e-3, 20e-3, 16e-3, 5600.0), Some(0.0));
    assert_eq!(unload_interval(0.0, 2e-3, 16e-3, 5600.0), None);
}

fn wheels() -> Vec<RwWheel> {
    [("RW_X", Vec3::X), ("RW_Y", Vec3::Y), ("RW_Z", Vec3::Z)]
        .iter()
        .map(|(id, axis)| RwWheel {
            id: (*id).to_owned(),
            axis: *axis,
            max_momentum: Qty::from_raw(0.02),
            max_torque: Qty::from_raw(0.002),
        })
        .collect()
}

fn spacecraft(inertia: Mat3) -> DisturbanceSpacecraft {
    DisturbanceSpacecraft {
        inertia,
        aero: None,
        srp: None,
        residual_dipole: Vec3::ZERO,
        atmosphere: Default::default(),
        density_scale: None,
    }
}

fn input(sc: DisturbanceSpacecraft, law: sat_astro::attitude::AttitudeLaw) -> RwUnloadingInput {
    RwUnloadingInput {
        timeline: leo_timeline(law, 2.0, 10.0),
        spacecraft: sc,
        wheels: wheels(),
        usable_momentum_fraction: None,
        required_unload_interval: None,
    }
}

#[test]
fn symmetric_nadir_spacecraft_needs_no_unload() {
    // 慣性が等方的なら重力傾斜トルクは 0、ほかの擾乱も無い → 角運動量は溜まらない
    let sc = spacecraft(Mat3::diagonal(Vec3::new(0.05, 0.05, 0.05)));
    let out = RwUnloading::run(&input(sc, nadir_pointing_z())).unwrap();
    assert!(out.peak_disturbance.value() < 1e-15);
    assert_eq!(out.unload_interval, None);
    assert_eq!(out.unloads_per_day, Some(0.0));
    assert!(out.first_saturation.is_none());
}

#[test]
fn srp_on_sun_pointing_accumulates_secularly() {
    // 太陽指向で CP が +X に 5 cm、Cr·A = 0.1 m²: τ = r × (−P·CrA·ẑ) は機体系で一定（日照中のみ）。
    // 太陽方向は 1 周ではほぼ慣性固定なので、1 周の永年成分 ≈ |τ| × 日照時間。
    let mut sc = spacecraft(Mat3::diagonal(Vec3::new(0.05, 0.05, 0.05)));
    sc.srp = Some(DisturbanceSurface {
        cp_offset: Vec3::new(0.05, 0.0, 0.0),
        coefficient_area: Qty::from_raw(0.1),
    });
    let i = input(sc, sun_pointing_z());
    let out = RwUnloading::run(&i).unwrap();
    let srp = &out.disturbances[2];
    assert_eq!(srp.key, DisturbanceKind::Srp);
    // P は太陽距離で変わるので、得られたピークから 1 AU 換算の値と 2 % 以内で比べる
    let tau_1au = 1361.0 / C_LIGHT * 0.1 * 0.05;
    assert!((srp.peak.value() - tau_1au).abs() < 0.02 * tau_1au);
    let samples = timeline::build(&i.timeline).unwrap();
    let sunlit: f64 = samples.iter().map(|s| s.sunlit).sum::<f64>() / samples.len() as f64;
    let expect = srp.peak.value() * sunlit * out.period.value();
    let got = out.secular_per_orbit.value();
    assert!((got - expect).abs() < 0.02 * expect, "{got} vs {expect}");
    // 容量は機体 y 軸まわり（RW_Y 1 台）の 0.8 × 20 mN·m·s
    assert!((out.capacity.value() - 0.016).abs() < 1e-3 * 0.016);
    let interval = out.unload_interval.unwrap().value();
    let expect_interval = (0.016 - out.cyclic_amplitude.value()) / got * out.period.value();
    assert!((interval - expect_interval).abs() < 1e-6 * expect_interval);
    // トルクの余裕 = 最大トルク / 必要なトルク − 1（利用率の最大の逆数 − 1）
    let worst = out
        .wheels
        .iter()
        .map(|w| w.torque_utilization.value())
        .fold(0.0, f64::max);
    let margin = out.min_torque_margin.unwrap().value();
    assert!((margin - (1.0 / worst - 1.0)).abs() < 1e-9 * margin && margin > 1000.0);
    assert_eq!(out.usable_momentum_fraction.value(), 0.8);
    assert_eq!(out.span_fallback, None);
}

#[test]
fn nadir_gravity_gradient_with_product_of_inertia_on_pyramid() {
    // 地心指向（機体 z = 地心、y = −軌道面法線）で慣性に Ixz があると、τ = 3μ/r³ (n × I n) = 3μ/r³ (0, Ixz, 0)。
    // y は慣性空間で向きが変わらないので、1 周の永年成分 = 3μ/r³ · Ixz · T。周期成分はほぼ 0。
    let ixz = 0.01;
    let inertia = Mat3 {
        m: [[0.1, 0.0, ixz], [0.0, 0.2, 0.0], [ixz, 0.0, 0.3]],
    };
    let mut i = input(spacecraft(inertia), nadir_pointing_z());
    let b = 54.74f64.to_radians();
    let (c, s) = (b.cos(), b.sin());
    for (w, axis) in i.wheels.iter_mut().zip([
        Vec3::new(c, 0.0, s),
        Vec3::new(-c, 0.0, s),
        Vec3::new(0.0, c, s),
    ]) {
        w.axis = axis;
    }
    i.wheels.push(RwWheel {
        id: "RW_D".to_owned(),
        axis: Vec3::new(0.0, -c, s),
        max_momentum: Qty::from_raw(0.02),
        max_torque: Qty::from_raw(0.002),
    });
    let out = RwUnloading::run(&i).unwrap();
    let r = i.timeline.orbit().elements.semi_major_axis.value();
    let k = 3.0 * sat_astro::constants::MU_EARTH / r.powi(3);
    let expect = k * ixz * out.period.value();
    let got = out.secular_per_orbit.value();
    assert!((got - expect).abs() < 1e-3 * expect, "{got} vs {expect}");
    assert!(out.cyclic_amplitude.value() < 1e-3 * expect);
    // ピラミッドの y 方向の容量は 使える上限 × 2 cos β
    let cap = 0.016 * 2.0 * c;
    assert!(
        (out.capacity.value() - cap).abs() < 1e-3 * cap,
        "{} vs {cap}",
        out.capacity.value()
    );
    let interval = out.unload_interval.unwrap().value();
    assert!((interval - cap / got * out.period.value()).abs() < 1e-2 * interval);
    assert!((out.unloads_per_day.unwrap() - 86_400.0 / interval).abs() < 1e-9);
    assert_eq!(out.wheels.len(), 4);
}

#[test]
fn aero_on_nadir_pointing_accumulates_about_orbit_normal() {
    // 地心指向で CP が地心側（+z）に 2 cm: 抗力は −x（速度と逆）なので τ = r × F は機体 y（軌道面の法線）まわり。
    // y は慣性空間で向きが変わらないので、1 周の永年成分 ≈ 空力トルクの平均 × 周期。
    let mut sc = spacecraft(Mat3::diagonal(Vec3::new(0.05, 0.05, 0.05)));
    sc.aero = Some(DisturbanceSurface {
        cp_offset: Vec3::new(0.0, 0.0, 0.02),
        coefficient_area: Qty::from_raw(2.2 * 0.03),
    });
    let out = RwUnloading::run(&input(sc, nadir_pointing_z())).unwrap();
    let aero = &out.disturbances[1];
    assert_eq!(aero.key, DisturbanceKind::Aero);
    assert!(aero.peak.value() > 0.0);
    let expect = aero.mean.value() * out.period.value();
    let got = out.secular_per_orbit.value();
    assert!((got - expect).abs() < 0.02 * expect, "{got} vs {expect}");
    // 500 km の指数大気（ρ ~ 7e-13 kg/m³）で τ = ½ρ·CdA·v²·0.02 ≈ 2.6e-8 N·m 程度
    assert!(
        aero.mean.value() > 5e-9 && aero.mean.value() < 1e-7,
        "{}",
        aero.mean.value()
    );
}
