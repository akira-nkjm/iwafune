//! `structure.static_load` のテスト（手計算との比較）。

use super::*;

fn item(id: &str, mass: f64, mount: &str, h: Option<f64>) -> StaticLoadItem {
    StaticLoadItem {
        id: id.to_owned(),
        mass: Qty::from_raw(mass),
        count: 1,
        mount: mount.to_owned(),
        cg_height: h.map(Qty::from_raw),
    }
}

/// RW 0.6 kg を PANEL（0.4 kg）に付け、PANEL を FRAME に付ける。機軸 7 g、横 3 g。
/// PANEL の取付部: 0.6 kg × 9.80665 × 7 = 41.19 N。FRAME の取付部: 1.0 kg → 68.65 N、
/// 横 29.42 N、モーメント 1.0·9.80665·3·0.05 = 1.471 N·m。
#[test]
fn hand_calculated_tree() {
    let input = StaticLoadInput {
        items: vec![
            item("RW", 0.6, "PANEL", Some(0.02)),
            item("PANEL", 0.4, "FRAME", Some(0.05)),
        ],
        cases: vec![
            StaticLoadCase {
                id: "a".to_owned(),
                label: "axial".to_owned(),
                axial: 7.0,
                lateral: 1.0,
            },
            StaticLoadCase {
                id: "l".to_owned(),
                label: "lateral".to_owned(),
                axial: 2.0,
                lateral: 3.0,
            },
        ],
        interfaces: vec![StaticLoadInterface {
            mount: "FRAME".to_owned(),
            allowable_axial: Some(Qty::from_raw(200.0)),
            allowable_lateral: None,
            allowable_moment: Some(Qty::from_raw(1.0)),
        }],
        safety_factor: None,
    };
    let out = StaticLoad::run(&input).unwrap();
    let frame = out.mounts.iter().find(|m| m.mount == "FRAME").unwrap();
    let panel = out.mounts.iter().find(|m| m.mount == "PANEL").unwrap();
    assert!((panel.axial.value() - 0.6 * G0 * 7.0).abs() < 1e-9);
    assert!((frame.carried_mass.value() - 1.0).abs() < 1e-12);
    assert!((frame.axial.value() - G0 * 7.0).abs() < 1e-9);
    assert!((frame.lateral.value() - G0 * 3.0).abs() < 1e-9);
    assert!((frame.moment.value() - G0 * 3.0 * 0.05).abs() < 1e-9);
    let ms_ax = 200.0 / (1.25 * G0 * 7.0) - 1.0;
    assert!((frame.margin_axial.unwrap().value() - ms_ax).abs() < 1e-9);
    // モーメントは 1.471 N·m × 1.25 > 1 N·m で負の余裕
    assert!(frame.margin_moment.unwrap().value() < 0.0);
    assert_eq!(frame.min_margin, frame.margin_moment);
    assert_eq!(out.safety_factor, DEFAULT_SAFETY_FACTOR);
    assert_eq!(out.checks.len(), 1);
    assert_eq!(out.checks[0].verdict, Verdict::Violation);
}

#[test]
fn mount_cycle_is_rejected() {
    let input = StaticLoadInput {
        items: vec![item("A", 1.0, "B", None), item("B", 1.0, "A", None)],
        cases: vec![StaticLoadCase {
            id: "a".to_owned(),
            label: String::new(),
            axial: 1.0,
            lateral: 1.0,
        }],
        interfaces: vec![],
        safety_factor: None,
    };
    assert!(StaticLoad::run(&input).is_err());
}

/// 横の荷重倍数が 0 なら横荷重は 0 で、MS は無限大なので数を出さない（None。JSON は null で、TS の型と合う）。
/// 機軸は 50 N / (1.25 × 1 kg × 9.80665 × 2) − 1 = 1.0394。
#[test]
fn zero_load_has_no_margin() {
    let input = StaticLoadInput {
        items: vec![item("A", 1.0, "LV", None)],
        cases: vec![StaticLoadCase {
            id: "a".to_owned(),
            label: String::new(),
            axial: 2.0,
            lateral: 0.0,
        }],
        interfaces: vec![StaticLoadInterface {
            mount: "LV".to_owned(),
            allowable_axial: Some(Qty::from_raw(50.0)),
            allowable_lateral: Some(Qty::from_raw(10.0)),
            allowable_moment: None,
        }],
        safety_factor: None,
    };
    let out = StaticLoad::run(&input).unwrap();
    let lv = &out.mounts[0];
    assert_eq!(lv.margin_lateral, None);
    let ax = lv.margin_axial.unwrap().value();
    assert!((ax - (50.0 / (1.25 * G0 * 2.0) - 1.0)).abs() < 1e-12);
    assert_eq!(lv.min_margin, lv.margin_axial);
    assert_eq!(lv.verdict, Verdict::Ok);
    let json = serde_json::to_value(lv).unwrap();
    assert!(json["margin_lateral"].is_null());
}
