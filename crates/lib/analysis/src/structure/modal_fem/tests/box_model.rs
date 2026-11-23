//! 箱形のモデル: パネルのつながり、剛体のモードの数、拘束の単調性、質量の総和、集中質量の取り付け先、ドリリングの倍率。

use super::*;
use ModalFemFace::*;

const T: f64 = 0.0015;
const MESH: f64 = 0.05;

/// 0.1 × 0.2 × 0.3 m の箱（板厚 1.5 mm）と、位置の違う 3 つの集中質量。要素 50 mm（x 2・y 4・z 6 分割）。
fn small_box(support: ModalFemSupport) -> ModalFemInput {
    let mut i = input(box_panels(0.1, 0.2, 0.3, T), support);
    i.mesh_size = Some(len(MESH));
    i.lumped = vec![
        lumped("A", [0.02, 0.05, 0.10], 0.4),
        lumped("B", [-0.03, -0.06, -0.05], 0.7),
        lumped("C", [0.0, 0.08, 0.12], 0.2),
    ];
    i
}

fn freqs(o: &ModalFemOutput) -> Vec<f64> {
    o.modes.iter().map(hz).collect()
}

/// 閉じた箱の 6 枚のパネルは 12 本の辺で接合する（向かい合うパネルは接合しない）。1 つの辺で共有する節点は z 方向 7 点。
#[test]
fn closed_box_joins_twelve_edges() {
    let o = solve(&small_box(ModalFemSupport::Free));
    assert_eq!(o.joints.len(), 12);
    let pair = |a: &str, b: &str| {
        o.joints
            .iter()
            .find(|j| (j.panel_a == a && j.panel_b == b) || (j.panel_a == b && j.panel_b == a))
    };
    assert!(pair("PX", "NX").is_none() && pair("PY", "NY").is_none() && pair("PZ", "NZ").is_none());
    assert_eq!(
        pair("PX", "PY").map(|j| j.shared_nodes),
        Some(7),
        "the edge along z has 7 nodes"
    );
    assert_eq!(
        pair("PX", "PZ").map(|j| j.shared_nodes),
        Some(5),
        "the edge along y has 5 nodes"
    );
    assert_eq!(
        pair("PY", "PZ").map(|j| j.shared_nodes),
        Some(3),
        "the edge along x has 3 nodes"
    );
    // 節点の数: 格子 3 × 5 × 7 の表面の節点 = 105 − 内部 1 × 3 × 5 = 90。
    assert_eq!(o.model.nodes, 90);
    assert_eq!(o.model.dofs, 540);
}

/// 棚（法線 z の板）が壁に当たる T 字の接合: 棚は 4 枚の壁に接合する（12 + 4 = 16）。
/// 棚の位置の格子線を壁も持つので、節点が一致する。
#[test]
fn shelf_joins_the_walls_at_a_t_junction() {
    let mut i = small_box(ModalFemSupport::Free);
    i.panels.push(panel(
        "SH",
        ModalFemAxis::Z,
        [0.0, 0.0, 0.05],
        [0.1, 0.2],
        T,
        0.33,
    ));
    let o = solve(&i);
    assert_eq!(o.joints.len(), 16);
    let shelf: Vec<&ModalFemJoint> = o
        .joints
        .iter()
        .filter(|j| j.panel_a == "SH" || j.panel_b == "SH")
        .collect();
    assert_eq!(shelf.len(), 4);
    assert!(
        shelf
            .iter()
            .all(|j| ["PX", "NX", "PY", "NY"].contains(&other(j, "SH")))
    );
}

fn other<'a>(j: &'a ModalFemJoint, id: &str) -> &'a str {
    if j.panel_a == id {
        &j.panel_b
    } else {
        &j.panel_a
    }
}

/// 離れた 2 枚の板は接合しない。辺が板厚の半分ほどずれていても、許容差（既定 1 mm）の内なら接合し、許容差 0 なら接合しない。
#[test]
fn joints_follow_the_tolerance() {
    let apart = input(
        vec![
            panel("A", ModalFemAxis::Z, [0.0; 3], [0.1, 0.1], T, 0.3),
            panel("B", ModalFemAxis::Z, [0.0, 0.0, 0.1], [0.1, 0.1], T, 0.3),
        ],
        ModalFemSupport::Free,
    );
    assert!(solve(&apart).joints.is_empty());
    let mut i = small_box(ModalFemSupport::Free);
    i.panels[0].center[0] = len(0.05 + 0.0007);
    assert_eq!(solve(&i).joints.len(), 12, "within 1 mm");
    i.merge_tolerance = Some(len(0.0));
    assert!(
        solve(&i).joints.len() < 12,
        "with no tolerance the offset panel is not joined"
    );
}

/// 剛体のモードの数 = 拘束で止まらない動きの数: 拘束なし 6、1 点（並進）3、2 点 1（2 点を結ぶ線のまわり）、一直線でない 3 点 0。
#[test]
fn rigid_body_mode_counts() {
    let point = |p: [f64; 3]| ModalFemFixedItem {
        target: ModalFemTarget::Point { position: pos(p) },
        clamp: false,
    };
    let corners = [
        [0.05, 0.1, -0.15],
        [-0.05, 0.1, -0.15],
        [-0.05, -0.1, -0.15],
    ];
    let rigid = |n: usize| {
        let items = corners[..n].iter().map(|c| point(*c)).collect();
        let mut i = small_box(if n == 0 {
            ModalFemSupport::Free
        } else {
            ModalFemSupport::Fixed { items }
        });
        i.mode_count = Some(8);
        let o = solve(&i);
        assert!(
            o.modes
                .iter()
                .filter(|m| m.class != ModalFemModeClass::Rigid)
                .all(|m| hz(m) > 1.0)
        );
        o.rigid_body_modes
    };
    assert_eq!([rigid(0), rigid(1), rigid(2), rigid(3)], [6, 3, 1, 0]);
}

/// 拘束を増やすと、同じ番号のモードの振動数は下がらない（固有値の最小最大原理）:
/// 拘束なし ≤ 底面の 4 つの角の並進の固定 ≤ 底面（面）の全自由度の固定。10 モードで比べる。
/// 許容差は数値の丸めだけ（相対 1e-9）。
#[test]
fn more_supports_never_lower_a_frequency() {
    let corners = [[0.05, 0.1], [-0.05, 0.1], [-0.05, -0.1], [0.05, -0.1]];
    let points = ModalFemSupport::Fixed {
        items: corners
            .iter()
            .map(|c| ModalFemFixedItem {
                target: ModalFemTarget::Point {
                    position: pos([c[0], c[1], -0.15]),
                },
                clamp: false,
            })
            .collect(),
    };
    let run = |s: ModalFemSupport| {
        let mut i = small_box(s);
        i.mode_count = Some(10);
        freqs(&solve(&i))
    };
    let (free, four, base) = (
        run(ModalFemSupport::Free),
        run(points),
        run(clamp_face(MinusZ)),
    );
    println!("free {free:?}\nfour {four:?}\nbase {base:?}");
    for k in 0..10 {
        assert!(four[k] >= free[k] * (1.0 - 1e-9) - 1e-6, "mode {k}");
        assert!(base[k] >= four[k] * (1.0 - 1e-9) - 1e-6, "mode {k}");
    }
    assert!(base[0] > four[0] && four[0] > free[0]);
}

/// レールの拘束が固定する自由度の数: 機軸（z）に平行な 4 本の辺（各 7 節点）の横 2 方向 + 8 つの角の機軸方向 = 4·7·2 + 8 = 64。
/// タブ（面 +x の、機軸に平行な 2 本の辺）は 2·7·3 = 42。どちらも剛体のモードが無い（レールは）。
#[test]
fn rails_and_tabs_select_the_expected_dofs() {
    let rails = solve(&small_box(ModalFemSupport::Rails));
    assert_eq!(rails.model.fixed_dofs, 64);
    assert_eq!(rails.rigid_body_modes, 0);
    let tabs = solve(&small_box(ModalFemSupport::Tabs { face: PlusX }));
    assert_eq!(tabs.model.fixed_dofs, 42);
    let mut bad = small_box(ModalFemSupport::Tabs { face: PlusZ });
    assert!(
        ModalFem::run(&bad).is_err(),
        "a face normal to the launch axis has no edge parallel to it"
    );
    bad.support = ModalFemSupport::BaseFixed { face: PlusZ };
    assert!(ModalFem::run(&bad).is_ok());
}

/// モデルの全質量 = 入力の質量の和（パネル + 集中質量 + 分布質量）。パネルの `mass` を与えた分は、密度 × 板厚 × 面積の代わりにその値。
#[test]
fn total_mass_is_the_sum_of_the_inputs() {
    let mut i = small_box(ModalFemSupport::Free);
    i.distributed_mass = Some(Qty::from_raw(0.35));
    let area = 2.0 * (0.1 * 0.2 + 0.1 * 0.3 + 0.2 * 0.3);
    let expected = RHO_AL * T * area + 0.4 + 0.7 + 0.2 + 0.35;
    let m = solve(&i).total_mass.value();
    assert!(rel(m, expected) < 1e-12, "{m} vs {expected}");
    i.panels[0].mass = Some(Qty::from_raw(0.123));
    let expected2 = expected - RHO_AL * T * 0.2 * 0.3 + 0.123;
    let m2 = solve(&i).total_mass.value();
    assert!(rel(m2, expected2) < 1e-12, "{m2} vs {expected2}");
}

/// パネルの `mass` は質量行列だけを変える: 全パネルの `mass` を密度で出した値の 2 倍にするのは、密度を 2 倍にするのと同じ（剛性は密度に依らない）。
#[test]
fn explicit_panel_mass_changes_only_the_mass() {
    let area = |p: &ModalFemPanel| {
        let s = p.size.map(|x| x.value());
        s[0] * s[1]
    };
    let mut by_mass = small_box(ModalFemSupport::Free);
    for p in &mut by_mass.panels {
        p.mass = Some(Qty::from_raw(2.0 * RHO_AL * T * area(p)));
    }
    let mut by_density = small_box(ModalFemSupport::Free);
    for p in &mut by_density.panels {
        p.density = Qty::from_raw(2.0 * RHO_AL);
    }
    let (a, b) = (freqs(&solve(&by_mass)), freqs(&solve(&by_density)));
    for (x, y) in a.iter().zip(&b) {
        assert!((x - y).abs() < 1e-6 + 1e-8 * y, "{x} vs {y}");
    }
}

/// 集中質量の取り付け先: 名前の指定がなければ最も近いパネルで、距離を出力する。名前を指定するとそのパネル。
#[test]
fn lumped_mass_attachment_is_reported() {
    let mut i = small_box(ModalFemSupport::Free);
    i.lumped = vec![
        lumped("NEAR_TOP", [0.01, 0.02, 0.14], 0.3),
        lumped("NAMED", [0.0, 0.0, 0.0], 0.3),
    ];
    i.lumped[1].panel = Some("NX".to_owned());
    let o = solve(&i);
    assert_eq!(o.attachments[0].panel, "PZ");
    assert!(o.attachments[0].automatic);
    assert!(rel(o.attachments[0].distance.value(), 0.01) < 1e-9);
    assert_eq!(o.attachments[1].panel, "NX");
    assert!(!o.attachments[1].automatic);
    assert!(rel(o.attachments[1].distance.value(), 0.05) < 1e-9);
    i.lumped[1].panel = Some("NOPE".to_owned());
    assert!(ModalFem::run(&i).is_err());
}

/// ドリリングの倍率を 10 分の 1 と 10 倍にしても、固定した底面の箱の下位 6 モードの振動数はほとんど変わらない。
/// 許容差 1 %（実測前に決めた値: ドリリングは接合部のねじりの伝わりだけに効く）。
#[test]
fn drilling_factor_is_insensitive() {
    let run = |f: f64| {
        let mut i = small_box(clamp_face(MinusZ));
        i.drilling_factor = Some(f);
        freqs(&solve(&i))
    };
    let (low, mid, high) = (
        run(DRILLING_FACTOR / 10.0),
        run(DRILLING_FACTOR),
        run(DRILLING_FACTOR * 10.0),
    );
    let worst = |a: &[f64]| {
        a.iter()
            .zip(&mid)
            .map(|(x, y)| rel(*x, *y))
            .fold(0.0, f64::max)
    };
    println!(
        "mid {mid:?}\nworst change: x0.1 {:e}, x10 {:e}",
        worst(&low),
        worst(&high)
    );
    assert!(worst(&low) < 0.01 && worst(&high) < 0.01);
}

/// 入力の誤りは受け付けない。
#[test]
fn invalid_inputs_are_rejected() {
    let mut i = small_box(ModalFemSupport::Free);
    i.panels[0].poisson_ratio = 0.6;
    assert!(ModalFem::run(&i).is_err());
    let mut i = small_box(ModalFemSupport::Free);
    i.mode_count = Some(0);
    assert!(ModalFem::run(&i).is_err());
    let mut i = small_box(ModalFemSupport::Free);
    i.panels[1].id = "PX".to_owned();
    assert!(ModalFem::run(&i).is_err(), "duplicate ids");
    let mut i = small_box(ModalFemSupport::Free);
    i.panels.push(panel(
        "DUP",
        ModalFemAxis::X,
        [0.05, 0.0, 0.0],
        [0.2, 0.3],
        T,
        0.33,
    ));
    assert!(ModalFem::run(&i).is_err(), "overlapping panels");
}
