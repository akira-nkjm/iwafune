use super::*;
use crate::power::testkit::leo;
use sat_astro::constants::{B0_EARTH, G0, R_EARTH};

fn mtq(id: &str, axis: Vec3, m: f64) -> UnloadTimeMtq {
    UnloadTimeMtq {
        id: id.to_owned(),
        axis,
        max_dipole: Qty::from_raw(m),
    }
}

/// +x 向きに 10 mN、+y に 0.1 m の位置（重心は原点）→ τ = (0, 0.1, 0) × (0.01, 0, 0) = (0, 0, −1 mN·m)。
fn thruster(id: &str, position: Vec3, direction: Vec3) -> UnloadTimeThruster {
    UnloadTimeThruster {
        id: id.to_owned(),
        thrust: Qty::from_raw(0.01),
        isp: Qty::from_raw(60.0),
        position,
        direction,
    }
}

fn input() -> UnloadTimeInput {
    UnloadTimeInput {
        momentum: Some(Qty::from_raw(0.01)),
        wheels: Vec::new(),
        usable_momentum_fraction: None,
        momentum_direction: Some(Vec3::Z),
        magnetorquers: vec![
            mtq("X", Vec3::X, 0.2),
            mtq("Y", Vec3::Y, 0.2),
            mtq("Z", Vec3::Z, 0.3),
        ],
        field: Some(UnloadTimeField::Constant {
            strength: Qty::from_raw(30e-6),
        }),
        mtq_efficiency: None,
        thrusters: vec![thruster("T1", Vec3::new(0.0, 0.1, 0.0), Vec3::X)],
        center_of_mass: Some(Vec3::ZERO),
        required_time: Some(Qty::from_raw(3600.0)),
    }
}

#[test]
fn mtq_time_by_hand() {
    // m_eff = 0.2（最も弱い軸）、τ = 0.5 × 0.2 × 30 µT = 3 µN·m → 0.01 / 3e-6 = 3333.3 s
    let out = UnloadTime::run(&input()).unwrap();
    assert_eq!(out.usable_momentum_fraction, None);
    let m = out.mtq.unwrap();
    assert!((m.effective_dipole.value() - 0.2).abs() < 1e-12);
    assert!((m.torque.value() - 3e-6).abs() < 1e-18);
    assert!((m.time.value() - 0.01 / 3e-6).abs() < 1e-6);
    assert_eq!(out.checks[0].verdict, Verdict::Ok);
}

#[test]
fn rcs_time_and_propellant_by_hand() {
    // ĥ = +z を捨てるには −z のトルク: T1 は (0, 0, −1 mN·m) → 0.01 / 1e-3 = 10 s、
    // 推薬 = 0.01 N × 10 s / (60 s × 9.80665) = 1.6995e-4 kg
    let out = UnloadTime::run(&input()).unwrap();
    let r = out.rcs.unwrap().worst;
    assert!((r.torque.value() - 1e-3).abs() < 1e-15);
    assert!((r.time.unwrap().value() - 10.0).abs() < 1e-9);
    assert!((r.propellant.unwrap().value() - 0.1 / (60.0 * G0)).abs() < 1e-12);
    assert!(r.off_axis_torque.value() < 1e-15);
    assert!((r.net_force.value() - 0.01).abs() < 1e-15);
    assert_eq!(r.thrusters, vec!["T1".to_owned()]);
    assert!(out.checks.iter().all(|c| c.verdict == Verdict::Ok));
}

#[test]
fn rcs_uses_the_center_of_mass() {
    // 重心を +y に 5 cm ずらすと腕は 5 cm → 0.5 mN·m → 20 s
    let mut i = input();
    i.center_of_mass = Some(Vec3::new(0.0, 0.05, 0.0));
    let r = UnloadTime::run(&i).unwrap().rcs.unwrap().worst;
    assert!((r.time.unwrap().value() - 20.0).abs() < 1e-9);
}

#[test]
fn rcs_picks_thrusters_with_a_useful_component() {
    // 偶力の 2 本（−z のトルク）と、逆向きのトルクを出す 1 本。逆向きのものは噴かない
    let mut i = input();
    i.thrusters = vec![
        thruster("A", Vec3::new(0.0, 0.1, 0.0), Vec3::X),
        thruster("B", Vec3::new(0.0, -0.1, 0.0), -Vec3::X),
        thruster("C", Vec3::new(0.0, -0.1, 0.0), Vec3::X),
    ];
    let r = UnloadTime::run(&i).unwrap().rcs.unwrap().worst;
    assert_eq!(r.thrusters, vec!["A".to_owned(), "B".to_owned()]);
    assert!((r.torque.value() - 2e-3).abs() < 1e-15);
    // 偶力なので正味の推力は 0
    assert!(r.net_force.value() < 1e-15);
    assert!((r.time.unwrap().value() - 5.0).abs() < 1e-9);
}

#[test]
fn rcs_without_direction_reports_the_worst_axis() {
    // 向きを与えなければ ±x・±y・±z を調べる。T1 だけでは −z のトルクしか出せないので、最悪は「捨てられない」
    let mut i = input();
    i.momentum_direction = None;
    let out = UnloadTime::run(&i).unwrap();
    let rcs = out.rcs.unwrap();
    assert_eq!(rcs.directions.len(), 6);
    assert!(rcs.worst.time.is_none());
    let pz = rcs.directions.iter().find(|d| d.label == "+z").unwrap();
    assert!((pz.time.unwrap().value() - 10.0).abs() < 1e-9);
    let v = |k: &str| out.checks.iter().find(|c| c.key == k).unwrap().verdict;
    assert_eq!(v("rcs_direction"), Verdict::Violation);
    assert_eq!(v("rcs_time"), Verdict::Violation);
}

#[test]
fn rcs_requires_center_of_mass() {
    let mut i = input();
    i.center_of_mass = None;
    assert!(UnloadTime::run(&i).is_err());
}

#[test]
fn skewed_mtq_effective_dipole() {
    // 45° に傾けた 2 台 + z: x 軸方向は 2 × 0.2 × cos 45°
    let m = [
        mtq("A", Vec3::new(1.0, 1.0, 0.0), 0.2),
        mtq("B", Vec3::new(1.0, -1.0, 0.0), 0.2),
        mtq("Z", Vec3::Z, 1.0),
    ];
    let e = effective_dipole(&m).unwrap();
    assert!((e - 0.4 * std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-12);
}

#[test]
fn orbit_averaged_field_is_within_dipole_bounds() {
    // 双極子: 赤道 B0 (R/r)³、磁極 2 B0 (R/r)³。極軌道の平均はその間に入る
    let mut i = input();
    let orbit = leo();
    let k = (R_EARTH / orbit.elements.semi_major_axis.value()).powi(3) * B0_EARTH;
    let period = orbit.period_s();
    i.field = Some(UnloadTimeField::Orbit {
        sampling: OrbitSampling::new(orbit, Qty::from_raw(period), Qty::from_raw(30.0)),
    });
    let m = UnloadTime::run(&i).unwrap().mtq.unwrap();
    assert_eq!(m.efficiency, DEFAULT_MTQ_EFFICIENCY);
    assert_eq!(m.assumption, UnloadAssumption::MtqMeanField);
    assert!(
        m.min_field.value() > 0.95 * k && m.max_field.value() < 2.05 * k,
        "{m:?}"
    );
    assert!(m.mean_field > m.min_field && m.mean_field < m.max_field);
    assert!((m.time.value() - 0.01 / (0.5 * 0.2 * m.mean_field.value())).abs() < 1e-6);
    assert!(m.orbits.unwrap() > 0.0);
}

#[test]
fn mtq_without_field_is_an_error() {
    let mut i = input();
    i.field = None;
    assert!(UnloadTime::run(&i).is_err());
}

#[test]
fn momentum_from_wheels_by_hand() {
    // momentum を省略: 最大のホイール 20 mN·m·s × 0.8 = 16 mN·m·s を捨てる → MTQ 3 µN·m で 5333 s
    let mut i = input();
    i.momentum = None;
    i.wheels = [0.02, 0.012]
        .iter()
        .enumerate()
        .map(|(k, h)| RwWheel {
            id: format!("RW{k}"),
            axis: Vec3::X,
            max_momentum: Qty::from_raw(*h),
            max_torque: Qty::from_raw(0.002),
        })
        .collect();
    let out = UnloadTime::run(&i).unwrap();
    assert!((out.momentum.value() - 0.016).abs() < 1e-15);
    // 使った割合を返す（与えた momentum なら None）
    assert_eq!(out.usable_momentum_fraction.map(Fraction::value), Some(0.8));
    assert!((out.mtq.unwrap().time.value() - 0.016 / 3e-6).abs() < 1e-6);
    i.wheels.clear();
    assert!(UnloadTime::run(&i).is_err());
}
