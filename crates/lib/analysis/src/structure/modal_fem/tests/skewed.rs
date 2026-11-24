//! 斜めのパネル（`orientation` を持つもの）。決め手は剛体の回転に対する不変性: 任意の軸まわりに任意の角度で回しても、
//! 拘束の無い（自由な）モデルの弾性の振動数は、軸に平行なモデルと同じ（要素の局所の行列を局所軸で回す部分と、節点の置き方の確認）。

use super::*;
use sat_astro::math::{Quat, Vec3};

/// 軸 (1, 2, 3)・角度 0.7 rad の回転。
fn rotation() -> Quat {
    Quat::from_axis_angle(Vec3::new(1.0, 2.0, 3.0), 0.7)
}

fn rotate(q: Quat, p: [f64; 3]) -> [f64; 3] {
    let v = q.rotate(Vec3::new(p[0], p[1], p[2]));
    [v.x, v.y, v.z]
}

/// 全体を回転 `q` で回し、`shift` だけずらす。パネルは姿勢つきの斜めのパネルになる（箱の座標系 = 元の機体座標系）。
fn rotated(i: &ModalFemInput, q: Quat, shift: [f64; 3]) -> ModalFemInput {
    let at = |p: [Qty<Length>; 3]| {
        let r = rotate(q, p.map(|x| x.value()));
        pos(std::array::from_fn(|k| r[k] + shift[k]))
    };
    let mut out = i.clone();
    for p in &mut out.panels {
        p.center = at(p.center);
        p.orientation = Some(q.to_wxyz());
    }
    for l in &mut out.lumped {
        l.position = at(l.position);
    }
    out
}

/// 弾性のモード（剛体でない）の振動数。
fn elastic(o: &ModalFemOutput) -> Vec<f64> {
    o.modes
        .iter()
        .filter(|m| m.class != ModalFemModeClass::Rigid)
        .map(hz)
        .collect()
}

fn same(a: &[f64], b: &[f64], tol: f64) {
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b) {
        assert!(rel(*x, *y) < tol, "{x} vs {y}");
    }
}

/// 自由な板（0.2 × 0.3 × 2 mm、集中質量つき）。
fn free_plate() -> ModalFemInput {
    let mut i = input(
        vec![panel(
            "P",
            ModalFemAxis::Z,
            [0.1, -0.05, 0.3],
            [0.2, 0.3],
            0.002,
            0.33,
        )],
        ModalFemSupport::Free,
    );
    i.lumped = vec![lumped("M", [0.15, 0.0, 0.3], 0.2)];
    i.mode_count = Some(12);
    i
}

/// 板を任意の軸・角度で回しても、弾性の振動数は同じ（許容差 1e-7: 節点の座標の丸めと反復の収束の許容値 1e-10。ほかの不変性のテストと同じ）。
/// 6 つの剛体のモードは 0 付近で、これは分類が剛体にする。
#[test]
fn rotated_plate_has_the_same_frequencies() {
    let base = solve(&free_plate());
    assert_eq!(base.rigid_body_modes, 6);
    let turned = solve(&rotated(&free_plate(), rotation(), [0.3, -0.2, 0.1]));
    assert_eq!(turned.rigid_body_modes, 6);
    same(&elastic(&base), &elastic(&turned), 1e-7);
    assert_eq!(turned.skewed_panels.len(), 1);
    assert!(rel(turned.total_mass.value(), base.total_mass.value()) < 1e-12);
}

/// 箱（6 枚のパネル、斜めに回したもの）も同じ。パネルの辺で格子が合うので、節点を共有して箱としてつながる（4 辺ともつながる）。
/// 質量の位置（集中質量）も回す。
#[test]
fn rotated_box_has_the_same_frequencies_and_joints() {
    let mut i = input(box_panels(0.1, 0.2, 0.3, 0.0015), ModalFemSupport::Free);
    i.mesh_size = Some(len(0.05));
    i.lumped = vec![
        lumped("A", [0.02, 0.05, 0.10], 0.4),
        lumped("B", [-0.03, -0.06, -0.05], 0.7),
    ];
    i.mode_count = Some(14);
    let base = solve(&i);
    let turned_input = rotated(&i, rotation(), [0.05, 0.1, -0.2]);
    let turned = solve(&turned_input);
    same(&elastic(&base), &elastic(&turned), 1e-7);
    assert_eq!(base.model.nodes, turned.model.nodes);
    assert_eq!(base.model.elements, turned.model.elements);
    let shared = |o: &ModalFemOutput| {
        let mut v: Vec<u32> = o.joints.iter().map(|j| j.shared_nodes).collect();
        v.sort_unstable();
        v
    };
    assert_eq!(shared(&base), shared(&turned));
    assert_eq!(turned.skewed_panels.len(), 6);
    assert!(
        turned
            .skewed_panels
            .iter()
            .all(|p| p.joined_edges == 4 && p.unjoined_edges == 0)
    );
}

/// 軸に平行なパネルと斜めのパネル（恒等の姿勢 = 実質は平行）が混ざっても、軸に平行なだけのモデルと同じ
/// （斜めのパネルの格子が共通の格子と合って節点を共有し、並べ直した節点の番号でも解が同じ）。
#[test]
fn skewed_and_aligned_panels_join_where_the_grids_match() {
    let mut i = input(
        box_panels(0.1, 0.2, 0.3, 0.0015),
        clamp_face(ModalFemFace::MinusZ),
    );
    i.mesh_size = Some(len(0.05));
    i.lumped = vec![
        lumped("A", [0.02, 0.05, 0.10], 0.4),
        lumped("B", [-0.03, -0.06, -0.05], 0.7),
    ];
    let base = solve(&i);
    let mut mixed = i.clone();
    for p in &mut mixed.panels {
        if p.id == "PX" || p.id == "NY" {
            p.orientation = Some([1.0, 0.0, 0.0, 0.0]);
        }
    }
    let o = solve(&mixed);
    same(
        &base.modes.iter().map(hz).collect::<Vec<_>>(),
        &o.modes.iter().map(hz).collect::<Vec<_>>(),
        1e-7,
    );
    assert_eq!(base.model.nodes, o.model.nodes);
    assert_eq!(o.skewed_panels.len(), 2);
    assert!(o.skewed_panels.iter().all(|p| p.joined_edges == 4));
}

/// つながっていない斜めのパネルは、つながりを辺ごとに出力に出し（自由な辺）、剛体のモードになる（黙ってつながったことにしない）。
/// 箱の上面（z = 0.15）の辺から +y の面に沿って立てた板（恒等の姿勢）は、上面と共有する 1 辺だけが「つながった」になる。
/// 立てた板の両側の辺は、根元の 1 節点（角）だけを共有するので「一部」で、先の辺は自由（つながらない）。
#[test]
fn unjoined_edges_are_reported() {
    let mut i = input(
        box_panels(0.1, 0.2, 0.3, 0.0015),
        clamp_face(ModalFemFace::MinusZ),
    );
    i.mesh_size = Some(len(0.05));
    let mut far = panel(
        "FAR",
        ModalFemAxis::Z,
        [0.5, 0.5, 0.5],
        [0.1, 0.1],
        0.002,
        0.33,
    );
    far.orientation = Some(rotation().to_wxyz());
    i.panels.push(far);
    i.mode_count = Some(8);
    let o = solve(&i);
    let far = &o.skewed_panels[0];
    assert_eq!(
        (far.id.as_str(), far.joined_edges, far.unjoined_edges),
        ("FAR", 0, 4)
    );
    assert_eq!(o.rigid_body_modes, 6, "a hanging panel is a mechanism");

    let mut i = input(
        box_panels(0.1, 0.2, 0.3, 0.0015),
        clamp_face(ModalFemFace::MinusZ),
    );
    i.mesh_size = Some(len(0.05));
    let mut flap = panel(
        "FLAP",
        ModalFemAxis::Y,
        [0.0, 0.1, 0.25],
        [0.1, 0.2],
        0.002,
        0.33,
    );
    flap.orientation = Some([1.0, 0.0, 0.0, 0.0]);
    i.panels.push(flap);
    let o = solve(&i);
    let flap = o
        .skewed_panels
        .iter()
        .find(|p| p.id == "FLAP")
        .expect("reported");
    assert_eq!(
        (
            flap.joined_edges,
            flap.partly_joined_edges,
            flap.unjoined_edges
        ),
        (1, 2, 1)
    );
    assert_eq!(o.rigid_body_modes, 0);
}
