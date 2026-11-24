use super::*;

pub(crate) fn node(id: &str, fixed: Option<f64>) -> ThermalNode {
    ThermalNode {
        cell: None,
        id: id.to_owned(),
        parts: vec![ThermalNodePart {
            id: id.to_owned(),
            mass: Qty::from_raw(1.0),
            count: 1,
            specific_heat: Qty::from_raw(900.0),
            ranges: ThermalRanges::default(),
            redundancy: crate::modes::Redundancy::None,
            states: Vec::new(),
        }],
        per_part: false,
        fixed_temperature: fixed.map(Qty::from_raw),
        required: Default::default(),
    }
}

/// 板の詳細の確かめ用: 節点 `part` を 3 × 2 の升に分ける（箱 0.3 × 0.2 × 0.003 m を (1, 2, 3) に置く）。
pub(crate) fn plate_of(part: &str) -> ThermalPlate {
    use crate::structure::interference::InterferenceBox;
    use sat_astro::math::{Quat, Vec3};
    ThermalPlate {
        part: part.to_owned(),
        cells: [3, 2],
        conductivity: Qty::from_raw(100.0),
        frame: InterferenceBox {
            id: part.to_owned(),
            count: 1,
            position: Vec3::new(1.0, 2.0, 3.0),
            orientation: Quat::IDENTITY,
            size: Vec3::new(0.3, 0.2, 0.003),
            mount_ancestors: Vec::new(),
        },
        attachments: Vec::new(),
    }
}

#[test]
fn loads_count_by_hand() {
    let net = Compiled::new(&ThermalNetwork {
        plates: Vec::new(),
        mounts: None,
        nodes: vec![node("n", None)],
        conductive: vec![],
        radiative: vec![],
        enclosure: None,
        surfaces: vec![],
        shadowing: None,
    })
    .unwrap();
    let q = net
        .loads(&[ThermalLoad {
            node: "n".into(),
            power: Qty::from_raw(2.0),
            count: 3,
            rf_output: None,
            heat_fraction: Some(0.5),
        }])
        .unwrap();
    assert_eq!(q, vec![3.0]); // 2 W × 3 台 × 0.5
}

fn range(min: f64, max: f64) -> Option<crate::thermal::range::ThermalTempRange> {
    Some(crate::thermal::range::ThermalTempRange {
        min: Qty::from_raw(min),
        max: Qty::from_raw(max),
    })
}

/// 1 kg × 2 台（900 J/(kg·K)）と 0.5 kg × 1 台（500 J/(kg·K)）→ C = 1800 + 250 = 2050 J/K。
/// 動作範囲は 233〜343 K と 253〜358 K の共通部分 253〜343 K。
#[test]
fn node_aggregates_parts() {
    let mut n = node("n", None);
    n.parts[0].count = 2;
    n.parts[0].ranges.operating = range(233.0, 343.0);
    n.parts.push(ThermalNodePart {
        id: "b".to_owned(),
        mass: Qty::from_raw(0.5),
        count: 1,
        specific_heat: Qty::from_raw(500.0),
        ranges: ThermalRanges {
            operating: range(253.0, 358.0),
            ..ThermalRanges::default()
        },
        redundancy: crate::modes::Redundancy::None,
        states: Vec::new(),
    });
    assert!((n.heat_capacity() - 2050.0).abs() < 1e-9);
    let r = n.ranges().operating.unwrap();
    assert_eq!((r.min.value(), r.max.value()), (253.0, 343.0));
    let c = Compiled::new(&ThermalNetwork {
        plates: Vec::new(),
        mounts: None,
        nodes: vec![n],
        conductive: vec![],
        radiative: vec![],
        enclosure: None,
        surfaces: vec![],
        shadowing: None,
    })
    .unwrap();
    assert!((c.capacity[0] - 2050.0).abs() < 1e-9);
}

/// 熱容量が無い節点の判定（`lacks_heat_capacity`）: 質量 0、員数 0、部品なしは熱容量 0。
/// 部品の 1 つに質量があれば足り、温度一定の境界節点は熱容量が要らない。解析の入力検査も同じ判定を通る。
#[test]
fn a_node_without_heat_capacity_is_refused() {
    assert!(!node("n", None).lacks_heat_capacity());
    let mut zero_mass = node("n", None);
    zero_mass.parts[0].mass = Qty::from_raw(0.0);
    assert!(zero_mass.lacks_heat_capacity());
    let mut zero_count = node("n", None);
    zero_count.parts[0].count = 0;
    assert!(zero_count.lacks_heat_capacity());
    let mut lumped = zero_mass.clone();
    lumped.parts.push(node("m", None).parts.remove(0));
    assert!(!lumped.lacks_heat_capacity());
    let mut empty = node("n", None);
    empty.parts.clear();
    assert!(empty.lacks_heat_capacity());
    let mut boundary = zero_mass.clone();
    boundary.fixed_temperature = Some(Qty::from_raw(3.0));
    assert!(!boundary.lacks_heat_capacity());
    for n in [zero_mass, zero_count, empty] {
        let net = ThermalNetwork {
            mounts: None,
            nodes: vec![n],
            conductive: vec![],
            radiative: vec![],
            enclosure: None,
            surfaces: vec![],
            plates: vec![],
            shadowing: None,
        };
        let e = Compiled::new(&net)
            .map(|_| ())
            .expect_err("refused")
            .to_string();
        assert!(e.contains("must be positive"), "{e}");
    }
}
