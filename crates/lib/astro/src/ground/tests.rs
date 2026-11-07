use super::*;
use crate::constants::{MU_EARTH, OMEGA_EARTH, R_EARTH};
use crate::earth::ecef_to_eci;
use crate::orbit::Propagator;

fn station(lat_deg: f64, lon_deg: f64, min_el_deg: f64) -> GroundStation {
    GroundStation {
        id: String::from("GS"),
        latitude: Qty::from_raw(lat_deg.to_radians()),
        longitude: Qty::from_raw(lon_deg.to_radians()),
        altitude: Qty::from_raw(0.0),
        min_elevation: Qty::from_raw(min_el_deg.to_radians()),
    }
}

fn epoch() -> Epoch {
    Epoch::parse("2027-04-01T00:00:00Z").expect("元期")
}

#[test]
fn look_angles_overhead_and_north() {
    let gs = station(35.0, 139.0, 0.0);
    let e = epoch();
    let up = geodetic_to_ecef(35f64.to_radians(), 139f64.to_radians(), 550e3);
    let la = look_angles(&gs, e, ecef_to_eci(up, e));
    assert!((la.elevation_rad.to_degrees() - 90.0).abs() < 1e-6);
    assert!((la.range_m - 550e3).abs() < 1e-3);
    // 北に少し離れた点は方位 ≈ 0°
    let north = geodetic_to_ecef(36f64.to_radians(), 139f64.to_radians(), 550e3);
    let la = look_angles(&gs, e, ecef_to_eci(north, e));
    let az = la.azimuth_rad.to_degrees();
    assert!(!(1.0..=359.0).contains(&az), "{az}");
    assert!(la.elevation_rad > 0.0);
}

/// 赤道上の局と赤道円軌道（Kepler）: すべて天頂通過、パスの長さは 2λ/(n − ω_E)、cos λ = R/r。
#[test]
fn equatorial_passes_match_closed_form() {
    let gs = station(0.0, 0.0, 0.0);
    let mut orbit = OrbitSpec::circular(
        epoch(),
        Qty::from_raw(550e3),
        Qty::from_raw(0.0),
        Qty::from_raw(0.0),
    );
    orbit.propagator = Propagator::Kepler;
    let passes = find_passes(&gs, &orbit, epoch(), 86_400.0, 30.0).expect("パス");
    let r = R_EARTH + 550e3;
    let n = (MU_EARTH / r.powi(3)).sqrt();
    let lambda = (R_EARTH / r).acos();
    let expected = 2.0 * lambda / (n - OMEGA_EARTH);
    let synodic = core::f64::consts::TAU / (n - OMEGA_EARTH);
    let full: Vec<&GroundPass> = passes
        .iter()
        .filter(|p| p.aos.j2000_s > epoch().j2000_s && p.los.j2000_s < epoch().j2000_s + 86_400.0)
        .collect();
    assert!(full.len() >= 13 && passes.len() <= 15, "{}", passes.len());
    for p in &full {
        let d = p.duration.value();
        assert!(
            (d - expected).abs() < 1.0,
            "長さ {d} s（期待 {expected} s）"
        );
        assert!(p.max_elevation.value().to_degrees() > 89.5);
        assert!(p.aos.j2000_s < p.tca.j2000_s && p.tca.j2000_s < p.los.j2000_s);
    }
    let gap = full[1].aos.j2000_s - full[0].aos.j2000_s;
    assert!((gap - synodic).abs() < 1.0, "{gap} vs {synodic}");
}

#[test]
fn sso_passes_over_tokyo_are_plausible() {
    let gs = station(35.7, 139.7, 5.0);
    let i = crate::orbit::sso_inclination_rad(R_EARTH + 550e3, 0.0);
    let orbit = OrbitSpec::circular(
        epoch(),
        Qty::from_raw(550e3),
        Qty::from_raw(i),
        Qty::from_raw(1.0),
    );
    let coarse = find_passes(&gs, &orbit, epoch(), 86_400.0, 60.0).expect("パス");
    let fine = find_passes(&gs, &orbit, epoch(), 86_400.0, 10.0).expect("パス");
    assert!((3..=8).contains(&coarse.len()), "{}", coarse.len());
    assert_eq!(coarse.len(), fine.len());
    for (c, f) in coarse.iter().zip(fine.iter()) {
        assert!(c.duration.value() < 12.5 * 60.0);
        assert!((c.aos.j2000_s - f.aos.j2000_s).abs() < 0.05);
        assert!((c.max_elevation.value() - f.max_elevation.value()).abs() < 1e-4);
    }
}

#[test]
fn rejects_bad_inputs() {
    let orbit = OrbitSpec::circular(
        epoch(),
        Qty::from_raw(550e3),
        Qty::from_raw(1.0),
        Qty::from_raw(0.0),
    );
    assert!(find_passes(&station(0.0, 0.0, 0.0), &orbit, epoch(), 3600.0, 0.0).is_err());
    assert!(find_passes(&station(95.0, 0.0, 0.0), &orbit, epoch(), 3600.0, 10.0).is_err());
}
