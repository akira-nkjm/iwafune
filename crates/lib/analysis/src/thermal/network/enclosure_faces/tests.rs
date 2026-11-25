//! 外殻の 6 面の節点を、候補が複数のときだけ位置で絞って決めるテスト。箱は辺 0.1 / 0.2 / 0.3 m（原点中心）で、各面に厚さ 2 mm のパネルが面の平面にまたがって載る
//! （面の外向きの面は、箱の面から 1 mm 外にある）。許容距離は最も短い辺 0.1 m の 5 % = 5 mm。

use super::super::couplings::tests::{mounts, net};
use super::super::{Compiled, ThermalNetwork, ThermalSurface};
use crate::structure::interference::InterferenceBox;
use crate::thermal::enclosure::{ThermalBoxSize, ThermalEnclosure};
use sat_astro::math::{Quat, Vec3};
use sat_units::typed::Qty;

const FACES: [&str; 6] = ["fpx", "fmx", "fpy", "fmy", "fpz", "fmz"];
const SIZE: [f64; 3] = [0.1, 0.2, 0.3];
const NORMALS: [[f64; 3]; 6] = [
    [1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, -1.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, -1.0],
];

fn plate_box(id: &str, center: [f64; 3], size: [f64; 3]) -> InterferenceBox {
    InterferenceBox {
        id: id.to_owned(),
        count: 1,
        position: Vec3::new(center[0], center[1], center[2]),
        orientation: Quat::IDENTITY,
        size: Vec3::new(size[0], size[1], size[2]),
        mount_ancestors: vec![],
    }
}

fn surface(id: &str, node: &str, normal: [f64; 3], area: f64) -> ThermalSurface {
    ThermalSurface {
        id: id.to_owned(),
        node: node.to_owned(),
        area: Qty::from_raw(area),
        normal: Vec3::new(normal[0], normal[1], normal[2]),
        count: 1,
        absorptivity: Some(0.3),
        emissivity: Some(0.8),
        coatings: vec![],
    }
}

/// 機体の外に立つ板: 外面の法線、面積、箱の中心と寸法。
type Paddle = ([f64; 3], f64, [f64; 3], [f64; 3]);

/// 6 面のパネルと、機体の外に立つ板 `paddle`（`normal`・`area` が指定どおり、箱は `center`・`size`）を持つ網。
fn scene(paddle: Option<Paddle>) -> ThermalNetwork {
    let mut nodes: Vec<&str> = FACES.to_vec();
    nodes.push("pad");
    let mut n: ThermalNetwork = net(&nodes);
    let mut boxes = Vec::new();
    for (i, face) in FACES.iter().enumerate() {
        let k = i / 2;
        let area: f64 = (0..3).filter(|a| *a != k).map(|a| SIZE[a]).product();
        n.surfaces.push(surface(face, face, NORMALS[i], area));
        // 板厚 2 mm のパネルが箱の面の平面にまたがる（箱の面から外へ 1 mm）
        let mut center = [0.0; 3];
        center[k] = NORMALS[i][k] * SIZE[k] / 2.0;
        let mut size = SIZE;
        size[k] = 0.002;
        boxes.push(plate_box(face, center, size));
    }
    if let Some((normal, area, center, size)) = paddle {
        n.surfaces.push(surface("paddle", "pad", normal, area));
        boxes.push(plate_box("paddle", center, size));
    }
    n.mounts = mounts(&[["pad", "fpy"]], 0.5, Some(0.002));
    n.enclosure = Some(ThermalEnclosure {
        size: Some(ThermalBoxSize {
            x: Qty::from_raw(SIZE[0]),
            y: Qty::from_raw(SIZE[1]),
            z: Qty::from_raw(SIZE[2]),
        }),
        emissivity: 0.8,
        center: Some(Vec3::ZERO),
        boxes,
    });
    n
}

fn used(n: &ThermalNetwork) -> crate::thermal::network::ThermalEnclosureUsed {
    Compiled::new(n).unwrap().couplings.enclosure.unwrap()
}

/// 機体の外（+Y 側。x ≈ +0.05 m、y ∈ [0.1, 0.3] m）に立つ板の、法線 −X・面積 0.2×0.3 m² は、−X 面（面積 0.2×0.3 m²）と同じ。
/// 位置を見なければ「−X 面が複数の節点に合う」で外殻は使えない（従来）。位置を見れば、板は −X 面（x = −0.05）の上に無いので除かれ、6 面の節点が決まる。
#[test]
fn a_plate_outside_the_body_is_not_a_box_face() {
    let paddle = Some((
        [-1.0, 0.0, 0.0],
        0.2 * 0.3,
        [0.049, 0.2, 0.0],
        [0.002, 0.2, 0.3],
    ));
    let mut n = scene(paddle);
    n.enclosure.as_mut().unwrap().boxes.clear();
    let before = used(&n);
    assert!(!before.applied);
    assert!(
        before
            .note
            .unwrap()
            .contains("-X face of the box matches several nodes")
    );

    let n = scene(paddle);
    let after = used(&n);
    assert!(after.applied, "{:?}", after.note);
    let nodes: Vec<&str> = after.faces.iter().map(|f| f.node.as_str()).collect();
    assert_eq!(nodes, FACES);
}

/// 板が機体の +X 面と同じ向き・面積で、面の平面から 10 mm 外に立つ（畳んで浮かせたパドルの背面）。許容距離は 5 mm なので面の上にない。
/// 面の平面の上（面から 1 mm 外）で、面と重ならない（y 方向に 0.4 m ずれた）板も、重なる面積が 0 なので面の上にない。
/// 同じ板が面の平面から 3 mm 外で面と重なれば、面の上にあり、+X 面は 2 つの節点に合う。
#[test]
fn position_decides_by_plane_distance_and_overlap() {
    let area = 0.2 * 0.3;
    let stood_off = Some(([1.0, 0.0, 0.0], area, [0.061, 0.0, 0.0], [0.002, 0.2, 0.3]));
    assert!(used(&scene(stood_off)).applied);
    let beside = Some(([1.0, 0.0, 0.0], area, [0.051, 0.4, 0.0], [0.002, 0.2, 0.3]));
    assert!(used(&scene(beside)).applied);
    let near = Some(([1.0, 0.0, 0.0], area, [0.053, 0.0, 0.0], [0.002, 0.2, 0.3]));
    let u = used(&scene(near));
    assert!(!u.applied);
    assert!(
        u.note
            .unwrap()
            .contains("+X face of the box matches several nodes")
    );
}

/// 候補の節点が 1 つなら、部品の箱が面の上になくても、その節点に決まる（従来どおり）。箱を +X へ 1 m 動かし、外殻の中心を渡さなくても（原点とみなして）6 面とも 1 つずつの候補なので適用される。
/// 候補が複数（機体の外に立つ板が同じ向き・面積）なら位置で絞るが、どれも面の上にないなら理由に「面の上にあるはず」と出る。
#[test]
fn a_single_candidate_is_taken_wherever_its_box_is() {
    let shift = |n: &mut ThermalNetwork| {
        let e = n.enclosure.as_mut().unwrap();
        for b in &mut e.boxes {
            b.position = Vec3::new(b.position.x + 1.0, b.position.y, b.position.z);
        }
        e.center = None;
    };
    let mut n = scene(None);
    shift(&mut n);
    assert!(used(&n).applied);
    let paddle = Some((
        [-1.0, 0.0, 0.0],
        0.2 * 0.3,
        [0.049, 0.2, 0.0],
        [0.002, 0.2, 0.3],
    ));
    let mut n = scene(paddle);
    shift(&mut n);
    let u = used(&n);
    assert!(!u.applied);
    assert!(u.note.unwrap().contains("must lie on the face"));
}

/// 部品の箱が分からない候補（寸法を持たない部品）は位置が決まらないので残る: 機体の外の板の箱が無ければ、−X 面は板と `fmx` の 2 つの節点に合う。
#[test]
fn a_candidate_without_a_box_is_kept() {
    let paddle = Some((
        [-1.0, 0.0, 0.0],
        0.2 * 0.3,
        [0.049, 0.2, 0.0],
        [0.002, 0.2, 0.3],
    ));
    let mut n = scene(paddle);
    n.enclosure
        .as_mut()
        .unwrap()
        .boxes
        .retain(|b| b.id != "paddle");
    let u = used(&n);
    assert!(!u.applied);
    assert!(
        u.note
            .unwrap()
            .contains("-X face of the box matches several nodes")
    );
}
