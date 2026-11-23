//! ラグランジュ点のテスト。導ける条件（5 次方程式の残差、勾配 0、正三角形、ヒル近似）を先に置く。

use super::*;
use crate::constants::AU;

fn sun_earth_moon() -> Cr3bp {
    Cr3bp::from_bodies(CentralBody::Sun, Secondary::EarthMoonSystem).expect("mu")
}

fn earth_moon() -> Cr3bp {
    Cr3bp::from_bodies(CentralBody::Earth, Secondary::Body(CentralBody::Moon)).expect("mu")
}

fn systems() -> Vec<Cr3bp> {
    vec![
        sun_earth_moon(),
        earth_moon(),
        Cr3bp::new(0.3).expect("mu"),
        Cr3bp::new(1e-9).expect("mu"),
    ]
}

/// Szebehely の 5 次方程式（γ は副天体または主天体からの距離）。L1・L2・L3 の順。
fn quintic(mu: f64, p: LibrationPoint, g: f64) -> f64 {
    match p {
        LibrationPoint::L1 => {
            g.powi(5) - (3.0 - mu) * g.powi(4) + (3.0 - 2.0 * mu) * g.powi(3) - mu * g * g
                + 2.0 * mu * g
                - mu
        }
        LibrationPoint::L2 => {
            g.powi(5) + (3.0 - mu) * g.powi(4) + (3.0 - 2.0 * mu) * g.powi(3)
                - mu * g * g
                - 2.0 * mu * g
                - mu
        }
        _ => {
            g.powi(5) + (2.0 + mu) * g.powi(4) + (1.0 + 2.0 * mu) * g.powi(3)
                - (1.0 - mu) * g * g
                - 2.0 * (1.0 - mu) * g
                - (1.0 - mu)
        }
    }
}

#[test]
fn collinear_points_satisfy_the_quintic_and_have_zero_gradient() {
    // 許容: 二分法の幅 1e-16 以下 × 勾配の傾き（≲ 10）+ 丸め ≲ 1e-15。残差 1e-12 / 1e-13 は十分な余裕。
    for cr in systems() {
        for p in [LibrationPoint::L1, LibrationPoint::L2, LibrationPoint::L3] {
            let x = cr.libration_point(p).expect("point")[0];
            let g = cr.gamma(p).expect("gamma");
            assert!(quintic(cr.mu(), p, g).abs() < 1e-13, "{p:?} mu={}", cr.mu());
            let grad = cr.potential_gradient([x, 0.0, 0.0]);
            assert!(grad.iter().all(|v| v.abs() < 1e-12), "{p:?} {grad:?}");
        }
    }
}

#[test]
fn triangular_points_are_equilateral_with_zero_gradient() {
    for cr in systems() {
        let mu = cr.mu();
        for p in [LibrationPoint::L4, LibrationPoint::L5] {
            let [x, y, z] = cr.libration_point(p).expect("point");
            let r1 = ((x + mu).powi(2) + y * y + z * z).sqrt();
            let r2 = ((x - 1.0 + mu).powi(2) + y * y + z * z).sqrt();
            assert!((r1 - 1.0).abs() < 1e-15 && (r2 - 1.0).abs() < 1e-15);
            let grad = cr.potential_gradient([x, y, z]);
            assert!(grad.iter().all(|v| v.abs() < 1e-14), "{grad:?}");
        }
    }
}

#[test]
fn collinear_points_are_ordered() {
    for cr in systems() {
        let x = |p| cr.libration_point(p).expect("point")[0];
        let mu = cr.mu();
        assert!(x(LibrationPoint::L3) < -mu);
        assert!(-mu < x(LibrationPoint::L1) && x(LibrationPoint::L1) < 1.0 - mu);
        assert!(1.0 - mu < x(LibrationPoint::L2));
    }
}

/// ヒル近似: h = (μ/3)^(1/3) として L1/L2 の副天体からの距離は γ = h(1 ∓ h/3 − h²/9 + …)。
/// 1 次までで切ると残りは相対 h²/9 程度。許容は相対 h²（残りの 9 倍の余裕）。
#[test]
fn hill_approximation_with_the_stated_correction() {
    for cr in systems() {
        let h = (cr.mu() / 3.0).cbrt();
        for (p, sign) in [(LibrationPoint::L1, -1.0), (LibrationPoint::L2, 1.0)] {
            let rel = cr.gamma(p).expect("gamma") / h - (1.0 + sign * h / 3.0);
            assert!(rel.abs() <= h * h, "{p:?} mu={} rel={rel}", cr.mu());
        }
    }
}

/// μ → 0 の極限: γ → h。相対の差は h/3 程度なので、許容は相対 h。
#[test]
fn distance_tends_to_the_hill_value_as_mu_goes_to_zero() {
    for mu in [1e-6, 1e-9, 1e-12] {
        let cr = Cr3bp::new(mu).expect("mu");
        let h = (mu / 3.0).cbrt();
        for p in [LibrationPoint::L1, LibrationPoint::L2] {
            assert!(
                (cr.gamma(p).expect("gamma") / h - 1.0).abs() <= h,
                "mu={mu}"
            );
        }
    }
}

/// Sun-(Earth+Moon) の L1/L2 は a·h ≈ 1.5e6 km。ヒル値との差は相対 h/3（≈ 0.7%）なので、許容は a·h·(h/3 + h²)。
#[test]
fn sun_earth_collinear_distance_is_about_1_5_million_km() {
    let cr = sun_earth_moon();
    let h = (cr.mu() / 3.0).cbrt();
    let a_km = AU / 1e3;
    for p in [LibrationPoint::L1, LibrationPoint::L2] {
        let d_km = cr.gamma(p).expect("gamma") * a_km;
        assert!((d_km - h * a_km).abs() <= h * a_km * (h / 3.0 + h * h));
        // 記憶による値（未検証）: 1.5e6 km 前後
        assert!((1.4e6..1.6e6).contains(&d_km), "{d_km}");
    }
}

#[test]
fn earth_moon_points_are_near_the_remembered_values() {
    // 記憶による値（未検証）: L1 ≈ 0.8369、L2 ≈ 1.1557（ともに a = 1 の単位）。緩い境界のみ。
    let cr = earth_moon();
    let l1 = cr.libration_point(LibrationPoint::L1).expect("l1")[0];
    let l2 = cr.libration_point(LibrationPoint::L2).expect("l2")[0];
    assert!(
        (l1 - 0.8369).abs() < 2e-3 && (l2 - 1.1557).abs() < 2e-3,
        "{l1} {l2}"
    );
}

#[test]
fn linear_modes_follow_from_the_hessian() {
    for cr in systems() {
        for p in [LibrationPoint::L1, LibrationPoint::L2, LibrationPoint::L3] {
            let m = cr.linear_modes(p).expect("modes");
            let h = cr.potential_hessian(cr.libration_point(p).expect("point"));
            assert!((h[0][0] - (1.0 + 2.0 * m.c)).abs() < 1e-12 * m.c.max(1.0));
            assert!((h[1][1] - (1.0 - m.c)).abs() < 1e-12 * m.c.max(1.0));
            // 面内: ω⁴ − (2−c)ω² + (1+2c)(1−c) = 0、鉛直: ω_z² = c
            let w2 = m.omega_planar.powi(2);
            let res = w2 * w2 - (2.0 - m.c) * w2 + (1.0 + 2.0 * m.c) * (1.0 - m.c);
            assert!(res.abs() < 1e-12 * m.c.max(1.0).powi(2), "{res}");
            assert!((m.omega_vertical.powi(2) - m.c).abs() < 1e-15 * m.c.max(1.0));
        }
    }
}

#[test]
fn invalid_mass_ratio_and_non_collinear_requests_are_typed_errors() {
    assert!(matches!(
        Cr3bp::new(0.0),
        Err(Cr3bpError::InvalidMassRatio(_))
    ));
    assert!(matches!(
        Cr3bp::new(0.7),
        Err(Cr3bpError::InvalidMassRatio(_))
    ));
    let cr = earth_moon();
    assert!(matches!(
        cr.linear_modes(LibrationPoint::L4),
        Err(Cr3bpError::NotCollinear(_))
    ));
}
