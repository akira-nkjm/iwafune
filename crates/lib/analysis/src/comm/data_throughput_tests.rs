//! `comm.data_throughput` のテスト（手計算・送信可能量の保存）。

use super::*;
use crate::comm::passes::fixtures::{sso_orbit, tokyo};

#[test]
fn backlog_hand_calculation() {
    // 発生 1 bit/s、送信 3 bit/s。0〜10 s 待機 → 10、10〜20 s 送信: 5 s で空になり以後 0。
    let mut b = Backlog {
        t: 0.0,
        value: 0.0,
        gen_bps: 1.0,
        points: vec![(0.0, 0.0)],
    };
    b.idle(10.0);
    assert_eq!(b.value, 10.0);
    let sent = b.downlink(20.0, 3.0);
    assert!((sent - 20.0).abs() < 1e-12);
    assert!(b.value.abs() < 1e-12);
    assert!(b.points.contains(&(15.0, 0.0)));
}

fn input() -> DataThroughputInput {
    DataThroughputInput {
        sampling: OrbitSampling::new(
            sso_orbit(),
            Qty::from_raw(2.0 * SECONDS_PER_DAY),
            Qty::from_raw(30.0),
        ),
        stations: vec![tokyo()],
        downlink_rate: Qty::from_raw(9600.0),
        usable_pass_fraction: Some(Fraction::new(0.8)),
        generated_per_day: Qty::from_raw(50.0e6),
        initial_backlog: None,
        storage_capacity: Some(Qty::from_raw(8.0e9)),
        link: None,
        required_margin_db: None,
    }
}

#[test]
fn capacity_is_rate_times_assigned_time() {
    let out = DataThroughput::run(&input()).unwrap();
    let assigned: f64 = out.passes.iter().map(|p| p.assigned.value()).sum();
    let total: f64 = out.days.iter().map(|d| d.capacity.value()).sum();
    assert!((total - 9600.0 * 0.8 * assigned).abs() < 1e-3 * total.max(1.0));
    assert_eq!(out.days.len(), 2);
    assert_eq!(out.usable_pass_fraction.value(), 0.8);
    // 発生 50 Mbit/日に対し、9.6 kbit/s × 数十分/日 ≈ 十数 Mbit/日なので溜まり続ける（送信の余裕は負）。
    // 余裕 = 1 日の送信可能量 / 発生量 − 1
    let margin = out.downlink_margin.unwrap().value();
    let expect = out.capacity_per_day.value() / 50.0e6 - 1.0;
    assert!((margin - expect).abs() < 1e-12 && margin < 0.0);
    assert!(out.final_backlog.value() > 0.0);
    let gen_total: f64 = out.days.iter().map(|d| d.generated.value()).sum();
    let sent: f64 = out.days.iter().map(|d| d.downlinked.value()).sum();
    assert!((gen_total - sent - out.final_backlog.value()).abs() < 1e-3);
    assert_eq!(out.link_closure, DataThroughputLinkClosure::NotEvaluated);
    assert_eq!(out.required_margin_db, None);
}

/// 発生量が 0 なら送信の余裕は無限大なので数を出さない（None。JSON は null で、TS の `number | null` と合う）。
#[test]
fn no_generation_has_no_margin() {
    let mut i = input();
    i.generated_per_day = Qty::ZERO;
    let out = DataThroughput::run(&i).unwrap();
    assert_eq!(out.downlink_margin, None);
    let json = serde_json::to_value(&out).unwrap();
    assert!(json["downlink_margin"].is_null());
}

fn uhf() -> RfLinkParams {
    let db = Decibel::new;
    RfLinkParams {
        frequency: Qty::from_raw(437.0e6),
        tx_power: Qty::from_raw(1.0),
        tx_gain_db: db(0.0),
        tx_loss_db: Some(db(1.0)),
        rx_gain_db: db(12.0),
        rx_loss_db: Some(db(0.0)),
        system_noise_temperature: Qty::from_raw(500.0),
        atmospheric_loss_db: Some(db(0.5)),
        polarization_loss_db: Some(db(3.0)),
        pointing_loss_db: Some(db(0.5)),
        other_loss_db: Some(db(0.0)),
    }
}

/// 所要 Eb/N0 20 dB・余裕 3 dB は、550 km で水平線（Eb/N0 ≈ 15 dB）では閉じず天頂（≈ 29 dB）では閉じる。
/// 求めた最低仰角では余裕がちょうど 3 dB になり、局の最低仰角が上がって数える時間が減る。
/// 所要 40 dB は天頂でも閉じないので、パスなしで違反になる。
#[test]
fn link_closure_raises_min_elevation() {
    let base = DataThroughput::run(&input()).unwrap();
    let mut with_link = input();
    with_link.link = Some(DataThroughputLink {
        rf: uhf(),
        required_eb_n0_db: Decibel::new(20.0),
    });
    let out = DataThroughput::run(&with_link).unwrap();
    let DataThroughputLinkClosure::ClosesAbove { min_elevation } = out.link_closure else {
        panic!("{:?}", out.link_closure);
    };
    let el = min_elevation.value();
    assert!(
        el > 10.0_f64.to_radians() && el < 80.0_f64.to_radians(),
        "{el}"
    );
    let range = sat_astro::ground::slant_range_m(sat_astro::constants::R_EARTH, 550.0e3, el);
    let margin = uhf().eb_n0_margin_db(range, 9600.0, Decibel::new(20.0));
    assert!((margin - 3.0).abs() < 1e-6, "{margin}");
    let total = |o: &DataThroughputOutput| o.passes.iter().map(|p| p.assigned.value()).sum::<f64>();
    assert!(total(&out) < total(&base));
    // 使った要求余裕は既定の 3 dB
    assert_eq!(out.required_margin_db, Some(Decibel::new(3.0)));

    let mut never = with_link;
    if let Some(link) = never.link.as_mut() {
        link.required_eb_n0_db = Decibel::new(40.0);
    }
    let out = DataThroughput::run(&never).unwrap();
    assert_eq!(out.link_closure, DataThroughputLinkClosure::NeverCloses);
    assert!(out.passes.is_empty());
    let c = out.checks.iter().find(|c| c.key == "link_closes").unwrap();
    assert_eq!(c.verdict, Verdict::Violation);
}
