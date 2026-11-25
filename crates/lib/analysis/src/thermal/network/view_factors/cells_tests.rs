//! 升に分けた板の放射（#456）。節点対の値（面全体の `E`）は升に分けない設計と同じで、その配り方だけを升と相手の面の `A_c F_cc'` で決める。
//! 基板 P1（90 mm × 96 mm × 10 mm、u = x）を 3 × 1 の升（x の境は −15 mm・15 mm）に分け、隙間 2.5 mm で向き合う基板 P2（30 mm × 96 mm × 5 mm）を x ∈ [15, 45] mm だけに置く
//! （P1 の升 2 の真上）。

use super::super::couplings::resolve_with_cells;
use super::super::tests::node;
use super::*;
use crate::thermal::network::{Compiled, ThermalPlate};
use sat_astro::math::{Quat, Vec3};
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

fn boxes() -> Vec<InterferenceBox> {
    vec![
        b("FRAME", [0.0; 3], [1.0; 3]),
        b("P1", [0.0, 0.0, 0.0], [0.09, 0.096, 0.01]),
        b("P2", [0.03, 0.0, 0.01], [0.03, 0.096, 0.005]),
    ]
}

fn plate(part: &str, frame: InterferenceBox, cells: [u32; 2]) -> ThermalPlate {
    ThermalPlate {
        part: part.to_owned(),
        cells,
        conductivity: Qty::from_raw(100.0),
        frame,
        attachments: Vec::new(),
    }
}

fn network(plates: Vec<ThermalPlate>) -> ThermalNetwork {
    let mut n = super::super::couplings::tests::net(&["FRAME", "P1", "P2"]);
    n.nodes = ["FRAME", "P1", "P2"].map(|id| node(id, None)).to_vec();
    n.view_factors = Some(ThermalViewFactors {
        emissivity: 0.8,
        boxes: boxes(),
    });
    n.plates = plates;
    n
}

/// 節点 `a` の升（通し番号の順）と `b` の升の間の放射の係数 `σ R` の表（行 = a の升）。板でない節点は 1 つの升。
fn between(c: &Compiled, a: &str, b: &str) -> Vec<Vec<f64>> {
    let (sa, sb) = (c.slots(a).unwrap(), c.slots(b).unwrap());
    sa.iter()
        .map(|&(i, _)| {
            sb.iter()
                .map(|&(j, _)| {
                    c.rad
                        .iter()
                        .filter(|&&(x, y, _)| (x, y) == (i, j) || (x, y) == (j, i))
                        .map(|r| r.2)
                        .sum()
                })
                .collect()
        })
        .collect()
}

/// 升に分けても、P1–P2 の値の合計は分けない設計と同じ（保存。`σ E`）。真上に P2 がある升 2 が大半を受け、升 0 は P2 から遠くてわずか。
/// 分けない設計の値と、解いた結合の一覧（`couplings`）の値は同じ（升への配り方は結合の一覧に出さない）。
#[test]
fn the_pair_value_is_conserved_and_goes_to_the_cells_that_see_the_other_board() {
    let split = network(vec![plate("P1", boxes()[1].clone(), [3, 1])]);
    let plain = network(vec![]);
    let (cs, _) = resolve_with_cells(&split).unwrap();
    let (cp, _) = resolve_with_cells(&plain).unwrap();
    let flat = |c: &crate::thermal::network::ThermalCouplings| -> Vec<(String, String, f64)> {
        c.radiative
            .iter()
            .map(|l| (l.a.clone(), l.b.clone(), l.exchange_area.value()))
            .collect()
    };
    assert_eq!(flat(&cs), flat(&cp));
    let e = cp
        .radiative
        .iter()
        .find(|l| l.a == "P1" && l.b == "P2")
        .unwrap()
        .exchange_area
        .value();
    let net = Compiled::new(&split).unwrap();
    let cells: Vec<f64> = between(&net, "P1", "P2")
        .into_iter()
        .map(|r| r[0])
        .collect();
    let sigma = sat_astro::constants::SIGMA_SB;
    assert!(
        (cells.iter().sum::<f64>() / (sigma * e) - 1.0).abs() < 1e-12,
        "{cells:?}"
    );
    assert!(
        cells[2] > cells[1] && cells[1] > cells[0] && cells[0] > 0.0,
        "{cells:?}"
    );
    assert!(cells[2] > 0.8 * cells.iter().sum::<f64>(), "{cells:?}");
}

/// 両端を升に分けると、升どうしの組（`A_c F_cc'`）で配る（P2 も 3 × 1 に分ける。升は x ∈ [15, 25]・[25, 35]・[35, 45] mm）:
/// P2 は P1 の升 2 の真上だけにあるので、P1 の升 2 の行が大半を占め、その中は P2 の中央の升（P1 の升 2 の中心の真上）が両側の升より大きく、
/// 両側は対称で同じ。合計は分けない設計と同じ。
/// 壁（容器）との組の合計も保存される。
#[test]
fn both_ends_split_use_cell_to_cell_values_and_conserve_every_pair() {
    let plates = vec![
        plate("P1", boxes()[1].clone(), [3, 1]),
        plate("P2", boxes()[2].clone(), [3, 1]),
    ];
    let split = Compiled::new(&network(plates)).unwrap();
    let plain = Compiled::new(&network(vec![])).unwrap();
    let sum = |c: &Compiled, a: &str, b: &str| -> f64 { between(c, a, b).iter().flatten().sum() };
    for (a, b) in [("P1", "P2"), ("FRAME", "P1"), ("FRAME", "P2")] {
        let (s, p) = (sum(&split, a, b), sum(&plain, a, b));
        assert!((s / p - 1.0).abs() < 1e-12, "{a}-{b}: {s} {p}");
    }
    let m = between(&split, "P1", "P2");
    assert!(
        m[2][1] > m[2][0] && (m[2][0] - m[2][2]).abs() < 1e-9 * m[2][0],
        "{m:?}"
    );
    let total: f64 = m.iter().flatten().sum();
    assert!(m[2].iter().sum::<f64>() > 0.8 * total, "{m:?}");
}

/// 升に分けない設計は、これまでの値（面積で配る）と同じ: 板の端が升に分かれなければ（節点が 1 つ）升の割合を使わない。
/// 明示の放射が形態係数の値を置き換えた対は、升の割合ではなく面積の割合で配る。
#[test]
fn an_explicit_value_replaces_the_cell_split_and_is_spread_by_area() {
    let mut n = network(vec![plate("P1", boxes()[1].clone(), [3, 1])]);
    n.radiative = vec![crate::thermal::network::ThermalRadiativeLink {
        a: "P1".into(),
        b: "P2".into(),
        exchange_area: Qty::from_raw(0.003),
    }];
    let net = Compiled::new(&n).unwrap();
    let cells: Vec<f64> = between(&net, "P1", "P2")
        .into_iter()
        .map(|r| r[0])
        .collect();
    let share = sat_astro::constants::SIGMA_SB * 0.003 / 3.0;
    assert!(
        cells.iter().all(|c| (c / share - 1.0).abs() < 1e-12),
        "{cells:?}"
    );
}
