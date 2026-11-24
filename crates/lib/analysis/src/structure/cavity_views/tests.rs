use super::*;
use sat_astro::math::Quat;
use sat_astro::view_factor::parallel_opposed;

fn b(id: &str, center: [f64; 3], size: [f64; 3]) -> InterferenceBox {
    InterferenceBox {
        id: id.to_owned(),
        count: 1,
        position: Vec3::new(center[0], center[1], center[2]),
        orientation: Quat::IDENTITY,
        size: Vec3::new(size[0], size[1], size[2]),
        mount_ancestors: Vec::new(),
    }
}

/// 一辺 1 m の容器（中空の殻）。
fn frame() -> InterferenceBox {
    b("FRAME", [0.0; 3], [1.0; 3])
}

/// 部品の面（`part`）のうち、法線が `normal` 向きのものの添字。
fn face_of(v: &CavityViews, part: &str, normal: Vec3) -> usize {
    v.surfaces
        .iter()
        .position(|s| s.part == part && !s.wall && s.face.normal.dot(normal) > 0.99)
        .expect("face")
}

fn pair_between(v: &CavityViews, i: usize, j: usize) -> Option<FacePair> {
    v.pairs
        .iter()
        .copied()
        .find(|p| (p.a, p.b) == (i.min(j), i.max(j)))
}

/// 内部の部品が無ければ空洞は無い。容器の外に出ている部品は `outside` に残り、面を持たない。
#[test]
fn no_internal_part_means_no_cavity() {
    assert!(cavity_views(&[frame(), b("OUT", [2.0, 0.0, 0.0], [0.2; 3])]).is_none());
    let v = cavity_views(&[
        frame(),
        b("IN", [0.0; 3], [0.2; 3]),
        b("OUT", [2.0, 0.0, 0.0], [0.2; 3]),
    ])
    .unwrap();
    assert_eq!(v.containers, ["FRAME"]);
    assert_eq!(v.outside, ["OUT"]);
    assert_eq!(v.surfaces.len(), 12);
    assert!(v.surfaces.iter().all(|s| s.part != "OUT"));
}

/// 壁は内側を向く（法線が中心を向く）。内部の部品の面は外向き。
#[test]
fn walls_face_inward_and_parts_face_outward() {
    let v = cavity_views(&[frame(), b("IN", [0.1, 0.0, 0.0], [0.2; 3])]).unwrap();
    for s in &v.surfaces {
        let to_centre = if s.wall {
            Vec3::ZERO
        } else {
            Vec3::new(0.1, 0.0, 0.0)
        } - s.face.center;
        let outward = s.face.normal.dot(to_centre) < 0.0;
        assert_eq!(outward, !s.wall, "{} {:?}", s.part, s.face.normal);
    }
}

/// 閉じた空洞（壁 6 面 + 中の 2 つの箱）では、どの面からも見える割合の和が 1（接触なし。遮蔽つきの求積の誤差の範囲で）。
#[test]
fn rows_sum_to_one_in_a_closed_cavity() {
    let v = cavity_views(&[
        frame(),
        b("A", [0.0; 3], [0.2; 3]),
        b("B", [0.3, 0.2, 0.1], [0.1, 0.15, 0.1]),
    ])
    .unwrap();
    assert_eq!(v.contact_pairs, 0);
    for (i, sum) in v.row_sums.iter().enumerate() {
        assert!(
            (sum - 1.0).abs() < 0.03,
            "surface {i} ({}): {sum}",
            v.surfaces[i].part
        );
    }
}

/// 隙間 2.5 mm で積み重ねた同じ大きさの基板（90 mm × 96 mm）の向き合う面は、遮蔽がなければ閉じた式 `A · F`（Howell C-11）に一致する。
#[test]
fn stacked_boards_match_the_closed_form() {
    let v = cavity_views(&[
        frame(),
        b("P1", [0.0, 0.0, 0.0], [0.09, 0.096, 0.01]),
        b("P2", [0.0, 0.0, 0.0125], [0.09, 0.096, 0.01]),
    ])
    .unwrap();
    let (i, j) = (face_of(&v, "P1", Vec3::Z), face_of(&v, "P2", -Vec3::Z));
    let pair = pair_between(&v, i, j).expect("facing pair");
    let exact = 0.09 * 0.096 * parallel_opposed(0.09, 0.096, 0.0025);
    assert!(
        (pair.exchange / exact - 1.0).abs() < 1e-3,
        "{} {exact}",
        pair.exchange
    );
    assert!(!pair.blocked);
}

/// 2 つの基板の間に、両方を覆う板を置くと、その組は 0（`blocked`）。板を除けば正。
#[test]
fn a_plate_between_blocks_the_pair() {
    let boards = |with_plate: bool| {
        let mut boxes = vec![
            frame(),
            b("P1", [0.0, 0.0, 0.0], [0.09, 0.096, 0.01]),
            b("P2", [0.0, 0.0, 0.2], [0.09, 0.096, 0.01]),
        ];
        if with_plate {
            boxes.push(b("PLATE", [0.0, 0.0, 0.1], [0.5, 0.5, 0.01]));
        }
        cavity_views(&boxes).unwrap()
    };
    let open = boards(false);
    let (i, j) = (
        face_of(&open, "P1", Vec3::Z),
        face_of(&open, "P2", -Vec3::Z),
    );
    assert!(pair_between(&open, i, j).unwrap().exchange > 0.0);
    let shut = boards(true);
    let (i, j) = (
        face_of(&shut, "P1", Vec3::Z),
        face_of(&shut, "P2", -Vec3::Z),
    );
    let pair = pair_between(&shut, i, j).expect("kept as blocked");
    assert_eq!(pair.exchange, 0.0);
    assert!(pair.blocked);
}

/// 壁や隣の部品に面で接する面の組は除く（伝導で結ぶ）。接している面は壁にも相手にも「見えない」ので、その面の行の和は 1 に満たない。
#[test]
fn touching_faces_are_left_to_conduction() {
    // 底の壁（z = −0.5）に載る基板と、その上に重ねた基板
    let v = cavity_views(&[
        frame(),
        b("LOW", [0.0, 0.0, -0.495], [0.2, 0.2, 0.01]),
        b("HIGH", [0.0, 0.0, -0.485], [0.1, 0.1, 0.01]),
    ])
    .unwrap();
    assert!(v.contact_pairs >= 2, "{}", v.contact_pairs);
    let bottom = face_of(&v, "LOW", -Vec3::Z);
    let wall = v
        .surfaces
        .iter()
        .position(|s| s.wall && s.face.normal.dot(Vec3::Z) > 0.99)
        .unwrap();
    assert!(pair_between(&v, bottom, wall).is_none());
    assert!(v.row_sums[bottom] < 1e-9, "{}", v.row_sums[bottom]);
}

/// 容器の外の部品は、壁の近くにあっても視線を遮らない（パネルは容器の殻の外にあり、板厚の分だけ壁の内側に食い込む）。
/// 食い込むパネルがあっても、部品の面と壁の面（パネルの食い込みの中に終点がある視線）の値は変わらない。
#[test]
fn parts_outside_do_not_block() {
    let boxes = |outside: bool| {
        let mut boxes = vec![frame(), b("A", [0.3, 0.0, 0.0], [0.1; 3])];
        if outside {
            boxes.push(b("PANEL", [0.5, 0.0, 0.0], [0.01, 1.2, 1.2]));
        }
        cavity_views(&boxes).unwrap()
    };
    let (without, with) = (boxes(false), boxes(true));
    assert_eq!(with.outside, ["PANEL"]);
    let i = face_of(&without, "A", Vec3::X);
    let wall = |v: &CavityViews| {
        v.surfaces
            .iter()
            .position(|s| s.wall && s.face.normal.dot(-Vec3::X) > 0.99)
            .unwrap()
    };
    let before = pair_between(&without, i, wall(&without)).unwrap().exchange;
    let after = pair_between(&with, i, wall(&with)).unwrap().exchange;
    assert!(before > 0.0);
    assert_eq!(before, after);
}
