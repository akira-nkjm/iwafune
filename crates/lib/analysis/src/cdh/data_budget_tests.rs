//! `cdh.data_budget` のテスト（手計算との比較）。

use super::*;
use crate::modes::StateAssignment;

fn rates(pairs: &[(&str, f64)]) -> StateValues<DataRate> {
    StateValues {
        default: None,
        by_state: pairs
            .iter()
            .map(|(s, r)| ((*s).to_owned(), Qty::from_raw(*r)))
            .collect(),
    }
}

fn assign(part: &str, state: &str, duty: &[(&str, f64)]) -> StateAssignment {
    StateAssignment {
        part: part.to_owned(),
        state: state.to_owned(),
        duty: duty.iter().map(|(s, f)| ((*s).to_owned(), *f)).collect(),
    }
}

fn input() -> DataBudgetInput {
    DataBudgetInput {
        parts: vec![
            DataBudgetPart {
                count: 1,
                id: "OBC".to_owned(),
                hk_rate: StateValues {
                    default: Some(Qty::from_raw(100.0)),
                    by_state: Default::default(),
                },
                mission_rate: StateValues::default(),
            },
            DataBudgetPart {
                count: 1,
                id: "CAM".to_owned(),
                hk_rate: rates(&[("off", 0.0), ("on", 50.0)]),
                mission_rate: rates(&[("off", 0.0), ("on", 1.0e6)]),
            },
        ],
        modes: vec![
            ModeDef {
                id: "NOM".to_owned(),
                label: "通常".to_owned(),
                assignments: vec![assign("OBC", "on", &[]), assign("CAM", "off", &[])],
            },
            ModeDef {
                id: "OBS".to_owned(),
                label: "観測".to_owned(),
                assignments: vec![
                    assign("OBC", "on", &[]),
                    assign("CAM", "off", &[("on", 0.5)]),
                ],
            },
        ],
        mode_shares: vec![
            DataBudgetModeShare {
                mode: "NOM".to_owned(),
                fraction: 0.9,
            },
            DataBudgetModeShare {
                mode: "OBS".to_owned(),
                fraction: 0.1,
            },
        ],
        storage: vec![DataBudgetStorage {
            part: "OBC".to_owned(),
            capacity: Qty::from_raw(16.0e9),
            count: 2,
        }],
        retention: Qty::from_raw(3.0 * SECONDS_PER_DAY),
        downlink_per_day: Some(Qty::from_raw(1.0e9)),
        min_storage_margin: None,
        compression_ratio: None,
        hk_overhead: None,
    }
}

/// 手計算: HK = (100 + 0.1·0.5·50) × 86400 = 8.856 Mbit、
/// ミッション = 0.1 × 0.5 × 1 Mbit/s × 86400 = 4.32 Gbit、3 日保持で 12.98 Gbit（32 Gbit の 40.6 %）。
#[test]
fn hand_calculated_budget() {
    let out = DataBudget::run(&input()).unwrap();
    assert!((out.hk_per_day.value() - 102.5 * SECONDS_PER_DAY).abs() < 1e-6);
    assert!((out.mission_per_day.value() - 0.05e6 * SECONDS_PER_DAY).abs() < 1e-3);
    let total = 102.5 * SECONDS_PER_DAY + 0.05e6 * SECONDS_PER_DAY;
    assert!((out.storage_required.value() - 3.0 * total).abs() < 1e-3);
    assert!((out.storage_margin.value() - (1.0 - 3.0 * total / 32.0e9)).abs() < 1e-12);
    // 16 Gbit × 2 台 = 32 Gbit、要求余裕は既定の 0.2
    assert_eq!(out.storage_capacity.value(), 32.0e9);
    assert_eq!(out.min_storage_margin.value(), DEFAULT_MIN_STORAGE_MARGIN);
    assert!((out.modes[1].mission_rate.value() - 0.5e6).abs() < 1e-9);
    let cam = &out.parts[1];
    assert!((cam.share.value() - (0.05e6 + 2.5) / (0.05e6 + 102.5)).abs() < 1e-12);
    let storage = out
        .checks
        .iter()
        .find(|c| c.key == "storage_margin")
        .unwrap();
    assert_eq!(storage.verdict, Verdict::Ok);
    let downlink = out.checks.iter().find(|c| c.key == "downlink").unwrap();
    assert_eq!(downlink.verdict, Verdict::Violation);
}

#[test]
fn unassigned_part_warns() {
    let mut inp = input();
    inp.modes[0].assignments.retain(|a| a.part != "CAM");
    let out = DataBudget::run(&inp).unwrap();
    let c = out
        .checks
        .iter()
        .find(|c| c.key == "hk:unassigned:NOM")
        .unwrap();
    assert!(c.message.contains("CAM"));
    assert!(out.checks.iter().any(|c| c.key == "mission:unassigned:NOM"));
}

#[test]
fn unknown_mode_is_an_error() {
    let mut inp = input();
    inp.mode_shares[0].mode = "NOPE".to_owned();
    assert!(DataBudget::run(&inp).is_err());
}

/// 圧縮率 0.25 はミッションデータを 1/4 に、HK の余分 0.1 は HK を 1.1 倍にする。
#[test]
fn compression_and_overhead() {
    let base = DataBudget::run(&input()).unwrap();
    let mut inp = input();
    inp.compression_ratio = Some(0.25);
    inp.hk_overhead = Some(0.1);
    let out = DataBudget::run(&inp).unwrap();
    assert!((out.mission_per_day.value() - 0.25 * base.mission_per_day.value()).abs() < 1e-6);
    assert!((out.hk_per_day.value() - 1.1 * base.hk_per_day.value()).abs() < 1e-6);
}

#[test]
fn rates_and_storage_use_per_unit_counts() {
    let mut i = input();
    let one = DataBudget::run(&i).unwrap();
    for p in &mut i.parts {
        p.count = 3;
    }
    for s in &mut i.storage {
        s.count *= 3;
    }
    let three = DataBudget::run(&i).unwrap();
    assert!((three.total_per_day.value() - 3.0 * one.total_per_day.value()).abs() < 1e-6);
    assert_eq!(
        three.storage_capacity.value(),
        3.0 * one.storage_capacity.value()
    );
}
