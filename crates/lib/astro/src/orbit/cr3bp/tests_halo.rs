//! ハロー軌道（Richardson の近似 + 補正）のテスト。

use super::tests_dynamics::check_monodromy;
use super::*;
use crate::constants::AU;

fn sun_earth_moon() -> Cr3bp {
    Cr3bp::from_bodies(CentralBody::Sun, Secondary::EarthMoonSystem).expect("mu")
}

fn earth_moon() -> Cr3bp {
    Cr3bp::from_bodies(CentralBody::Earth, Secondary::Body(CentralBody::Moon)).expect("mu")
}

/// 太陽–地球 L2 の z 振幅 120 000 km（近似の z の最大値）。
fn se_l2(cr: &Cr3bp, branch: Branch) -> HaloOrbit {
    cr.halo_orbit(
        LibrationPoint::L2,
        branch,
        120_000e3 / AU,
        &CorrectorConfig::default(),
    )
    .expect("halo")
}

#[test]
fn richardson_coefficients_agree_with_the_linearisation() {
    for (cr, p) in [
        (sun_earth_moon(), LibrationPoint::L1),
        (sun_earth_moon(), LibrationPoint::L2),
        (earth_moon(), LibrationPoint::L1),
        (earth_moon(), LibrationPoint::L2),
    ] {
        let c = RichardsonCoefficients::new(&cr, p).expect("coefficients");
        let m = cr.linear_modes(p).expect("modes");
        // c2 = ヘッセ行列から導いた c（γ 単位に直しても同じ無次元量）、λ = ω_p
        assert!((c.c2 - m.c).abs() < 1e-9 * m.c, "{p:?} c2 {} {}", c.c2, m.c);
        assert!((c.lambda - m.omega_planar).abs() < 1e-12);
        assert!((c.delta - (m.omega_planar.powi(2) - m.c)).abs() < 1e-9);
    }
}

#[test]
fn halo_guess_exists_only_at_l1_and_l2() {
    let cr = sun_earth_moon();
    assert!(matches!(
        cr.halo_guess(LibrationPoint::L3, Branch::North, 1e-4),
        Err(Cr3bpError::HaloPointUnsupported(_))
    ));
}

/// 補正後の周期性: 1 周期後の状態と元の状態の差（最大成分）。許容 1e-8:
/// 収束判定 1e-11（半周期の vx, vz）が、半周期の写像の増幅（≲ 1e3、モノドロミー行列の固有値の大きさ）で拡がる分。
#[test]
fn corrected_sun_earth_halo_is_periodic_and_conserves_jacobi() {
    let cr = sun_earth_moon();
    let h = se_l2(&cr, Branch::North);
    let cfg = CorrectorConfig::default();
    assert!(h.orbit.residual <= cfg.tolerance);
    let times: Vec<f64> = (1..=8).map(|i| h.orbit.period * i as f64 / 8.0).collect();
    let flow = cr
        .flow(h.orbit.state, &times, &cfg.integrator)
        .expect("flow");
    let end = flow[7];
    let diff = (0..6)
        .map(|i| (end[i] - h.orbit.state[i]).abs())
        .fold(0.0, f64::max);
    assert!(diff < 1e-8, "periodicity {diff:e}");
    for s in &flow {
        assert!((cr.jacobi(s) - h.orbit.jacobi).abs() < 1e-9);
    }
}

/// 近似は補正の初期値として妥当: 位置の差は軌道の x 方向の大きさ γ·ax の 10%、周期の差は 1% 以内
/// （3 次の近似の誤差 O(振幅⁴) に対して先に決めた許容）。
#[test]
fn guess_is_close_to_the_corrected_orbit() {
    for (cr, p, amp) in [
        (sun_earth_moon(), LibrationPoint::L2, 120_000e3 / AU),
        (sun_earth_moon(), LibrationPoint::L1, 120_000e3 / AU),
        (earth_moon(), LibrationPoint::L1, 8_000e3 / 3.844e8),
        (earth_moon(), LibrationPoint::L2, 8_000e3 / 3.844e8),
    ] {
        let h = cr
            .halo_orbit(p, Branch::North, amp, &CorrectorConfig::default())
            .expect("halo");
        let gamma = cr.gamma(p).expect("gamma");
        let size = gamma * h.guess.ax;
        assert!(h.state_difference()[0].abs() < 0.1 * size, "{p:?} x");
        assert!(
            h.period_difference().abs() < 0.01 * h.orbit.period,
            "{p:?} T"
        );
        // z0 は固定している
        assert_eq!(h.orbit.state[2], h.guess.state[2]);
    }
}

#[test]
fn halo_monodromy_is_symplectic_with_a_unit_pair() {
    let cr = sun_earth_moon();
    let h = se_l2(&cr, Branch::North);
    check_monodromy(&cr, &h.orbit);
}

/// 北と南は z → −z の鏡像。初期値は丸めまで一致し、流れも鏡像のまま。
#[test]
fn south_branch_is_the_mirror_image_of_the_north_branch() {
    let cr = sun_earth_moon();
    let (n, s) = (se_l2(&cr, Branch::North), se_l2(&cr, Branch::South));
    let mirror = [1.0, 1.0, -1.0, 1.0, 1.0, -1.0];
    for i in 0..6 {
        assert!(
            (s.orbit.state[i] - mirror[i] * n.orbit.state[i]).abs() < 1e-12,
            "{i}"
        );
    }
    assert!((s.orbit.period - n.orbit.period).abs() < 1e-10);
    let cfg = CorrectorConfig::default();
    let t = [0.37 * n.orbit.period];
    let fn_ = cr.flow(n.orbit.state, &t, &cfg.integrator).expect("flow")[0];
    let fs = cr.flow(s.orbit.state, &t, &cfg.integrator).expect("flow")[0];
    for i in 0..6 {
        assert!((fs[i] - mirror[i] * fn_[i]).abs() < 1e-9, "{i}");
    }
}

#[test]
fn earth_moon_halos_converge_for_both_points() {
    let cr = earth_moon();
    for p in [LibrationPoint::L1, LibrationPoint::L2] {
        let h = cr
            .halo_orbit(
                p,
                Branch::South,
                8_000e3 / 3.844e8,
                &CorrectorConfig::default(),
            )
            .expect("halo");
        assert!(h.orbit.state[2] < 0.0 && h.orbit.iterations <= 10);
    }
}

/// 記憶による緩い目安（未検証）: 太陽–地球 L2、z 振幅 ~120 000 km のハローの周期は約 180 日。
#[test]
fn sun_earth_l2_halo_period_is_about_half_a_year() {
    let cr = sun_earth_moon();
    let h = se_l2(&cr, Branch::North);
    let n = (crate::constants::GM_SUN + crate::constants::MU_EARTH + crate::constants::GM_MOON)
        / AU.powi(3);
    let days = h.orbit.period / n.sqrt() / crate::constants::SECONDS_PER_DAY;
    assert!((170.0..190.0).contains(&days), "{days}");
}

/// 1 周期の表: 最後の点は最初と同じ状態、`as_table` の長さが合う。
#[test]
fn sampled_orbit_closes_and_gives_a_table() {
    let cr = sun_earth_moon();
    let h = se_l2(&cr, Branch::North);
    let cfg = CorrectorConfig::default();
    let s = cr
        .sample_orbit(&h.orbit, 100, &cfg.integrator)
        .expect("sample");
    let (t, st) = s.as_table();
    assert_eq!((t.len(), st.len()), (101, 101));
    let (a, b) = (s.states[0], s.states[100]);
    assert!((0..6).all(|i| (a[i] - b[i]).abs() < 1e-8));
    assert!((s.z_amplitude() - 0.0).abs() > 0.0);
    assert!(cr.sample_orbit(&h.orbit, 1, &cfg.integrator).is_err());
}
