use super::*;

/// 直交 3 台、各 最大 10 mN·m·s・1 mN·m。使ってよい割合 0.8 → 軸まわり h = 8 mN·m·s、τ = 1 mN·m。
fn wheels() -> Vec<RwWheel> {
    [("RW_X", Vec3::X), ("RW_Y", Vec3::Y), ("RW_Z", Vec3::Z)]
        .iter()
        .map(|(id, axis)| RwWheel {
            id: (*id).to_owned(),
            axis: *axis,
            max_momentum: Qty::from_raw(0.01),
            max_torque: Qty::from_raw(0.001),
        })
        .collect()
}

fn input(angle_deg: f64) -> SlewTimeInput {
    SlewTimeInput {
        inertia: Mat3::diagonal(Vec3::new(0.1, 0.06, 0.04)),
        wheels: wheels(),
        usable_momentum_fraction: None,
        angle: Qty::from_raw(angle_deg.to_radians()),
        axis: None,
        required_time: None,
    }
}

#[test]
fn trapezoidal_slew_by_hand() {
    // x 軸: I = 0.1、h²/(Iτ) = 64e-6 / 1e-4 = 0.64 rad < 90° → 台形。
    // t = θI/h + h/τ = (π/2)(0.1)/0.008 + 8 = 19.635 + 8 = 27.635 s、最大角速度 h/I = 0.08 rad/s（4.58°/s）
    let out = SlewTime::run(&input(90.0)).unwrap();
    let x = &out.axes[0];
    assert_eq!(
        (x.label.as_str(), x.profile),
        ("x", SlewProfile::Trapezoidal)
    );
    let expect = std::f64::consts::FRAC_PI_2 * 0.1 / 0.008 + 8.0;
    assert!((x.time.value() - expect).abs() < 1e-9, "{}", x.time.value());
    assert!((x.time.value() - 27.635).abs() < 1e-3);
    assert!((x.accel_time.value() - 8.0).abs() < 1e-12);
    assert!((x.coast_time.value() - (expect - 16.0)).abs() < 1e-9);
    assert!((x.peak_rate.value() - 0.08).abs() < 1e-12);
    assert!((x.momentum.value() - 0.008).abs() < 1e-15);
    assert!((x.torque.value() - 0.001).abs() < 1e-15);
    // 慣性の最も大きい x が最悪
    assert_eq!(out.worst.label, "x");
    assert_eq!(out.usable_momentum_fraction.value(), 0.8);
    assert!(out.checks.is_empty());
}

#[test]
fn triangular_slew_by_hand() {
    // 10° = 0.17453 rad < 0.64 rad → 三角形。t = 2√(θI/τ) = 2√(17.453) = 8.3554 s、最大角速度 √(θτ/I) = 0.041777 rad/s
    let out = SlewTime::run(&input(10.0)).unwrap();
    let x = &out.axes[0];
    assert_eq!(x.profile, SlewProfile::Triangular);
    let theta = 10f64.to_radians();
    assert!((x.time.value() - 2.0 * (theta * 0.1 / 0.001).sqrt()).abs() < 1e-12);
    assert!((x.time.value() - 8.3554).abs() < 1e-4);
    assert!((x.peak_rate.value() - (theta * 0.001 / 0.1).sqrt()).abs() < 1e-12);
    assert_eq!(x.coast_time.value(), 0.0);
}

#[test]
fn profiles_meet_at_the_threshold() {
    // θ = h²/(Iτ) = 0.64 rad では三角形と台形の時間が一致する（2h/τ = 16 s）
    let mut i = input(0.0);
    i.axis = Some(Vec3::X);
    i.angle = Qty::from_raw(0.64);
    let tri = SlewTime::run(&i).unwrap().worst;
    i.angle = Qty::from_raw(0.64 * (1.0 + 1e-12));
    let trap = SlewTime::run(&i).unwrap().worst;
    assert_eq!(tri.profile, SlewProfile::Triangular);
    assert_eq!(trap.profile, SlewProfile::Trapezoidal);
    assert!((tri.time.value() - 16.0).abs() < 1e-9 && (trap.time.value() - 16.0).abs() < 1e-9);
}

#[test]
fn given_axis_uses_projected_inertia_and_wheel_capacity() {
    // 軸 (1, 1, 0)/√2: I = (0.1 + 0.06)/2 = 0.08、各ホイールが 1/√2 ずつ持つので τ = √2 mN·m、h = 8√2 mN·m·s
    let mut i = input(180.0);
    i.axis = Some(Vec3::new(1.0, 1.0, 0.0));
    i.required_time = Some(Qty::from_raw(30.0));
    let out = SlewTime::run(&i).unwrap();
    assert_eq!(out.axes.len(), 1);
    let a = &out.worst;
    assert!((a.moment_of_inertia.value() - 0.08).abs() < 1e-15);
    let (tau, h) = (2f64.sqrt() * 1e-3, 2f64.sqrt() * 8e-3);
    assert!((a.torque.value() - tau).abs() < 1e-15);
    let expect = std::f64::consts::PI * 0.08 / h + h / tau;
    assert!((a.time.value() - expect).abs() < 1e-9);
    // 22.2 + 8 = 30.2 s > 30 s → 違反
    assert_eq!(out.checks[0].verdict, Verdict::Violation);
}

#[test]
fn angles_out_of_range_are_rejected() {
    assert!(SlewTime::run(&input(0.0)).is_err());
    assert!(SlewTime::run(&input(181.0)).is_err());
    assert!(SlewTime::run(&input(180.0)).is_ok());
}
