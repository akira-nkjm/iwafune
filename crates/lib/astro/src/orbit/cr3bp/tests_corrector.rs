//! 周期軌道の補正器のテスト（面内のリアプノフ軌道は線形の極限が導ける）。

use super::tests_dynamics::check_monodromy;
use super::*;
use std::f64::consts::TAU;

fn earth_moon() -> Cr3bp {
    Cr3bp::from_bodies(CentralBody::Earth, Secondary::Body(CentralBody::Moon)).expect("mu")
}

/// 面内のリアプノフ軌道（Az = 0）の Richardson 近似を、x0 を固定して補正する。
fn lyapunov(cr: &Cr3bp, point: LibrationPoint, ax: f64) -> (PeriodicOrbit, RichardsonCoefficients) {
    let c = RichardsonCoefficients::new(cr, point).expect("coefficients");
    let mut state = c.local_state(ax, 0.0, 1.0, 0.0);
    state.iter_mut().for_each(|v| *v *= c.gamma);
    state[0] += cr.libration_point(point).expect("point")[0];
    let nu = 1.0 + c.s1 * ax * ax;
    let cfg = CorrectorConfig {
        fixed: Fixed::X0,
        ..CorrectorConfig::default()
    };
    let orbit = cr
        .correct_periodic(
            &PeriodicGuess {
                state,
                period: TAU / (c.lambda * nu),
            },
            &cfg,
        )
        .expect("converges");
    (orbit, c)
}

/// 振幅 → 0 で周期は線形の振動数 2π/ω_p（ヘッセ行列から導く）に近づく。周波数のずれは Richardson の s1·ax²
/// （ν = 1 + s1·ax²）と一致するはずで、残りは次の次数 O(ax⁴)。ax ≤ 0.04 で許容は相対 3%（打ち切りの ax² ~ 1.6e-3 × 係数 ≲ 10）。
#[test]
fn lyapunov_period_tends_to_the_linear_period_with_the_predicted_shift() {
    for (cr, point) in [
        (earth_moon(), LibrationPoint::L1),
        (earth_moon(), LibrationPoint::L2),
    ] {
        let modes = cr.linear_modes(point).expect("modes");
        let t_lin = TAU / modes.omega_planar;
        let mut last = f64::INFINITY;
        for ax in [0.04, 0.02, 0.01] {
            let (orbit, c) = lyapunov(&cr, point, ax);
            let shift = t_lin / orbit.period - 1.0;
            let predicted = c.s1 * ax * ax;
            assert!(
                ((shift - predicted) / predicted).abs() < 0.03,
                "{point:?} ax={ax}: shift {shift:e} predicted {predicted:e}"
            );
            assert!(shift.abs() < last);
            last = shift.abs();
        }
        assert!(last < 1e-3);
    }
}

#[test]
fn lyapunov_orbit_is_planar_and_conserves_jacobi() {
    let cr = earth_moon();
    let (orbit, _) = lyapunov(&cr, LibrationPoint::L1, 0.02);
    assert!(orbit.state[2].abs() < 1e-15 && orbit.state[5].abs() < 1e-15);
    let times: Vec<f64> = (1..=10).map(|i| orbit.period * i as f64 / 10.0).collect();
    let cfg = CorrectorConfig::default();
    for s in cr.flow(orbit.state, &times, &cfg.integrator).expect("flow") {
        assert!((cr.jacobi(&s) - orbit.jacobi).abs() < 1e-9);
        assert!(s[2].abs() < 1e-10);
    }
}

/// リアプノフ軌道のモノドロミー行列（検査の内容は `check_monodromy`）。
#[test]
fn lyapunov_monodromy_is_symplectic_with_a_unit_pair() {
    let cr = earth_moon();
    let (orbit, _) = lyapunov(&cr, LibrationPoint::L1, 0.02);
    check_monodromy(&cr, &orbit);
}

#[test]
fn failure_is_a_typed_error() {
    let cr = earth_moon();
    let c = RichardsonCoefficients::new(&cr, LibrationPoint::L1).expect("coefficients");
    let mut state = c.local_state(0.05, 0.0, 1.0, 0.0);
    state.iter_mut().for_each(|v| *v *= c.gamma);
    state[0] += cr.libration_point(LibrationPoint::L1).expect("point")[0];
    // 反復の上限 0: 初期値の残差が許容を超えていれば収束していない
    let cfg = CorrectorConfig {
        max_iterations: 0,
        fixed: Fixed::X0,
        ..CorrectorConfig::default()
    };
    let guess = PeriodicGuess { state, period: 3.0 };
    assert!(matches!(
        cr.correct_periodic(&guess, &cfg),
        Err(Cr3bpError::NotConverged { iterations: 0, .. })
    ));
    // vy0 = 0 は出発の向きが決まらない
    let l1 = cr.libration_point(LibrationPoint::L1).expect("point");
    let rest = PeriodicGuess {
        state: [l1[0], 0.0, 0.0, 0.0, 0.0, 0.0],
        period: 3.0,
    };
    assert!(matches!(
        cr.correct_periodic(&rest, &CorrectorConfig::default()),
        Err(Cr3bpError::InvalidArgument(_))
    ));
}

#[test]
fn fixing_x0_also_converges_for_a_halo_and_keeps_x0() {
    let cr = Cr3bp::from_bodies(CentralBody::Sun, Secondary::EarthMoonSystem).expect("mu");
    let g = cr
        .halo_guess(
            LibrationPoint::L2,
            Branch::North,
            120_000e3 / crate::constants::AU,
        )
        .expect("guess");
    let cfg = CorrectorConfig {
        fixed: Fixed::X0,
        ..CorrectorConfig::default()
    };
    let o = cr
        .correct_periodic(
            &PeriodicGuess {
                state: g.state,
                period: g.period,
            },
            &cfg,
        )
        .expect("converges");
    assert_eq!(o.state[0], g.state[0]);
    assert!(o.residual <= cfg.tolerance);
}
