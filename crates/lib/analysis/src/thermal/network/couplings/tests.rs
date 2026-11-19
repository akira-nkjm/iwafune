use super::super::tests::node;
use super::*;
use crate::thermal::enclosure::ThermalBoxSize;
use crate::thermal::network::{Compiled, ThermalLoad, ThermalMounts};
use sat_units::typed::Qty;

fn cond(a: &str, b: &str, g: f64) -> ThermalConductiveLink {
    ThermalConductiveLink {
        a: a.to_owned(),
        b: b.to_owned(),
        conductance: Qty::from_raw(g),
    }
}

fn rad(a: &str, b: &str, area: f64) -> ThermalRadiativeLink {
    ThermalRadiativeLink {
        a: a.to_owned(),
        b: b.to_owned(),
        exchange_area: Qty::from_raw(area),
    }
}

fn mounts(pairs: &[[&str; 2]], g: f64, area: Option<f64>) -> Option<ThermalMounts> {
    Some(ThermalMounts {
        mounts: pairs
            .iter()
            .map(|[a, b]| [(*a).into(), (*b).into()])
            .collect(),
        conductance: Qty::from_raw(g),
        exchange_area: area.map(Qty::from_raw),
    })
}

fn net(nodes: &[&str]) -> ThermalNetwork {
    ThermalNetwork {
        mounts: None,
        nodes: nodes.iter().map(|n| node(n, None)).collect(),
        conductive: vec![],
        radiative: vec![],
        enclosure: None,
        surfaces: vec![],
    }
}

fn conductance(c: &ThermalCouplings, a: &str, b: &str) -> Option<(f64, ThermalCouplingSource)> {
    let (a, b) = if a <= b { (a, b) } else { (b, a) };
    c.conductive
        .iter()
        .find(|l| l.a == a && l.b == b)
        .map(|l| (l.conductance.value(), l.source))
}

fn exchange(c: &ThermalCouplings, a: &str, b: &str) -> Option<(f64, ThermalCouplingSource)> {
    let (a, b) = if a <= b { (a, b) } else { (b, a) };
    c.radiative
        .iter()
        .find(|l| l.a == a && l.b == b)
        .map(|l| (l.exchange_area.value(), l.source))
}

/// 取付 2 か所（0.5 W/K × 2 = 1.0 W/K）の対に 0.8 W/K を明示すると、取付の値は置き換わる。
/// 固定 300 K の節点に 10 W の節点をつなぐと T = 300 + 10 / 0.8 = 312.5 K（手計算）。書いていない対は取付のまま。
#[test]
fn explicit_conductance_replaces_the_mount_value() {
    let mut n = net(&["a", "b", "c"]);
    n.nodes[0].fixed_temperature = Some(Qty::from_raw(300.0));
    n.mounts = mounts(&[["a", "b"], ["b", "a"], ["b", "c"]], 0.5, None);
    n.conductive = vec![cond("b", "a", 0.8)];
    let c = Compiled::new(&n).unwrap();
    assert_eq!(
        conductance(&c.couplings, "a", "b"),
        Some((0.8, ThermalCouplingSource::Explicit))
    );
    assert_eq!(
        conductance(&c.couplings, "b", "c"),
        Some((0.5, ThermalCouplingSource::Mount))
    );
    // c を固定にして b の収支だけを見る: 10 W を b に入れ、c を 300 K に固定
    n.nodes[2].fixed_temperature = Some(Qty::from_raw(300.0));
    n.conductive = vec![cond("b", "a", 0.8)];
    let c = Compiled::new(&n).unwrap();
    let q = c
        .loads(&[ThermalLoad {
            node: "b".into(),
            power: Qty::from_raw(10.0),
            count: 1,
            rf_output: None,
            heat_fraction: Some(1.0),
        }])
        .unwrap();
    let s = c.solve_steady(&q, 3.0, 290.0).unwrap();
    // 10 = (0.8 + 0.5)(T − 300) → T = 300 + 10 / 1.3
    assert!(
        (s.temps[1] - (300.0 + 10.0 / 1.3)).abs() < 1e-6,
        "{}",
        s.temps[1]
    );
}

/// 0 を明示すると、その対は切り離される（取付があっても）。
#[test]
fn zero_conductance_isolates_the_pair() {
    let mut n = net(&["a", "b"]);
    n.nodes[0].fixed_temperature = Some(Qty::from_raw(300.0));
    n.mounts = mounts(&[["a", "b"]], 0.5, None);
    n.conductive = vec![cond("a", "b", 0.0)];
    let c = Compiled::new(&n).unwrap();
    assert_eq!(
        conductance(&c.couplings, "a", "b"),
        Some((0.0, ThermalCouplingSource::Explicit))
    );
    assert!(c.solve_steady(&[0.0, 1.0], 3.0, 290.0).is_err());
}

#[test]
fn explicit_radiation_replaces_the_default() {
    let mut n = net(&["a", "b", "c"]);
    n.mounts = mounts(&[["a", "b"], ["b", "c"]], 0.5, Some(0.002));
    n.radiative = vec![rad("b", "a", 0.01)];
    let c = Compiled::new(&n).unwrap().couplings;
    assert_eq!(
        exchange(&c, "a", "b"),
        Some((0.01, ThermalCouplingSource::Explicit))
    );
    assert_eq!(
        exchange(&c, "b", "c"),
        Some((0.002, ThermalCouplingSource::Mount))
    );
}

#[test]
fn bad_explicit_couplings_are_rejected() {
    let mut n = net(&["a", "b"]);
    n.conductive = vec![cond("a", "b", 1.0), cond("b", "a", 2.0)];
    assert!(Compiled::new(&n).is_err(), "duplicate pair (either order)");
    n.conductive = vec![cond("a", "a", 1.0)];
    assert!(Compiled::new(&n).is_err(), "same node");
    n.conductive = vec![cond("a", "z", 1.0)];
    assert!(Compiled::new(&n).is_err(), "unknown node");
    n.conductive = vec![cond("a", "b", -1.0)];
    assert!(Compiled::new(&n).is_err(), "negative");
}

fn face_surface(node: &str, normal: [f64; 3], area: f64) -> ThermalSurface {
    ThermalSurface {
        id: format!("s_{node}"),
        node: node.to_owned(),
        area: Qty::from_raw(area),
        normal: sat_astro::math::Vec3::new(normal[0], normal[1], normal[2]),
        count: 1,
        absorptivity: Some(0.3),
        emissivity: Some(0.8),
        coatings: vec![],
    }
}

/// 辺 0.1 / 0.2 / 0.3 m の箱。面 `+X` から `-Z` の節点は fpx … fmz、内部の部品の節点は `in`。
fn box_net() -> ThermalNetwork {
    let faces = ["fpx", "fmx", "fpy", "fmy", "fpz", "fmz"];
    let mut nodes: Vec<&str> = faces.to_vec();
    nodes.push("in");
    let mut n = net(&nodes);
    let (x, y, z) = (0.1, 0.2, 0.3);
    let areas = [y * z, y * z, x * z, x * z, x * y, x * y];
    let normals = [
        [1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, -1.0],
    ];
    n.surfaces = (0..6)
        .map(|i| face_surface(faces[i], normals[i], areas[i]))
        .collect();
    n.mounts = mounts(
        &[["fpx", "fmx"], ["in", "fpx"], ["in", "fmz"]],
        0.5,
        Some(0.002),
    );
    n.enclosure = Some(ThermalEnclosure {
        size: Some(ThermalBoxSize {
            x: Qty::from_raw(x),
            y: Qty::from_raw(y),
            z: Qty::from_raw(z),
        }),
        emissivity: 0.8,
    });
    n
}

/// 外殻を使うと、面どうし 15 対はすべて外殻の値になり（取付の既定値は使わない）、
/// 内部の部品の対は取付の既定値のまま。明示の放射は外殻にも勝つ。
#[test]
fn enclosure_replaces_face_pairs_and_explicit_wins() {
    let mut n = box_net();
    n.radiative = vec![rad("fpz", "fmz", 0.123)];
    let c = Compiled::new(&n).unwrap().couplings;
    let used = c.enclosure.as_ref().unwrap();
    assert!(used.applied);
    assert_eq!(used.faces.len(), 6);
    assert_eq!(used.view_factors.len(), 6);
    let from_enclosure = c
        .radiative
        .iter()
        .filter(|l| l.source == ThermalCouplingSource::Enclosure)
        .count();
    assert_eq!(from_enclosure, 14); // 15 対のうち 1 対は明示
    assert_eq!(
        exchange(&c, "fpz", "fmz"),
        Some((0.123, ThermalCouplingSource::Explicit))
    );
    let (v, s) = exchange(&c, "fpx", "fmx").unwrap();
    assert_eq!(s, ThermalCouplingSource::Enclosure);
    assert_ne!(v, 0.002);
    assert_eq!(
        exchange(&c, "fpx", "in"),
        Some((0.002, ThermalCouplingSource::Mount))
    );
    assert_eq!(
        exchange(&c, "fmz", "in"),
        Some((0.002, ThermalCouplingSource::Mount))
    );
    // 面 +X と -X の値は、直接計算した ε_i A_i B_ij と一致する
    let direct = crate::thermal::enclosure::box_exchange([0.1, 0.2, 0.3], 0.8).unwrap();
    assert!((v - direct.exchange_m2[0][1]).abs() < 1e-15);
}

/// 面が足りなければ外殻は使わず（誤りにはしない）、理由を残して取付の既定値のまま。
#[test]
fn enclosure_not_applied_without_six_faces() {
    let mut n = box_net();
    n.surfaces.pop();
    let c = Compiled::new(&n).unwrap().couplings;
    let used = c.enclosure.as_ref().unwrap();
    assert!(!used.applied);
    assert!(used.note.as_ref().unwrap().contains("-Z"));
    assert!(used.view_factors.is_empty());
    assert_eq!(
        exchange(&c, "fpx", "fmx").unwrap().1,
        ThermalCouplingSource::Mount
    );
    // 箱の寸法が決まらなくても同じ（使わず、理由を残す）
    let mut m = box_net();
    m.enclosure.as_mut().unwrap().size = None;
    let used = Compiled::new(&m).unwrap().couplings.enclosure.unwrap();
    assert!(!used.applied && used.note.unwrap().contains("size"));
    // 外殻を求めていなければ出力に外殻の欄はない
    n.enclosure = None;
    assert!(Compiled::new(&n).unwrap().couplings.enclosure.is_none());
}
