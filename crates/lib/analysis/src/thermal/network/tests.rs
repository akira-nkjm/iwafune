use super::*;

pub(crate) fn node(id: &str, fixed: Option<f64>) -> ThermalNode {
    ThermalNode {
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

#[test]
fn loads_count_by_hand() {
    let net = Compiled::new(&ThermalNetwork {
        mounts: None,
        nodes: vec![node("n", None)],
        conductive: vec![],
        radiative: vec![],
        enclosure: None,
        surfaces: vec![],
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
        mounts: None,
        nodes: vec![n],
        conductive: vec![],
        radiative: vec![],
        enclosure: None,
        surfaces: vec![],
    })
    .unwrap();
    assert!((c.capacity[0] - 2050.0).abs() < 1e-9);
}
