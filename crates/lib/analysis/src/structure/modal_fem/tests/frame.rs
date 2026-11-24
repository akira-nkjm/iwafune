//! フレームの部材（箱の辺の梁）。レールだけの片持ち梁を梁の理論と比べ、質量の保存・取り付けの規則（1 次モーメントの保存）・剛性が入る向きを確かめる。

use super::*;
use crate::structure::modal_estimate::{ModalEstimateBeam, beam_frequencies_hz};
use frame_tie::tie;
use sat_units::typed::Pressure;

/// アルミのフレーム（箱の辺の部材）。
fn frame(
    center: [f64; 3],
    size: [f64; 3],
    width: f64,
    wall: Option<f64>,
    kg: f64,
) -> ModalFemFrame {
    ModalFemFrame {
        id: "F".to_owned(),
        center: pos(center),
        size: size.map(len),
        youngs_modulus: Qty::<Pressure>::from_raw(E_AL),
        poisson_ratio: 0.33,
        section: ModalFemMemberSection {
            width: len(width),
            wall: wall.map(len),
        },
        mass: Qty::<Mass>::from_raw(kg),
    }
}

/// 2 本のレール（z 方向、x = ±w/2、y = w/2）を、ごく薄いウェブ（y = w/2 の面）でつないだ長さ L の片持ち梁。z = 0 を固定。
/// フレームの箱の辺のうち節点が並ぶのはウェブの 2 本の辺だけ（残りの 10 本は辺に沿ったパネルが無いので部材にならない）。
fn rails_model(w: f64, l: f64, rail: f64, kg: f64) -> ModalFemInput {
    let mut web = panel(
        "WEB",
        ModalFemAxis::Y,
        [0.0, w / 2.0, l / 2.0],
        [w, l],
        1.0e-6,
        0.33,
    );
    web.mass = Some(Qty::from_raw(1.0e-6));
    let mut i = input(vec![web], clamp_face(ModalFemFace::MinusZ));
    i.mesh_size = Some(len(0.01));
    // 箱はウェブより z 方向に 10 mm ずつ長い（ウェブの上下の縁が箱の辺の上に乗って、x 方向の部材ができないように）
    i.frames = vec![frame([0.0, 0.0, l / 2.0], [w, w, l + 0.02], rail, None, kg)];
    i
}

/// 梁の理論: レール 1 本は、質量 M/2・曲げ剛性 EI の片持ち梁（ウェブは剛性・質量ともに無視できる）。
/// L = 0.4 m、ウェブの幅 10 mm、レールの断面 6 mm 角（中実）。許容差 1e-3: 要素 10 mm（40 要素）の集中質量の離散化誤差
/// （相対 0.13 (β₁/n)² ≈ 3e-4、`beam_element` の収束で確認）、ウェブの曲げ剛性（レールの 4e-4 倍 → 振動数で 2e-4）、ウェブの質量（2e-5）の見込み。
#[test]
fn rails_cantilever_matches_beam_theory() {
    let (w, l, a) = (0.01, 0.4, 0.006);
    let section = ModalFemMemberSection {
        width: len(a),
        wall: None,
    }
    .properties()
    .expect("a section");
    let kg = 2.0 * RHO_AL * section.area * l;
    let mut i = rails_model(w, l, a, kg);
    // 近い 4 つのモード（2 本のレール × 2 方向）が重根に近いので、求める数は 4（5 つ目以降はウェブの局所のモードで、反復が遅い）
    i.mode_count = Some(4);
    let (reference, _) = beam_frequencies_hz(&ModalEstimateBeam {
        id: "R".to_owned(),
        length: len(l),
        bending_stiffness: Qty::from_raw(E_AL * section.second_moment),
        axial_stiffness: None,
        mass: Qty::from_raw(kg / 2.0),
        tip_mass: None,
    })
    .expect("beam");
    let o = solve(&i);
    let f = hz(&o.modes[0]);
    println!("rails: fem {f}, beam {reference}");

    assert!(rel(f, reference) < 1.0e-3, "{f} vs {reference}");
    let model = &o.frames[0];
    assert_eq!((model.edges, model.unmodelled_edges.len()), (2, 10));
    assert_eq!(model.members, 2 * 40);
    assert!(rel(model.mass.value(), kg) < 1e-12);
    assert!(rel(o.total_mass.value(), kg + 1.0e-6) < 1e-12);
    // 軸（z）・横（x, y）の入れ替えで同じ（部材の局所軸から機体の軸への写しが、どの軸の部材でも正しい）
    for perm in [[2, 0, 1], [1, 2, 0]] {
        let g = solve(&remap(&i, perm, [1.0; 3], [0.0; 3]));
        assert!(
            rel(hz(&g.modes[0]), f) < 1e-7,
            "{perm:?}: {} vs {f}",
            hz(&g.modes[0])
        );
    }
}

/// 質量が保たれる: パネル + フレーム + 集中質量 + 分布質量 = 全質量（フレームは部材にだけ置く）。
fn box_with_frame() -> ModalFemInput {
    let mut i = input(
        box_panels(0.1, 0.2, 0.3, 0.0015),
        clamp_face(ModalFemFace::MinusZ),
    );
    i.mesh_size = Some(len(0.05));
    i.frames = vec![frame([0.0; 3], [0.1, 0.2, 0.3], 0.006, Some(0.001), 0.4)];
    i.distributed_mass = Some(Qty::from_raw(0.15));
    let mut a = lumped("A", [0.0, 0.06, 0.05], 0.5);
    a.frame = Some("F".to_owned());
    a.footprint = Some([0.09, 0.09, 0.04].map(len));
    let mut b = lumped("B", [0.02, -0.06, -0.1], 0.3);
    b.frame = Some("F".to_owned());
    i.lumped = vec![a, b];
    i
}

#[test]
fn total_mass_is_conserved_with_members() {
    let i = box_with_frame();
    let o = solve(&i);
    let panels: f64 = i
        .panels
        .iter()
        .map(|p| RHO_AL * p.thickness.value() * p.size[0].value() * p.size[1].value())
        .sum();
    let expected = panels + 0.4 + 0.5 + 0.3 + 0.15;
    assert!(
        rel(o.total_mass.value(), expected) < 1e-12,
        "{} vs {expected}",
        o.total_mass.value()
    );
    let f = &o.frames[0];
    assert_eq!((f.edges, f.unmodelled_edges.len()), (12, 0));
    assert!(rel(f.mass.value(), 0.4) < 1e-12);
    // 部品の取り付けの割合は、部品ごとに 1 になる
    for id in ["A", "B"] {
        let total: f64 = o
            .attachments
            .iter()
            .filter(|a| a.id == id)
            .map(|a| a.share.value())
            .sum();
        assert!((total - 1.0).abs() < 1e-12, "{id}: {total}");
    }
    assert!(
        o.attachments
            .iter()
            .all(|a| a.panel == "F" && a.edge.is_some() && a.automatic)
    );
}

/// 部材が 1 本も無いフレーム（箱の辺に節点が無い）は、質量を全体に配って保存し、辺を出力に出す。
#[test]
fn frame_without_member_nodes_spreads_its_mass() {
    let mut i = input(
        vec![panel(
            "P",
            ModalFemAxis::Z,
            [0.0; 3],
            [0.2, 0.2],
            0.002,
            0.33,
        )],
        clamp_face(ModalFemFace::MinusX),
    );
    i.frames = vec![frame([0.0; 3], [0.5, 0.5, 0.5], 0.006, None, 0.2)];
    let o = solve(&i);
    let expected = RHO_AL * 0.002 * 0.04 + 0.2;
    assert!(rel(o.total_mass.value(), expected) < 1e-12);
    let f = &o.frames[0];
    assert_eq!((f.members, f.edges, f.unmodelled_edges.len()), (0, 0, 12));
    assert_eq!(f.mass.value(), 0.0);
    // 部材の無いフレームに付く部品は、最も近いパネルにつなぐ
    let mut l = lumped("A", [0.0, 0.0, 0.1], 0.1);
    l.frame = Some("F".to_owned());
    i.lumped = vec![l];
    let o = solve(&i);
    assert_eq!(o.attachments.len(), 1);
    assert_eq!(
        (
            o.attachments[0].panel.as_str(),
            o.attachments[0].edge.as_deref()
        ),
        ("P", None)
    );
}

/// つなぎ先の優先: 名前で指したパネルが、フレームより先。
#[test]
fn a_named_panel_beats_the_frame() {
    let mut i = box_with_frame();
    i.lumped[0].panel = Some("PX".to_owned());
    let o = solve(&i);
    let rows: Vec<_> = o.attachments.iter().filter(|a| a.id == "A").collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (
            rows[0].panel.as_str(),
            rows[0].automatic,
            rows[0].edge.as_deref()
        ),
        ("PX", false, None)
    );
}

/// 取り付けの規則（d₁ + d₂ + 線分までの距離が最小の 2 辺に、2 点を結ぶ線分への投影で配る）で、質量の合計と 1 次モーメントが保たれる。
/// 箱は 0.1 × 0.2 × 0.3 m（原点が中心）で、質量は 2 kg。手計算（辺の最も近い点 q と距離 d）:
/// - p = (0.05, 0.04, 0)（+x の面の上）: z 方向の辺 (x, y) = (0.05, 0.1)（d = 0.06）と (0.05, −0.1)（0.14）の組が 0.2 + 0（p は線分 (0.05, ±0.1, 0) の上）で最小
///   （(0.05, 0.1) と (−0.05, 0.1) の組は 0.06 + 0.117 + 0.06 = 0.237、y 方向の辺 (x, z) = (0.05, ±0.15) の組は 0.3）。重心は p のまま（s = 0.3）。
/// - p = (0.01, 0.04, 0): (0.05, 0.1) と (−0.05, 0.1) の組が 0.0721 + 0.0849 + 0.06 = 0.217 で最小（(0.05, 0.1) と (0.05, −0.1) は 0.258）。
///   x 方向の線分 (±0.05, 0.1, 0) に投影されるので、y のずれ（0.04 → 0.1）は落ち、重心は (0.01, 0.1, 0)（s = 0.4）。
/// - p = 原点: 対角の 2 辺（2 × 0.1118 = 0.2236 + 0）。p は線分の上なので、重心は原点（s = 0.5）。
#[test]
fn tie_preserves_mass_and_first_moment() {
    let i = box_with_frame();
    let mesh = mesh::build(&i).expect("mesh");
    let moment = |p: [f64; 3]| {
        let mut out = vec![0.0; mesh.node_count()];
        let mut l = lumped("X", p, 2.0);
        l.frame = Some("F".to_owned());
        let rows = tie(&mesh, 0, &l, p, &mut out);
        let total: f64 = out.iter().sum();
        let m: [f64; 3] =
            std::array::from_fn(|k| out.iter().zip(&mesh.coords).map(|(w, c)| w * c[k]).sum());
        let shares: f64 = rows.iter().map(|r| r.share).sum();
        (total, m, shares)
    };
    for (p, expected) in [
        ([0.05, 0.04, 0.0], [0.05, 0.04, 0.0]),
        ([0.01, 0.04, 0.0], [0.01, 0.1, 0.0]),
        ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
    ] {
        let (total, m, shares) = moment(p);
        assert!((total - 2.0).abs() < 1e-12 && (shares - 1.0).abs() < 1e-12);
        for k in 0..3 {
            assert!(
                (m[k] - 2.0 * expected[k]).abs() < 1e-12,
                "{p:?} axis {k}: {} vs {}",
                m[k],
                2.0 * expected[k]
            );
        }
    }
}

/// フレームに剛性が入ると、同じ質量の分布（部材に配った質量）のまま、横の一次が上がる: 断面を太くすると振動数が上がり、
/// 部材の剛性が無い（断面が無視できる）ときの値に近づく。
#[test]
fn members_stiffen_the_model() {
    let lateral = |width: f64| {
        let mut i = box_with_frame();
        for panel in &mut i.panels {
            panel.thickness = len(0.0005);
        }
        i.frames[0].section.width = len(width);
        i.frames[0].section.wall = None;
        i.lumped.clear();
        i.distributed_mass = None;
        solve(&i).lowest_lateral.expect("a lateral mode").value()
    };
    let (thin, mid, thick) = (lateral(0.0005), lateral(0.004), lateral(0.008));
    println!("lateral: {thin} {mid} {thick}");
    assert!(thin < mid && mid < thick);
}

/// 不正なフレームの入力は拒否する: 知らないフレームへの取り付け・幅が 0 の断面・肉厚が幅の半分以上・フレームの ID の重複。
#[test]
fn invalid_frames_are_rejected() {
    let ok = box_with_frame();
    let mut unknown = ok.clone();
    unknown.lumped[0].frame = Some("NOPE".to_owned());
    let mut flat = ok.clone();
    flat.frames[0].section.width = len(0.0);
    let mut thick = ok.clone();
    thick.frames[0].section.wall = Some(len(0.003));
    let mut twice = ok.clone();
    twice.frames.push(ok.frames[0].clone());
    for (name, i) in [
        ("unknown", unknown),
        ("flat", flat),
        ("thick", thick),
        ("twice", twice),
    ] {
        assert!(ModalFem::run(&i).is_err(), "{name}");
    }
}
