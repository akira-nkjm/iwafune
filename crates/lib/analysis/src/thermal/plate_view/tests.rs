//! 板のメッシュの幾何の手計算（四隅の位置・向き・通し番号）。

use super::*;
use crate::structure::interference::InterferenceBox;
use sat_astro::math::Quat;

fn plate(position: Vec3, orientation: Quat) -> ThermalPlate {
    ThermalPlate {
        part: "P".to_owned(),
        cells: [3, 2],
        conductivity: Qty::from_raw(100.0),
        frame: InterferenceBox {
            id: "P".to_owned(),
            count: 1,
            position,
            orientation,
            size: Vec3::new(0.3, 0.2, 0.003),
            mount_ancestors: Vec::new(),
        },
        attachments: Vec::new(),
    }
}

fn field(plates: &[ThermalPlate]) -> ThermalPlateTemperatures {
    let temperatures = ["P[0,0]", "P[0,1]", "P[1,0]", "P[1,1]", "P[2,0]", "P[2,1]"]
        .into_iter()
        .zip([300.0, 301.0, 302.0, 303.0, 304.0, 305.0])
        .collect();
    let source = PlateFieldSource {
        quantity: ThermalPlateQuantity::End,
        case: None,
        temperatures,
    };
    plate_temperatures(plates, vec![source]).unwrap().unwrap()
}

/// 手計算: 0.3 × 0.2 × 0.003 m（法線は z、u = x、v = y）を (1, 2, 3) に置くと、頂点 (0,0) は (0.85, 1.9, 3)、(3,2) は (1.15, 2.1, 3)。
/// 升 (2,1)（通し番号 5）の四隅は (1.05, 2.0)・(1.15, 2.0)・(1.15, 2.1)・(1.05, 2.1)（左回り）。
#[test]
fn identity_pose_by_hand() {
    let d = field(&[plate(Vec3::new(1.0, 2.0, 3.0), Quat::IDENTITY)]);
    assert_eq!(d.mesh.nodes.len(), 4 * 3);
    assert_eq!(d.mesh.groups[0].quads.len(), 6);
    let near = |p: [f32; 3], x: f32, y: f32, z: f32| {
        assert!(
            (p[0] - x).abs() < 1e-5 && (p[1] - y).abs() < 1e-5 && (p[2] - z).abs() < 1e-5,
            "{p:?} vs ({x}, {y}, {z})"
        );
    };
    let at = |k: u32| d.mesh.nodes[k as usize];
    near(d.mesh.nodes[0], 0.85, 1.9, 3.0);
    near(d.mesh.nodes[11], 1.15, 2.1, 3.0);
    let q = d.mesh.groups[0].quads[5];
    near(at(q[0]), 1.05, 2.0, 3.0);
    near(at(q[1]), 1.15, 2.0, 3.0);
    near(at(q[2]), 1.15, 2.1, 3.0);
    near(at(q[3]), 1.05, 2.1, 3.0);
    assert_eq!(d.cells[5].node, "P[2,1]");
    assert_eq!((d.cells[5].cell.i, d.cells[5].cell.j), (2, 1));
    assert_eq!(d.fields[0].temperature[5].value(), 305.0);
}

/// 箱を z 軸まわりに 90° 回すと、部品の x 軸は機体の y 軸に向く: 頂点 (3,2)（部品の (0.15, 0.1)）は
/// 機体で (−0.1, 0.15) のずれ → (0.9, 2.15, 3)。
#[test]
fn the_box_pose_is_applied() {
    let q = Quat::from_axis_angle(Vec3::Z, std::f64::consts::FRAC_PI_2);
    let d = field(&[plate(Vec3::new(1.0, 2.0, 3.0), q)]);
    let p = d.mesh.nodes[11];
    assert!((p[0] - 0.9).abs() < 1e-5 && (p[1] - 2.15).abs() < 1e-5 && (p[2] - 3.0).abs() < 1e-5);
}

#[test]
fn no_plates_means_no_detail() {
    assert!(plate_temperatures(&[], Vec::new()).unwrap().is_none());
}

#[test]
fn a_missing_cell_temperature_is_an_error() {
    let source = PlateFieldSource {
        quantity: ThermalPlateQuantity::Min,
        case: None,
        temperatures: BTreeMap::new(),
    };
    assert!(plate_temperatures(&[plate(Vec3::ZERO, Quat::IDENTITY)], vec![source]).is_err());
}
