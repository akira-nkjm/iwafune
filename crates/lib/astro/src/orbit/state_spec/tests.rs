use super::*;
use crate::constants::AU;
use crate::force::GravityModel;
use crate::math::Vec3;
use crate::orbit::{OrbitSpec, Propagator};
use sat_units::typed::Qty;

fn epoch() -> Epoch {
    Epoch::parse("2030-01-01T00:00:00Z").expect("epoch")
}

/// 原点の天体の点質量だけの力（地球を第三体に自動で加えるのも外す）。
fn point_mass_only() -> ForceConfig {
    ForceConfig {
        omit_earth: true,
        ..ForceConfig::gravity_only(GravityModel::PointMass)
    }
}

/// 半径 `r` の円軌道の状態（x 軸上、y 向きに円速度）。
fn circular(central: CentralBody, r: f64) -> OrbitState {
    OrbitState {
        r_eci: Vec3::new(r, 0.0, 0.0),
        v_eci: Vec3::new(0.0, (central.gm() / r).sqrt(), 0.0),
    }
}

fn period_s(central: CentralBody, r: f64) -> f64 {
    TAU * (r.powi(3) / central.gm()).sqrt()
}

/// 状態が、原点の点質量だけの円軌道のケプラー要素の状態と同じなら、位置は 1 周期にわたって Kepler の解（手計算の
/// 二体問題）と一致する。許容は半径の 1e-8: 積分の相対許容 1e-11 × ステップ数（刻みの上限 600 s で 1 周期に ~12–20 ステップ）
/// が ~1e-10、密出力の補間の誤差と along-track への増幅（周期の誤差 × 1 周）を含めて 2 桁の余裕。
#[test]
fn circular_state_reproduces_the_kepler_positions_over_one_period() {
    let central = CentralBody::Mars;
    let a = central.radius_m() + 400e3;
    let mut kepler = OrbitSpec::circular_about(
        central,
        epoch(),
        Qty::from_raw(400e3),
        Qty::from_raw(0.0),
        Qty::from_raw(0.0),
    );
    kepler.propagator = Propagator::Kepler;
    let x0 = kepler.state_at(0.0).expect("state");
    assert!((x0.r_eci.norm() - a).abs() < 1e-6);
    let spec = StateSpec::with_defaults(
        epoch(),
        central,
        x0,
        period_s(central, a),
        Some(point_mass_only()),
    );
    let t_end = period_s(central, a);
    let times: Vec<f64> = (0..=16).map(|i| t_end * f64::from(i) / 16.0).collect();
    let numeric = spec.states_at(&times).expect("propagation");
    let exact = kepler.states_at(&times).expect("kepler");
    for ((t, n), k) in times.iter().zip(&numeric).zip(&exact) {
        let dr = (n.r_eci - k.r_eci).norm();
        assert!(dr < 1e-8 * a, "t = {t}: {dr} m");
    }
}

/// 1 au・円速度の日心の状態（太陽の点質量だけ）: 距離は 1 年のあいだ 1 au のまま、周期は `2π√(r³/GM☉)`（手計算）で、
/// それは 1 ユリウス年（365.25 日）の 2e-5 以内（太陽だけの 1 au の周期は 365.256 日）。許容は半径の 1e-8:
/// 刻みの上限 1 日で ~400 ステップ × 相対許容 1e-11 ≈ 4e-9 の積分の誤差に余裕を見たもの。
#[test]
fn heliocentric_circular_state_stays_at_one_au_and_closes_in_a_year() {
    let central = CentralBody::Sun;
    let t = period_s(central, AU);
    assert!((t / (365.25 * 86_400.0) - 1.0).abs() < 2e-5, "{t}");
    let spec = StateSpec::with_defaults(
        epoch(),
        central,
        circular(central, AU),
        t,
        Some(point_mass_only()),
    );
    let times: Vec<f64> = (0..=24).map(|i| t * f64::from(i) / 24.0).collect();
    let states = spec.states_at(&times).expect("propagation");
    for (time, s) in times.iter().zip(&states) {
        let d = (s.r_eci.norm() - AU).abs();
        assert!(d < 1e-8 * AU, "t = {time}: distance differs by {d} m");
    }
    let end = states.last().expect("end");
    assert!((end.r_eci - states[0].r_eci).norm() < 1e-8 * AU);
}

/// 表の格子の刻みは、円軌道では周期の 1/100（`TABLE_STEPS_PER_ORBIT`）。表（エルミート補間）の位置は、直接の積分と
/// 半径の 4.2e-8 以内で一致する: 円軌道の補間の誤差の上限は `(ωh)⁴/384 · r`（`h` = 周期/100 で `(2π/100)⁴/384 = 4.06e-8`）に、
/// 積分の誤差（~1e-9）を足したもの。
#[test]
fn the_table_step_is_a_hundredth_of_the_circular_period_and_interpolates_within_its_bound() {
    let central = CentralBody::Sun;
    let t = period_s(central, AU);
    let spec = StateSpec::with_defaults(
        epoch(),
        central,
        circular(central, AU),
        t,
        Some(point_mass_only()),
    );
    assert!((spec.table_step_s() / (t / 100.0) - 1.0).abs() < 1e-12);
    let traj = crate::orbit::Trajectory::State(spec.clone());
    let track = traj.track(t, f64::INFINITY).expect("track");
    let times = [0.37 * t, 0.5 * t + 12_345.0, 0.91 * t];
    let direct = spec.states_at(&times).expect("direct");
    for (time, d) in times.iter().zip(&direct) {
        let dr = (track.state_at(*time).r_eci - d.r_eci).norm();
        assert!(dr < 4.2e-8 * AU, "t = {time}: {dr} m");
    }
}

/// 近点を持つ楕円では、格子は近点での 1 周の時間（`2π q²/h`）から決まる: 離心率 0.5 の楕円は円軌道（同じ近点距離）より細かい。
#[test]
fn an_eccentric_state_has_a_finer_table_than_the_circle_at_its_periapsis() {
    let central = CentralBody::Sun;
    let q = AU;
    let mu = central.gm();
    // 近点（x 軸）に位置し、速度は近点速度 √(μ(1+e)/q)
    let e = 0.5;
    let state = OrbitState {
        r_eci: Vec3::new(q, 0.0, 0.0),
        v_eci: Vec3::new(0.0, (mu * (1.0 + e) / q).sqrt(), 0.0),
    };
    let ecc = StateSpec::with_defaults(epoch(), central, state, 1e7, None);
    let circ = StateSpec::with_defaults(epoch(), central, circular(central, q), 1e7, None);
    // 2π q²/h、h = √(μ q (1+e))
    let expected = TAU * q * q / (mu * q * (1.0 + e)).sqrt() / TABLE_STEPS_PER_ORBIT;
    assert!((ecc.table_step_s() / expected - 1.0).abs() < 1e-9);
    assert!(ecc.table_step_s() < circ.table_step_s());
}

/// 力の設定を書かなければ、原点の天体で決まる既定（`ForceConfig::deep_space_defaults`）。書けばそれを使う。
#[test]
fn forces_default_by_the_origin_and_an_explicit_setting_wins() {
    let state = circular(CentralBody::Sun, AU);
    let sun = StateSpec::with_defaults(epoch(), CentralBody::Sun, state, 1.0, None);
    assert_eq!(sun.forces, ForceConfig::heliocentric_point_masses(&[]));
    let explicit = StateSpec::with_defaults(
        epoch(),
        CentralBody::Sun,
        state,
        1.0,
        Some(point_mass_only()),
    );
    assert_eq!(explicit.forces, point_mass_only());
    assert_eq!(sun.integrator().max_step, 86_400.0);
    let earth = StateSpec::with_defaults(epoch(), CentralBody::Earth, state, 1.0, None);
    assert_eq!(earth.integrator().max_step, 600.0);
}

/// 値の検査: 期間・位置・速度・力の設定。
#[test]
fn invalid_values_are_rejected() {
    let ok = StateSpec::with_defaults(
        epoch(),
        CentralBody::Sun,
        circular(CentralBody::Sun, AU),
        1.0e6,
        None,
    );
    assert!(ok.validate().is_ok());
    for span in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let mut s = ok.clone();
        s.span_s = span;
        assert!(s.validate().is_err(), "span {span}");
    }
    let mut zero = ok.clone();
    zero.state.r_eci = Vec3::ZERO;
    assert!(zero.validate().is_err());
    let mut nan = ok.clone();
    nan.state.v_eci = Vec3::new(f64::NAN, 0.0, 0.0);
    assert!(nan.validate().is_err());
    // EGM96 は地球が原点のときだけ
    let mut egm = ok;
    egm.forces.gravity = GravityModel::Harmonics {
        degree: 4,
        order: 4,
    };
    assert!(egm.validate().is_err());
}
