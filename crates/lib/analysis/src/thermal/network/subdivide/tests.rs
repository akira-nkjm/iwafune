use super::*;
use crate::thermal::network::tests::node;
use crate::thermal::network::{Compiled, ThermalConductiveLink, ThermalLoad, ThermalNetwork};
use sat_astro::math::{Quat, Vec3};

fn frame(id: &str, c: [f64; 3], size: [f64; 3]) -> InterferenceBox {
    InterferenceBox {
        id: id.to_owned(),
        count: 1,
        position: Vec3::new(c[0], c[1], c[2]),
        orientation: Quat::IDENTITY,
        size: Vec3::new(size[0], size[1], size[2]),
        mount_ancestors: Vec::new(),
    }
}

fn plate(cells: [u32; 2]) -> ThermalPlate {
    ThermalPlate {
        part: "P".to_owned(),
        cells,
        conductivity: Qty::from_raw(100.0),
        frame: frame("P", [0.0; 3], [0.3, 0.2, 0.003]),
        attachments: Vec::new(),
    }
}

fn network(cells: [u32; 2], g: f64) -> ThermalNetwork {
    ThermalNetwork {
        plates: vec![plate(cells)],
        mounts: None,
        nodes: vec![node("P", None), node("B", None)],
        conductive: vec![ThermalConductiveLink {
            a: "B".into(),
            b: "P".into(),
            conductance: Qty::from_raw(g),
        }],
        radiative: Vec::new(),
        enclosure: None,
        view_factors: None,
        surfaces: Vec::new(),
        shadowing: None,
    }
}

/// 手計算: 0.3 × 0.2 × 0.003 m を 3 × 2 に分けると升は 0.1 × 0.1 m。面内の伝導は u 方向・v 方向とも
/// k t (0.1) / 0.1 = 100 · 0.003 = 0.3 W/K。隣り合う組は u 方向 2 × 2 = 4、v 方向 3 × 1 = 3 で、合計 7 本。
#[test]
fn in_plane_conductance_by_hand() {
    let c = Compiled::new(&network([3, 2], 0.0)).unwrap();
    let plate_links: Vec<_> = c.cond.iter().filter(|(a, b, _)| *a < 6 && *b < 6).collect();
    assert_eq!(plate_links.len(), 7);
    assert!(plate_links.iter().all(|(_, _, g)| (g - 0.3).abs() < 1e-12));
    let used = &c.couplings.plates[0];
    assert_eq!(used.cells, [3, 2]);
    assert!((used.conductance_u.value() - 0.3).abs() < 1e-12);
    assert!((used.thickness.value() - 0.003).abs() < 1e-15);
    // 升の熱容量 = 質量 1 kg の 1/6 × 900 J/(kg·K) = 150 J/K。最小の時定数は伝導の合計が最小の角の升（0.3 · 2 + 外への 0.5/6… 下記）
    assert!((c.capacity[0] - 150.0).abs() < 1e-9);
}

/// 明示の伝導 B–P（1.2 W/K）は板の升すべてに面積の割合で配られ、合計は元の値と同じ（保存）。節点 ID は升のもの。
#[test]
fn explicit_coupling_is_spread_by_area_and_conserves_the_total() {
    let c = Compiled::new(&network([3, 2], 1.2)).unwrap();
    let b = c.index("B").unwrap();
    let to_b: Vec<f64> = c
        .cond
        .iter()
        .filter(|(i, j, _)| *i == b || *j == b)
        .map(|x| x.2)
        .collect();
    assert_eq!(to_b.len(), 6);
    assert!(to_b.iter().all(|g| (g - 0.2).abs() < 1e-12)); // 1.2 / 6
    assert!((to_b.iter().sum::<f64>() - 1.2).abs() < 1e-12);
    // 出力の結合は板の部品の名前のまま
    assert_eq!(c.couplings.conductive[0].b, "P");
    assert!(c.index("P").is_err());
    assert_eq!(c.ids[0], "P[0,0]");
}

/// 発熱は升の面積の割合で配る（9 W → 6 升で 1.5 W ずつ、合計は 9 W）。
#[test]
fn loads_are_spread_by_area() {
    let c = Compiled::new(&network([3, 2], 0.0)).unwrap();
    let q = c
        .loads(&[ThermalLoad {
            node: "P".into(),
            power: Qty::from_raw(3.0),
            count: 3,
            rf_output: None,
            heat_fraction: Some(1.0),
        }])
        .unwrap();
    assert!(q[..6].iter().all(|x| (x - 1.5).abs() < 1e-12));
    assert!((q.iter().sum::<f64>() - 9.0).abs() < 1e-12);
}

/// 取付の伝導は相手の箱と接する升に、接触面積の割合で配る。0.3 × 0.2 の板（3 × 2 升、升は 0.1 × 0.1、中心は原点）の +z の面に、
/// 0.15 × 0.1 の箱を x ∈ [−0.15, 0]・y ∈ [−0.1, 0] に置く。升 (0,0)（x ∈ [−0.15, −0.05]、y ∈ [−0.1, 0]）には 0.1 × 0.1 = 0.01 m²、
/// 升 (1,0)（x ∈ [−0.05, 0.05]）には 0.05 × 0.1 = 0.005 m² が接するので、割合は 2/3 と 1/3（取付 3 W/K → 2 W/K と 1 W/K）。
#[test]
fn mount_conduction_goes_to_the_cells_under_the_footprint() {
    use crate::thermal::network::ThermalMounts;
    let mut net = network([3, 2], 0.0);
    net.conductive.clear();
    net.plates[0].attachments = vec![ThermalPlateAttachment {
        node: "B".into(),
        footprint: frame("B", [-0.075, -0.05, 0.0015 + 0.01], [0.15, 0.1, 0.02]),
    }];
    net.mounts = Some(ThermalMounts {
        mounts: vec![["B".into(), "P".into()]],
        conductance: Qty::from_raw(3.0),
        exchange_area: None,
        contact: None,
    });
    let c = Compiled::new(&net).unwrap();
    let b = c.index("B").unwrap();
    let mut to_b: Vec<(usize, f64)> = c
        .cond
        .iter()
        .filter(|(i, _, _)| *i == b)
        .map(|(_, j, g)| (*j, *g))
        .collect();
    to_b.sort_by_key(|x| x.0);
    // 升 (0,0) は通し番号 0、升 (1,0) は 2
    assert_eq!(to_b.len(), 2);
    assert_eq!(to_b[0].0, 0);
    assert!(
        (to_b[0].1 - 2.0).abs() < 1e-9 && (to_b[1].1 - 1.0).abs() < 1e-9,
        "{to_b:?}"
    );
    assert!(c.couplings.plates[0].spread_by_area.is_empty());
}

/// 箱が許容差の外（板の面から 5 mm 浮いている）でも、板の面に射影した形と升の重なりで同じ割合（2/3 と 1/3）に配る。
#[test]
fn a_floating_footprint_is_located_by_projection() {
    use crate::thermal::network::ThermalMounts;
    let mut net = network([3, 2], 0.0);
    net.conductive.clear();
    net.plates[0].attachments = vec![ThermalPlateAttachment {
        node: "B".into(),
        footprint: frame(
            "B",
            [-0.075, -0.05, 0.0015 + 0.005 + 0.01],
            [0.15, 0.1, 0.02],
        ),
    }];
    net.mounts = Some(ThermalMounts {
        mounts: vec![["B".into(), "P".into()]],
        conductance: Qty::from_raw(3.0),
        exchange_area: None,
        contact: None,
    });
    let c = Compiled::new(&net).unwrap();
    let b = c.index("B").unwrap();
    let mut g: Vec<(usize, f64)> = c
        .cond
        .iter()
        .filter(|x| x.0 == b)
        .map(|x| (x.1, x.2))
        .collect();
    g.sort_by_key(|x| x.0);
    assert_eq!(g.len(), 2);
    assert!(
        (g[0].1 - 2.0).abs() < 1e-9 && (g[1].1 - 1.0).abs() < 1e-9,
        "{g:?}"
    );
    assert!(c.couplings.plates[0].spread_by_area.is_empty());
}

/// 接する箱が無い（位置を決められない）取付は升の面積の割合で配り、相手を出力に残す。
#[test]
fn an_unlocated_mount_is_spread_by_area_and_reported() {
    use crate::thermal::network::ThermalMounts;
    let mut net = network([2, 2], 0.0);
    net.conductive.clear();
    net.mounts = Some(ThermalMounts {
        mounts: vec![["B".into(), "P".into()]],
        conductance: Qty::from_raw(2.0),
        exchange_area: None,
        contact: None,
    });
    let c = Compiled::new(&net).unwrap();
    let b = c.index("B").unwrap();
    let g: Vec<f64> = c.cond.iter().filter(|x| x.0 == b).map(|x| x.2).collect();
    assert_eq!(g.len(), 4);
    assert!(g.iter().all(|x| (x - 0.5).abs() < 1e-12));
    assert_eq!(c.couplings.plates[0].spread_by_area, ["B"]);
}

/// サーモスタット付きのヒータは単一の節点にしか置けない（板の部品の節点には置けない）。
#[test]
fn a_thermostat_heater_cannot_sit_on_a_plate() {
    use crate::thermal::network::ThermalHeater;
    let c = Compiled::new(&network([2, 2], 0.0)).unwrap();
    let h = ThermalHeater {
        id: "H".into(),
        node: "P".into(),
        on_below: Qty::from_raw(280.0),
        off_above: Qty::from_raw(290.0),
        power: Qty::from_raw(5.0),
        count: 1,
        redundancy: Default::default(),
        states: Vec::new(),
    };
    let err = c.heaters(&[h]).unwrap_err().to_string();
    assert!(err.contains("subdivided plate"), "{err}");
}

/// 升の最小の時定数 C / ΣG: 1 × 2 に分けた 0.3 × 0.2 の板（k t = 0.3 W/m·K·m、升は 0.3 × 0.1）の面内の伝導は
/// k t · 0.3 / 0.1 = 0.9 W/K の 1 本。升の熱容量は 450 J/K なので時定数は 500 s。
#[test]
fn smallest_time_constant_by_hand() {
    let c = Compiled::new(&network([1, 2], 0.0)).unwrap();
    let u = &c.couplings.plates[0];
    assert!((u.conductance_v.value() - 0.9).abs() < 1e-12);
    assert!((u.min_time_constant.value() - 500.0).abs() < 1e-9);
}
