use super::trace::{integrate_momentum, secular_and_cyclic};
use super::*;
use crate::aocs::disturbance::{DisturbanceSurface, FiringThruster, ThrustDisturbance};
use crate::aocs::thrusters::Thruster;
use crate::power::testkit::{leo_timeline, nadir_pointing_z, sun_pointing_z};
use crate::spec::{ChannelKind, Verdict};
use sat_astro::constants::C_LIGHT;
use sat_astro::constants::G0;
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
    let ranges = orbit_ranges(&t, sat_astro::orbit::Cycle::Periodic { period_s: period });
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
        geometry: None,
        residual_dipole: None,
        atmosphere: Default::default(),
        density_scale: None,
        thrust: None,
    }
}

/// 係数・面積・圧力中心をすべて書いた面。
fn written(coefficient: f64, area: f64, cp: Vec3) -> DisturbanceSurface {
    DisturbanceSurface {
        coefficient,
        area: Some(Qty::from_raw(area)),
        cp_offset: Some(cp),
    }
}

fn input(sc: DisturbanceSpacecraft, law: sat_astro::attitude::AttitudeLaw) -> RwUnloadingInput {
    RwUnloadingInput {
        timeline: leo_timeline(law, 2.0, 10.0),
        spacecraft: sc,
        wheels: wheels(),
        usable_momentum_fraction: None,
        required_unload_interval: None,
        rcs: None,
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
    sc.srp = Some(written(1.0, 0.1, Vec3::new(0.05, 0.0, 0.0)));
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
    // ナディア指向（機体 z = 地心、y = −軌道面法線）で慣性に Ixz があると、τ = 3μ/r³ (n × I n) = 3μ/r³ (0, Ixz, 0)。
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
    let r = i
        .timeline
        .orbit()
        .elements()
        .expect("elements")
        .elements
        .semi_major_axis
        .value();
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
    // ナディア指向で CP が地心側（+z）に 2 cm: 抗力は −x（速度と逆）なので τ = r × F は機体 y（軌道面の法線）まわり。
    // y は慣性空間で向きが変わらないので、1 周の永年成分 ≈ 空力トルクの平均 × 周期。
    let mut sc = spacecraft(Mat3::diagonal(Vec3::new(0.05, 0.05, 0.05)));
    sc.aero = Some(written(2.2, 0.03, Vec3::new(0.0, 0.0, 0.02)));
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

fn thruster(id: &str, position: Vec3, direction: Vec3) -> Thruster {
    Thruster {
        id: id.to_owned(),
        thrust: Qty::from_raw(0.01),
        isp: Qty::from_raw(60.0),
        position,
        direction,
    }
}

/// ナディア指向で、+z に噴く 10 mN のスラスタの推力の線が重心から x に 2 mm ずれ、時間の 1 % 噴射する。
fn thrust_misaligned() -> ThrustDisturbance {
    ThrustDisturbance {
        center_of_mass: Vec3::new(-0.002, 0.0, 0.0),
        firing: vec![FiringThruster {
            thruster: thruster("T", Vec3::new(0.0, 0.0, -0.1), Vec3::Z),
            firing_fraction: Fraction::new(0.01),
        }],
    }
}

#[test]
fn thrust_misalignment_accumulates_about_orbit_normal() {
    // τ = 0.01 × (0.002, 0, 0) × (0, 0, 0.01) = (0, −2e-7, 0) N·m（機体 y = 軌道面の法線まわり、慣性空間で向きが変わらない）。
    // 1 周の永年成分 = 2e-7 × 周期。ほかの擾乱は 0（等方的な慣性・面なし）
    let mut sc = spacecraft(Mat3::diagonal(Vec3::new(0.05, 0.05, 0.05)));
    sc.thrust = Some(thrust_misaligned());
    let out = RwUnloading::run(&input(sc, nadir_pointing_z())).unwrap();
    let thrust = out.disturbances.last().unwrap();
    assert_eq!(thrust.key, DisturbanceKind::Thrust);
    assert!((thrust.peak.value() - 2e-7).abs() < 1e-18);
    let expect = 2e-7 * out.period.value();
    let got = out.secular_per_orbit.value();
    assert!((got - expect).abs() < 1e-6 * expect, "{got} vs {expect}");
    assert!(
        out.series
            .channels
            .iter()
            .any(|c| c.kind == ChannelKind::ThrustTorque)
    );
}

#[test]
fn thrust_kind_is_listed_only_with_thrusters() {
    let sc = spacecraft(Mat3::diagonal(Vec3::new(0.05, 0.05, 0.05)));
    let out = RwUnloading::run(&input(sc, nadir_pointing_z())).unwrap();
    assert_eq!(out.disturbances.len(), 4);
    assert!(
        !out.series
            .channels
            .iter()
            .any(|c| c.kind == ChannelKind::ThrustTorque)
    );
    assert!(out.rcs_unloading.is_none());
    let json = serde_json::to_value(&out).unwrap();
    assert!(json.get("rcs_unloading").is_none());
}

/// 機体の各軸 ±x・±y・±z のトルクを 1 本ずつが出す 6 本（腕 0.1 m、10 mN → 1 mN·m）。
fn six_axis_rcs() -> Vec<Thruster> {
    [
        ("XP", Vec3::Y, Vec3::Z),
        ("XM", Vec3::Y, -Vec3::Z),
        ("YP", Vec3::Z, Vec3::X),
        ("YM", Vec3::Z, -Vec3::X),
        ("ZP", Vec3::X, Vec3::Y),
        ("ZM", Vec3::X, -Vec3::Y),
    ]
    .iter()
    .map(|(id, p, d)| thruster(id, *p * 0.1, *d))
    .collect()
}

#[test]
fn rcs_propellant_and_delta_v_per_day_by_hand() {
    // 推力誤差だけが溜まる機体（上のテストと同じ）。1 日の角運動量 H = 2e-7 N·m × 1 日 = 0.01728 N·m·s。
    // どの向きも 1 本（1 mN·m）で捨てる → 噴射 H / 1e-3 s、推薬 H / (0.1 m × 60 s × g₀)、
    // ΔV 換算 = 60 g₀ ln(10 / (10 − 推薬))
    let mut sc = spacecraft(Mat3::diagonal(Vec3::new(0.05, 0.05, 0.05)));
    sc.thrust = Some(thrust_misaligned());
    let mut i = input(sc, nadir_pointing_z());
    i.rcs = Some(RwUnloadingRcsInput {
        thrusters: six_axis_rcs(),
        center_of_mass: Vec3::ZERO,
        mass: Qty::from_raw(10.0),
    });
    let out = RwUnloading::run(&i).unwrap();
    let r = out.rcs_unloading.unwrap();
    let h = out.secular_per_orbit.value() * 86_400.0 / out.period.value();
    assert!((h - 2e-7 * 86_400.0).abs() < 1e-6 * h);
    assert!((r.momentum_per_day.value() - h).abs() < 1e-15);
    assert!((r.per_day.time.unwrap().value() - h / 1e-3).abs() < 1e-9);
    let mp = h / (0.1 * 60.0 * G0);
    assert!((r.per_day.propellant.unwrap().value() - mp).abs() < 1e-12);
    let dv = 60.0 * G0 * (10.0 / (10.0 - mp)).ln();
    assert!((r.delta_v_per_day.unwrap().value() - dv).abs() < 1e-12);
    // 小さい推薬なら ΔV ≈ v_e · m_p / m₀ = H / (0.1 m × 10 kg)
    assert!((dv - h / (0.1 * 10.0)).abs() < 1e-3 * dv);
    assert!(out.checks.iter().all(|c| c.key != "rcs_direction"));
}

#[test]
fn rcs_without_a_direction_is_a_violation() {
    let sc = spacecraft(Mat3::diagonal(Vec3::new(0.05, 0.05, 0.05)));
    let mut i = input(sc, nadir_pointing_z());
    i.rcs = Some(RwUnloadingRcsInput {
        thrusters: six_axis_rcs()[..1].to_vec(),
        center_of_mass: Vec3::ZERO,
        mass: Qty::from_raw(10.0),
    });
    let out = RwUnloading::run(&i).unwrap();
    let r = out.rcs_unloading.unwrap();
    assert!(r.per_day.time.is_none() && r.delta_v_per_day.is_none());
    let check = out
        .checks
        .iter()
        .find(|c| c.key == "rcs_direction")
        .unwrap();
    assert_eq!(check.verdict, crate::spec::Verdict::Violation);
}

/// 中心天体 `body` の高度 100 km の極軌道に直した入力。空力の面と残留磁気を与えて、大気・磁場の扱いを見る。
fn about_body(body: sat_astro::ephem::CentralBody) -> RwUnloadingInput {
    let mut sc = spacecraft(Mat3::diagonal(Vec3::new(0.05, 0.05, 0.05)));
    sc.aero = Some(written(1.0, 0.1, Vec3::new(0.05, 0.0, 0.0)));
    sc.residual_dipole = Some(Vec3::new(0.1, 0.0, 0.0));
    let mut i = input(sc, nadir_pointing_z());
    let epoch = i.timeline.orbit().epoch();
    let orbit = sat_astro::orbit::OrbitSpec::circular_about(
        body,
        epoch,
        Qty::from_raw(100e3),
        Qty::from_raw(90f64.to_radians()),
        Qty::from_raw(0.0),
    );
    i.timeline = TimelineSpec::new(
        orbit.clone(),
        nadir_pointing_z(),
        Qty::from_raw(orbit.period_s() * 2.0),
        Qty::from_raw(30.0),
    );
    i
}

/// 月: 空力・磁気トルクは 0 で、それが正しい理由（大気・磁場が無い）を `Ok` の判定で伝える。
/// 地球では判定を足さない（デモの出力を変えない）。判定の文言の場合分けは `disturbance/environment.rs` のテスト。
#[test]
fn moon_states_zero_aero_and_magnetic_torque_in_the_checks() {
    let moon = RwUnloading::run(&about_body(sat_astro::ephem::CentralBody::Moon)).unwrap();
    let key = |k| moon.disturbances.iter().find(|d| d.key == k).unwrap();
    assert_eq!(key(DisturbanceKind::Aero).peak.value(), 0.0);
    assert_eq!(key(DisturbanceKind::Magnetic).peak.value(), 0.0);
    let c = moon.checks.iter().find(|c| c.key == "environment_models");
    assert_eq!(c.expect("note").verdict, Verdict::Ok);
    let earth = RwUnloading::run(&about_body(sat_astro::ephem::CentralBody::Earth)).unwrap();
    assert!(earth.checks.iter().all(|c| c.key != "environment_models"));
}
