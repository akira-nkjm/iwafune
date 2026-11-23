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
    Epoch::parse("2027-04-01T00:00:00Z").expect("epoch")
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
    let passes = find_passes(&gs, &orbit, epoch(), 86_400.0, 30.0).expect("passes");
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
            "duration {d} s (expected {expected} s)"
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
    let coarse = find_passes(&gs, &orbit, epoch(), 86_400.0, 60.0).expect("passes");
    let fine = find_passes(&gs, &orbit, epoch(), 86_400.0, 10.0).expect("passes");
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

#[test]
fn slant_range_at_zenith_horizon_and_on_the_moon() {
    // 天頂では高度そのもの、地平線では √((R + h)² − R²)（接線の長さ）
    let h = 500.0e3;
    assert!((slant_range_m(R_EARTH, h, std::f64::consts::FRAC_PI_2) - h).abs() < 1e-6);
    let horizon = ((R_EARTH + h).powi(2) - R_EARTH.powi(2)).sqrt();
    assert!((slant_range_m(R_EARTH, h, 0.0) - horizon).abs() < 1e-6);
    // 月（R = 1737.4 km）の高度 100 km を地平線で見る: √(100 × 3574.8) km ≈ 597.9 km
    let moon = slant_range_m(crate::constants::R_MOON, 100.0e3, 0.0);
    assert!((moon - 597.9e3).abs() < 0.1e3, "{moon}");
}

/// 月を回る極軌道（軌道面が地球と月を結ぶ線を含む）: 月の向こう側（元期の位置）では局から見て月の陰で
/// 見えず（余裕 < 0）、手前側（半周後）では見える。局から衛星への線分と球の交わりを 2 次方程式で解いた値と全標本で一致する。
#[test]
fn moon_orbiter_behind_the_moon_is_occulted() {
    use crate::ephem::CentralBody;
    let moon = CentralBody::Moon.position_eci_m(epoch());
    let mut orbit = OrbitSpec::circular_about(
        CentralBody::Moon,
        epoch(),
        Qty::from_raw(100.0e3),
        Qty::from_raw(core::f64::consts::FRAC_PI_2),
        Qty::from_raw(moon.y.atan2(moon.x)),
    );
    orbit.propagator = Propagator::Kepler;
    let period = orbit.period_s();
    let gs = station(0.0, 0.0, 0.0);
    let track = Track::new(&gs, &orbit, epoch(), period, 60.0).expect("track");
    let hidden = |t: f64| {
        // |S + u d − C|² = R² が u ∈ [0, 1] に解を持つか
        let e = epoch().plus_seconds(t);
        let sat = eci_to_ecef(track.position(t), e);
        let centre = eci_to_ecef(CentralBody::Moon.position_eci_m(e), e);
        let (d, c) = (sat - track.topo.pos, centre - track.topo.pos);
        let (a, b) = (d.dot(d), -2.0 * d.dot(c));
        let k = c.dot(c) - CentralBody::Moon.radius_m().powi(2);
        let disc = b * b - 4.0 * a * k;
        disc >= 0.0 && {
            let u = (-b - disc.sqrt()) / (2.0 * a);
            (0.0..=1.0).contains(&u)
        }
    };
    let mut seen = [false, false];
    for i in 0..=(period / 60.0) as usize {
        let t = i as f64 * 60.0;
        let margin = track.occultation_margin(CentralBody::Moon, t);
        assert_eq!(margin < 0.0, hidden(t), "t = {t}: margin {margin}");
        seen[usize::from(margin < 0.0)] = true;
    }
    assert!(
        seen[0] && seen[1],
        "both occulted and visible samples are expected"
    );
    assert!(track.occultation_margin(CentralBody::Moon, 0.0) < 0.0);
    assert!(track.occultation_margin(CentralBody::Moon, 0.5 * period) > 0.0);
}
