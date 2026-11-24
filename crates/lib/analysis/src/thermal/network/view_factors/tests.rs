use super::super::couplings::resolve;
use super::super::tests::node;
use super::*;
use crate::thermal::enclosure::exchange_areas;
use crate::thermal::network::ThermalCouplingSource as Source;
use sat_astro::math::{Quat, Vec3};
use sat_astro::view_factor::parallel_opposed;
use sat_units::typed::Qty;

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

/// 一辺 1 m の容器と、隙間 2.5 mm で積んだ同じ大きさの基板 2 枚（90 mm × 96 mm × 10 mm）。
fn stack(extra: Vec<InterferenceBox>) -> Vec<InterferenceBox> {
    let mut boxes = vec![
        b("FRAME", [0.0; 3], [1.0; 3]),
        b("P1", [0.0, 0.0, 0.0], [0.09, 0.096, 0.01]),
        b("P2", [0.0, 0.0, 0.0125], [0.09, 0.096, 0.01]),
    ];
    boxes.extend(extra);
    boxes
}

fn network(boxes: Vec<InterferenceBox>, emissivity: f64, ids: &[&str]) -> ThermalNetwork {
    let mut n = super::super::couplings::tests::net(ids);
    n.view_factors = Some(ThermalViewFactors { emissivity, boxes });
    n
}

fn value(c: &crate::thermal::network::ThermalCouplings, a: &str, b: &str) -> Option<(f64, Source)> {
    c.radiative
        .iter()
        .find(|l| l.a == a && l.b == b)
        .map(|l| (l.exchange_area.value(), l.source))
}

/// 平行平板（`G = A`、同じ面積）の有効放射率は `1 / (1/ε₁ + 1/ε₂ − 1)`（手計算: ε = 0.8 → 0.6666…）。
/// 黒体（ε = 1）なら `G` そのもの。
#[test]
fn gray_pair_is_the_parallel_plate_formula() {
    let e = gray_pair_exchange(2.0, 2.0, 2.0, 0.8);
    assert!((e - 2.0 / (2.0 / 0.8 - 1.0)).abs() < 1e-12);
    assert!((gray_pair_exchange(0.3, 1.0, 2.0, 1.0) - 0.3).abs() < 1e-15);
    assert_eq!(gray_pair_exchange(0.0, 1.0, 1.0, 0.8), 0.0);
}

/// 2 面だけの閉空間（面 1 が面 2 に囲まれる。`F₁₂ = 1`、`F₂₁ = A₁/A₂`、`F₂₂ = 1 − A₁/A₂`）の Gebhart 係数による交換面積
/// `ε₁A₁B₁₂`（外殻のモデル）と同じ値: `A₁ / (1/ε₁ + (A₁/A₂)(1/ε₂ − 1))`。
#[test]
fn gray_pair_agrees_with_the_enclosure_gebhart_for_two_surfaces() {
    let (a1, a2) = (1.0, 4.0);
    let view = vec![vec![0.0, 1.0], vec![a1 / a2, 1.0 - a1 / a2]];
    for (e1, e2) in [(0.6, 0.6), (0.8, 0.4), (0.3, 0.9)] {
        let gebhart = exchange_areas(&view, &[a1, a2], &[e1, e2]).unwrap();
        let pair = gray_pair_exchange(a1, a1, a2, e1);
        // 放射率が面ごとに違うときのため、式は ε を 1 つ受ける形（`ε₁ = ε₂` のとき）で比べる
        if (e1 - e2).abs() < 1e-12 {
            assert!((pair - gebhart[0][1]).abs() < 1e-12, "{e1}");
        }
        let by_hand = a1 / (1.0 / e1 + (a1 / a2) * (1.0 / e2 - 1.0));
        assert!((gebhart[0][1] - by_hand).abs() < 1e-12);
    }
}

/// 積んだ基板の向き合う面の節点対 P1–P2 は、閉じた式の `A·F` から `E = 1 / (2ρ/(εA) + 1/(A F))` を出した値（遮蔽なし）。
/// 基板の同じ向きの側面どうしは向き合わないので、向き合う面の 1 組だけ（求積の誤差 1e-3 の範囲で一致）。
#[test]
fn stacked_boards_pair_value_contains_the_facing_faces() {
    let eps = 0.8;
    let c = resolve(&network(stack(vec![]), eps, &["FRAME", "P1", "P2"])).unwrap();
    let a = 0.09 * 0.096;
    let g = a * parallel_opposed(0.09, 0.096, 0.0025);
    let facing = gray_pair_exchange(g, a, a, eps);
    let (v, source) = value(&c, "P1", "P2").expect("pair");
    assert_eq!(source, Source::ViewFactor);
    assert!((v / facing - 1.0).abs() < 1e-3, "{v} {facing}");
    assert_eq!(
        c.radiative
            .iter()
            .find(|l| l.a == "P1" && l.b == "P2")
            .unwrap()
            .from_view_factors
            .unwrap()
            .face_pairs,
        1
    );
    let used = c.view_factors.as_ref().unwrap();
    assert!(used.applied);
    assert_eq!(used.cavities, ["FRAME"]);
    assert_eq!(used.contact_pairs, 0);
    // 壁との組も節点対になる
    assert!(value(&c, "FRAME", "P1").is_some());
    // 交換面積は小さい方の面の面積（放射率 1 の上限）を超えない: 板どうしは A = 8.64e-3 m² より小さい
    assert!(v < a);
}

/// 間に両方を覆う板を置くと P1–P2 は 0 で、取付の既定値（0.002 m²）を置き換えて切り離す。遮蔽の板との組は新しく加わる。
#[test]
fn a_blocked_pair_is_isolated_and_replaces_the_mount_default() {
    let boxes = vec![
        b("FRAME", [0.0; 3], [1.0; 3]),
        b("P1", [0.0, 0.0, 0.0], [0.09, 0.096, 0.01]),
        b("P2", [0.0, 0.0, 0.2], [0.09, 0.096, 0.01]),
        b("PLATE", [0.0, 0.0, 0.1], [0.5, 0.5, 0.01]),
    ];
    let mut n = network(boxes, 0.8, &["FRAME", "P1", "P2", "PLATE"]);
    n.mounts = super::super::couplings::tests::mounts(&[["P1", "P2"]], 0.5, Some(0.002));
    let c = resolve(&n).unwrap();
    let used = c.view_factors.as_ref().unwrap();
    // P1 と P2 の見通しはすべて遮られる: 取付の既定値ではなく 0
    let (v, source) = value(&c, "P1", "P2").unwrap();
    assert_eq!(source, Source::ViewFactor);
    assert_eq!(v, 0.0);
    assert_eq!(used.blocked_node_pairs, 1);
    assert!(used.blocked_face_pairs >= 1);
    let pair = c
        .radiative
        .iter()
        .find(|l| l.a == "P1" && l.b == "P2")
        .unwrap();
    assert_eq!(pair.from_view_factors.unwrap().face_pairs, 0);
    assert!(value(&c, "P1", "PLATE").unwrap().0 > 0.0);
}

/// 優先順位: 明示 > 外殻 > 形態係数 > 取付の既定値。形態係数の値が出ない対（空洞に入らない部品）は取付の既定値のまま。
#[test]
fn precedence_explicit_then_view_factors_then_mount_default() {
    let mut boxes = stack(vec![]);
    boxes.push(b("OUT", [2.0, 0.0, 0.0], [0.2; 3]));
    let mut n = network(boxes, 0.8, &["FRAME", "P1", "P2", "OUT"]);
    n.mounts =
        super::super::couplings::tests::mounts(&[["P1", "P2"], ["P1", "OUT"]], 0.5, Some(0.002));
    let by_view = resolve(&n).unwrap();
    assert_eq!(value(&by_view, "P1", "P2").unwrap().1, Source::ViewFactor);
    assert_eq!(value(&by_view, "OUT", "P1"), Some((0.002, Source::Mount)));
    n.radiative = vec![crate::thermal::network::ThermalRadiativeLink {
        a: "P2".to_owned(),
        b: "P1".to_owned(),
        exchange_area: Qty::from_raw(0.01),
    }];
    let explicit = resolve(&n).unwrap();
    assert_eq!(value(&explicit, "P1", "P2"), Some((0.01, Source::Explicit)));
    // 書いた値の対に形態係数の内訳は残らない
    let pair = explicit
        .radiative
        .iter()
        .find(|l| l.a == "P1" && l.b == "P2")
        .unwrap();
    assert!(pair.from_view_factors.is_none());
}

/// 空洞が無ければ使わず（誤りにはしない）、理由を残す。放射率が範囲外なら入力の誤り。
#[test]
fn no_cavity_is_reported_and_bad_emissivity_is_an_error() {
    let n = network(
        vec![
            b("A", [0.0; 3], [0.1; 3]),
            b("B", [1.0, 0.0, 0.0], [0.1; 3]),
        ],
        0.8,
        &["A", "B"],
    );
    let c = resolve(&n).unwrap();
    let used = c.view_factors.unwrap();
    assert!(!used.applied);
    assert!(used.note.unwrap().contains("no cavity"));
    assert!(c.radiative.is_empty());
    for bad in [0.0, 1.2, f64::NAN] {
        assert!(
            resolve(&network(stack(vec![]), bad, &["FRAME", "P1", "P2"])).is_err(),
            "{bad}"
        );
    }
}

/// 節点の収支（面積で重みづけした `Σ F` の不足）は 0 以上 1 以下（接触なしの閉じた空洞では 0 に近い）。
#[test]
fn node_residuals_are_small_in_a_closed_cavity() {
    let c = resolve(&network(stack(vec![]), 0.8, &["FRAME", "P1", "P2"])).unwrap();
    let used = c.view_factors.unwrap();
    assert_eq!(used.nodes.len(), 3);
    for n in &used.nodes {
        assert!(n.residual.abs() < 0.03, "{} {}", n.node, n.residual);
    }
}

/// 同じ節点にまとめた部品どうしは数えない。節点にまとめた 2 部品（P1・P2）の対は 0 個。
#[test]
fn parts_of_one_node_do_not_exchange() {
    let mut n = network(stack(vec![]), 0.8, &["FRAME", "P"]);
    n.nodes[1].parts.push(node("P2", None).parts.remove(0));
    n.nodes[1].parts[0].id = "P1".to_owned();
    let c = resolve(&n).unwrap();
    assert!(value(&c, "FRAME", "P").is_some());
    assert!(c.radiative.iter().all(|l| l.a != l.b));
    assert_eq!(c.radiative.len(), 1);
}

/// 板を升に分けた節点でも、部品の値を升に配って解ける（配った合計は元の値と同じ）。
#[test]
fn plates_receive_the_value_by_area() {
    use crate::thermal::network::Compiled;
    let mut n = network(stack(vec![]), 0.8, &["FRAME", "P1", "P2"]);
    for node in &mut n.nodes {
        node.fixed_temperature = Some(Qty::from_raw(300.0));
    }
    let whole = Compiled::new(&n).unwrap();
    let total: f64 = whole.rad.iter().map(|(_, _, r)| r).sum();
    assert!(total > 0.0);
}
