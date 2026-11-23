//! 能力の曲線の引き方のテスト（手計算との比較）。

use super::*;

/// C3 [km²/s²] → 質量 [kg]: (0, 500)・(10, 400)・(20, 250)・(30, 100)。
fn curve() -> Vec<CapabilityPoint> {
    [(0.0, 500.0), (10.0, 400.0), (20.0, 250.0), (30.0, 100.0)]
        .into_iter()
        .map(|(c3, kg)| CapabilityPoint {
            c3: Qty::from_raw(c3 * 1e6),
            mass: Qty::from_raw(kg),
        })
        .collect()
}

fn at(c3_km2_s2: f64) -> Option<f64> {
    mass_at_c3(&curve(), c3_km2_s2 * 1e6).unwrap()
}

/// 節点の C3 では、その点の質量そのもの。
#[test]
fn nodes_return_their_own_mass() {
    assert_eq!(at(0.0), Some(500.0));
    assert_eq!(at(10.0), Some(400.0));
    assert_eq!(at(20.0), Some(250.0));
    assert_eq!(at(30.0), Some(100.0));
}

/// 手計算: C3 = 12 は (10, 400) と (20, 250) の間で t = 0.2、400 − 0.2 × 150 = 370 kg。
/// C3 = 15 は t = 0.5 で 325 kg、C3 = 25 は (20, 250) と (30, 100) の中点で 175 kg。
#[test]
fn between_nodes_is_linear_in_c3() {
    assert!((at(12.0).unwrap() - 370.0).abs() < 1e-9);
    assert!((at(15.0).unwrap() - 325.0).abs() < 1e-9);
    assert!((at(25.0).unwrap() - 175.0).abs() < 1e-9);
}

/// 最後の点より大きい C3 は外挿せず送れない（`None`）。最初の点より小さい C3 は最初の点の質量。
#[test]
fn beyond_the_end_is_not_capable_and_below_the_start_is_clamped() {
    assert_eq!(at(30.000001), None);
    assert_eq!(at(60.0), None);
    assert_eq!(at(-5.0), Some(500.0));
}

#[test]
fn a_curve_with_one_point_is_a_constant_up_to_that_point() {
    let one = vec![curve().remove(1)];
    assert_eq!(mass_at_c3(&one, 5e6).unwrap(), Some(400.0));
    assert_eq!(mass_at_c3(&one, 10e6).unwrap(), Some(400.0));
    assert_eq!(mass_at_c3(&one, 10.1e6).unwrap(), None);
}

#[test]
fn malformed_curves_are_input_errors() {
    assert!(mass_at_c3(&[], 0.0).is_err());
    let mut unsorted = curve();
    unsorted.swap(1, 2);
    assert!(mass_at_c3(&unsorted, 1e6).is_err());
    let mut zero = curve();
    zero[3].mass = Qty::from_raw(0.0);
    assert!(mass_at_c3(&zero, 1e6).is_err());
    assert!(mass_at_c3(&curve(), f64::NAN).is_err());
}
