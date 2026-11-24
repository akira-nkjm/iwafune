//! 部品の箱から面積と腕を出す擾乱トルクの確認（手計算）。面の評価そのものの確認は `disturbance/surface/tests.rs`。

use super::*;
use crate::aocs::disturbance::{DisturbanceGeometry, DisturbanceSurface, SurfaceValueSource};
use crate::power::testkit::{leo_timeline, sun_pointing_z};
use crate::structure::interference::InterferenceBox;
use sat_astro::constants::C_LIGHT;
use sat_astro::math::{Mat3, Quat};

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

/// 箱 1 つ（0.1 × 0.2 × 0.3 m、中心 (0.01, 0.02, 0.03)）、重心は原点、太陽を +z に向ける。
fn sun_pointing_box(srp: DisturbanceSurface) -> RwUnloadingInput {
    let b = InterferenceBox {
        id: "B".to_owned(),
        count: 1,
        position: Vec3::new(0.01, 0.02, 0.03),
        orientation: Quat::IDENTITY,
        size: Vec3::new(0.1, 0.2, 0.3),
        mount_ancestors: Vec::new(),
    };
    RwUnloadingInput {
        timeline: leo_timeline(sun_pointing_z(), 2.0, 10.0),
        spacecraft: DisturbanceSpacecraft {
            inertia: Mat3::diagonal(Vec3::new(0.05, 0.05, 0.05)),
            aero: None,
            srp: Some(srp),
            geometry: Some(DisturbanceGeometry {
                boxes: vec![b],
                center_of_mass: Vec3::ZERO,
            }),
            residual_dipole: None,
            atmosphere: Default::default(),
            density_scale: None,
            thrust: None,
        },
        wheels: wheels(),
        usable_momentum_fraction: None,
        required_unload_interval: None,
        rcs: None,
    }
}

fn from_geometry() -> DisturbanceSurface {
    DisturbanceSurface {
        coefficient: 1.5,
        area: None,
        cp_offset: None,
    }
}

/// 太陽指向（+z が太陽、機体系で一定）: 面積 = x·y = 0.02 m²、腕 = (0.01, 0.02, 0)、F = −P·Cr·A·ẑ。
/// τ = r × F の大きさは P·Cr·A·|r| = P · 1.5 · 0.02 · √(0.01² + 0.02²)。P は太陽距離で ±3 % 変わるので 1 AU の値と 4 % 以内で比べる。
/// 面積は太陽指向で一定（腕も）なので、最小 = 最大。
#[test]
fn srp_torque_uses_the_projected_area_and_the_centroid_arm() {
    let out = RwUnloading::run(&sun_pointing_box(from_geometry())).unwrap();
    let expect = 1361.0 / C_LIGHT * 1.5 * 0.02 * (0.01f64.hypot(0.02));
    let peak = out.disturbances[2].peak.value();
    assert_eq!(out.disturbances[2].key, DisturbanceKind::Srp);
    assert!((peak - expect).abs() < 0.04 * expect, "{peak} vs {expect}");
    let [surface] = &out.surfaces[..] else {
        panic!("one surface (SRP) expected: {:?}", out.surfaces);
    };
    assert_eq!(surface.key, DisturbanceKind::Srp);
    assert_eq!(surface.area_source, SurfaceValueSource::Geometry);
    assert_eq!(surface.cp_source, SurfaceValueSource::Geometry);
    assert!((surface.area_min.value() - 0.02).abs() < 1e-9);
    assert!((surface.area_max.value() - 0.02).abs() < 1e-9);
    assert!((surface.arm_max.value() - 0.01f64.hypot(0.02)).abs() < 1e-9);
}

/// 書かれた圧力中心（x に 5 cm）は箱の腕に勝つ。面積は箱から（0.02）。
/// τ の大きさは P·Cr·A·0.05。出力は圧力中心が書かれた値で、面積は箱だと言う。
#[test]
fn a_written_centre_of_pressure_replaces_the_geometric_arm() {
    let mut surface = from_geometry();
    surface.cp_offset = Some(Vec3::new(0.05, 0.0, 0.0));
    let out = RwUnloading::run(&sun_pointing_box(surface)).unwrap();
    let expect = 1361.0 / C_LIGHT * 1.5 * 0.02 * 0.05;
    let peak = out.disturbances[2].peak.value();
    assert!((peak - expect).abs() < 0.04 * expect, "{peak} vs {expect}");
    let s = &out.surfaces[0];
    assert_eq!(s.cp_source, SurfaceValueSource::Written);
    assert_eq!(s.area_source, SurfaceValueSource::Geometry);
    assert!((s.arm_max.value() - 0.05).abs() < 1e-12);
}

/// 書かれた面積（0.1 m²）は箱の面積に勝つ。腕は箱から。τ = P·Cr·0.1·|r|。
#[test]
fn a_written_area_replaces_the_geometric_area() {
    let mut surface = from_geometry();
    surface.area = Some(Qty::from_raw(0.1));
    let out = RwUnloading::run(&sun_pointing_box(surface)).unwrap();
    let expect = 1361.0 / C_LIGHT * 1.5 * 0.1 * 0.01f64.hypot(0.02);
    let peak = out.disturbances[2].peak.value();
    assert!((peak - expect).abs() < 0.04 * expect, "{peak} vs {expect}");
    let s = &out.surfaces[0];
    assert_eq!(s.area_source, SurfaceValueSource::Written);
    assert_eq!(s.cp_source, SurfaceValueSource::Geometry);
    assert_eq!(s.area_min.value(), 0.1);
}

/// 面積を書かず、使える箱も無ければ、黙って 0 にせず入力の誤り。
#[test]
fn an_area_that_is_neither_written_nor_derivable_is_an_error() {
    let mut i = sun_pointing_box(from_geometry());
    i.spacecraft.geometry = None;
    assert!(RwUnloading::run(&i).is_err());
}
