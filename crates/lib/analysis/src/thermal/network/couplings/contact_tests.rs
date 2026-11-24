//! 取付の伝導を接触面積から出す設定（`contact_conductance_per_area`）の結合の試験。
//! 手計算: 接触面積 0.005 m²（100 × 50 mm の面）に h = 300 W/(m²·K) なら 1.5 W/K。

use super::tests::{cond, mounts, net};
use super::*;
use crate::structure::interference::InterferenceBox;
use crate::thermal::network::{
    Compiled, ThermalContact, ThermalContactFallbackReason, ThermalLoad, ThermalMountBoxes,
    ThermalMounts,
};
use sat_astro::math::{Quat, Vec3};
use sat_units::typed::Qty;

/// 接触面積から出す取付の試験の部品: 200 mm の立方体 `base`（上面は z = 0.1 m）と、その上面に載る板
/// （`plate`: 100 × 50 mm の面で接し、接触面積 0.005 m²。`shifted`: 0.1 m ずらして 0.0025 m²。`floating`: 隙間 1 mm。`unsized`: 箱なし）。
fn part_box(id: &str, centre: [f64; 3], size: [f64; 3]) -> Option<InterferenceBox> {
    Some(InterferenceBox {
        id: id.to_owned(),
        count: 1,
        position: Vec3::new(centre[0], centre[1], centre[2]),
        orientation: Quat::IDENTITY,
        size: Vec3::new(size[0], size[1], size[2]),
        mount_ancestors: vec![],
    })
}

fn base() -> Option<InterferenceBox> {
    part_box("base", [0.0; 3], [0.2; 3])
}

fn plate() -> Option<InterferenceBox> {
    part_box("plate", [0.0, 0.0, 0.15], [0.1, 0.05, 0.1])
}

fn shifted() -> Option<InterferenceBox> {
    part_box("shifted", [0.1, 0.0, 0.15], [0.1, 0.05, 0.1])
}

fn floating() -> Option<InterferenceBox> {
    part_box("floating", [0.0, 0.0, 0.151], [0.1, 0.05, 0.1])
}

/// 部品の取付（取り付ける部品の節点・取付先の節点・両端の箱）を、`h`（W/(m²·K)）と 1 取付あたりの値 `g` で組む。
fn contact_mounts(
    rows: &[(&str, &str, Option<InterferenceBox>, Option<InterferenceBox>)],
    h: f64,
    g: f64,
) -> Option<ThermalMounts> {
    Some(ThermalMounts {
        mounts: rows
            .iter()
            .map(|(a, b, _, _)| [(*a).into(), (*b).into()])
            .collect(),
        conductance: Qty::from_raw(g),
        exchange_area: None,
        contact: Some(ThermalContact {
            conductance_per_area: Qty::from_raw(h),
            mounts: rows
                .iter()
                .map(|(a, b, x, y)| ThermalMountBoxes {
                    parts: [(*a).into(), (*b).into()],
                    boxes: [x.clone(), y.clone()],
                })
                .collect(),
        }),
    })
}

/// 固定 300 K の節点に 10 W の節点を、接触面積 0.005 m² で取り付ける。h = 300 W/(m²·K) なら G = 300 × 0.005 = 1.5 W/K で、
/// 定常は T = 300 + 10 / 1.5（手計算）。1 取付あたりの値（0.5 W/K）は使わない。
#[test]
fn contact_area_sets_the_conductance_of_a_mount() {
    let mut n = net(&["base", "plate"]);
    n.nodes[0].fixed_temperature = Some(Qty::from_raw(300.0));
    n.mounts = contact_mounts(&[("plate", "base", plate(), base())], 300.0, 0.5);
    let c = Compiled::new(&n).unwrap();
    let used = &c.couplings.conductive[0];
    assert_eq!(used.source, ThermalCouplingSource::Contact);
    assert!((used.conductance.value() - 1.5).abs() < 1e-12);
    let detail = used.from_contact.unwrap();
    assert!((detail.area.value() - 0.005).abs() < 1e-15);
    assert_eq!((detail.contact_mounts, detail.fixed_mounts), (1, 0));
    let summary = c.couplings.mount_contact.as_ref().unwrap();
    assert_eq!(summary.geometric_mounts, 1);
    assert!(summary.fallbacks.is_empty());
    let q = c
        .loads(&[ThermalLoad {
            node: "plate".into(),
            power: Qty::from_raw(10.0),
            count: 1,
            rf_output: None,
            heat_fraction: Some(1.0),
        }])
        .unwrap();
    let s = c.solve_steady(&q, 3.0, 290.0).unwrap();
    assert!(
        (s.temps[1] - (300.0 + 10.0 / 1.5)).abs() < 1e-6,
        "{}",
        s.temps[1]
    );
}

/// 同じ節点対への取付が 2 か所（接触面積 0.005 と 0.0025 m²）なら、伝導は足し合わさる: 300 × (0.005 + 0.0025) = 2.25 W/K。
#[test]
fn mounts_between_the_same_nodes_add_their_contact_areas() {
    let mut n = net(&["lump", "base"]);
    n.mounts = contact_mounts(
        &[
            ("lump", "base", plate(), base()),
            ("lump", "base", shifted(), base()),
        ],
        300.0,
        0.5,
    );
    let c = Compiled::new(&n).unwrap().couplings;
    let used = &c.conductive[0];
    assert!((used.conductance.value() - 2.25).abs() < 1e-12);
    let detail = used.from_contact.unwrap();
    assert!((detail.area.value() - 0.0075).abs() < 1e-15);
    assert_eq!(detail.contact_mounts, 2);
}

/// 箱が無い・隙間が許容差より大きい取付は 1 取付あたりの値（0.5 W/K）に戻り、理由が出力に残る。
/// 接触面積から出した取付と同じ対にあれば、対の値は 1.5 + 0.5 + 0.5 = 2.5 W/K で、混ざった対として数が出る。
#[test]
fn mounts_without_contact_use_the_fixed_value_and_say_why() {
    let mut n = net(&["lump", "base", "other"]);
    n.mounts = contact_mounts(
        &[
            ("lump", "base", plate(), base()),
            ("lump", "base", None, base()),
            ("lump", "base", floating(), base()),
            ("other", "base", None, None),
        ],
        300.0,
        0.5,
    );
    let c = Compiled::new(&n).unwrap().couplings;
    let mixed = c
        .conductive
        .iter()
        .find(|l| l.a == "base" && l.b == "lump")
        .unwrap();
    assert_eq!(mixed.source, ThermalCouplingSource::Contact);
    assert!((mixed.conductance.value() - 2.5).abs() < 1e-12);
    assert_eq!(mixed.from_contact.unwrap().fixed_mounts, 2);
    // 箱のない取付だけの対は取付の値で、`contact` を持たない
    let fixed = c
        .conductive
        .iter()
        .find(|l| l.a == "base" && l.b == "other")
        .unwrap();
    assert_eq!(fixed.source, ThermalCouplingSource::Mount);
    assert_eq!(fixed.conductance.value(), 0.5);
    assert!(fixed.from_contact.is_none());
    let summary = c.mount_contact.unwrap();
    assert_eq!(summary.geometric_mounts, 1);
    let reasons: Vec<_> = summary.fallbacks.iter().map(|f| f.reason).collect();
    assert_eq!(reasons.len(), 3);
    assert_eq!(
        reasons
            .iter()
            .filter(|r| **r == ThermalContactFallbackReason::NoBox)
            .count(),
        2
    );
    assert_eq!(
        reasons
            .iter()
            .filter(|r| **r == ThermalContactFallbackReason::NotTouching)
            .count(),
        1
    );
}

/// 明示の値は接触面積から出した値も置き換える（出所は `explicit`、内訳は残さない）。
#[test]
fn explicit_conductance_replaces_the_contact_value() {
    let mut n = net(&["base", "plate"]);
    n.mounts = contact_mounts(&[("plate", "base", plate(), base())], 300.0, 0.5);
    n.conductive = vec![cond("plate", "base", 0.8)];
    let c = Compiled::new(&n).unwrap().couplings;
    assert_eq!(c.conductive[0].source, ThermalCouplingSource::Explicit);
    assert_eq!(c.conductive[0].conductance.value(), 0.8);
    assert!(c.conductive[0].from_contact.is_none());
}

/// 接触面積から出す設定でなければ、出力に `contact` の項目は無い（従来の出力と同じ）。
#[test]
fn without_a_coefficient_the_output_is_unchanged() {
    let mut n = net(&["a", "b"]);
    n.mounts = mounts(&[["a", "b"]], 0.5, None);
    let c = Compiled::new(&n).unwrap().couplings;
    assert!(c.mount_contact.is_none());
    assert!(c.conductive[0].from_contact.is_none());
    let json = serde_json::to_value(&c).unwrap();
    assert!(json.get("mount_contact").is_none());
    assert!(json["conductive"][0].get("from_contact").is_none());
}
