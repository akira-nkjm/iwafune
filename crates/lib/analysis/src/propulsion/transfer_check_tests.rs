//! 日付つきの遷移の多体の確認（`propulsion.transfer_check`）。Curtis Example 8.8（1996-11-07 → 1997-09-12 の地球 → 火星）を
//! 解析式の暦（このクレートのテストは DE440 を登録しない）で回す。伝搬の検証（太陽だけで Lambert の到着に戻る・保存量・
//! 摂動の見積もり）は `sat_astro::transfer::cruise` のテストにある。ここは結果の組み立てと判定を見る。

use super::dated_tests::mars_input;
use super::tests::{input, maneuver};
use super::transfer_check::judge;
use super::*;
use crate::spec::{Analysis, ChannelKind, Verdict, quantity_text};
use sat_astro::ephem::{CentralBody, EphemerisSource};
use sat_astro::time::Epoch;
use sat_units::typed::Length;

fn run_curtis() -> TransferCheckOutput {
    TransferCheck::run(&mars_input(97.0, "1996-11-07T00:00:00Z")).unwrap()
}

/// 解析式の暦では数値は出すが判定しない（判定も無い）。出所・日付・飛行時間は遷移のもの。
#[test]
fn analytic_ephemeris_reports_numbers_without_judging() {
    let out = run_curtis();
    assert_eq!(out.transfers.len(), 1);
    let t = &out.transfers[0];
    assert_eq!(t.maneuver, "MARS");
    assert_eq!((t.from, t.to), (CentralBody::Earth, CentralBody::Mars));
    assert_eq!(t.ephemeris, EphemerisSource::Analytic);
    assert!(!t.judged);
    assert!(t.checks.is_empty(), "{:?}", t.checks);
    assert!(
        t.notes
            .contains(&TransferCheckNote::AnalyticEphemerisNotJudged)
    );
    assert!(t.notes.contains(&TransferCheckNote::PointMassCruise));
    assert!(!t.notes.contains(&TransferCheckNote::ClosestApproachAtEnd));
    // 影響圏は火星の a (m/M)^(2/5) ≈ 5.8e8 m、ずれはその比
    let soi = t.arrival_soi_radius.unwrap().value();
    assert!((5.0e8..7.0e8).contains(&soi), "{soi}");
    let ratio = t.miss_in_soi_radii.unwrap();
    assert!((ratio - t.miss_distance.value() / soi).abs() < 1e-12);
    assert!(t.miss_distance.value() > 0.0);
    // 最接近は到着のずれ以下で、時刻は到着の日の前後（飛行時間 309 日の ±10 日）
    assert!(t.closest_approach_distance.value() <= t.miss_distance.value() + 1e-6);
    let days = t.closest_approach_time.value() / 86_400.0;
    assert!((299.0..=319.0).contains(&days), "{days}");
    assert!(
        (t.closest_approach_epoch.seconds_since(t.departure) - t.closest_approach_time.value())
            .abs()
            < 1e-6
    );
}

/// 到着の超過速度は、`propulsion.maneuvers` の遷移（Lambert）の値と同じで、伝搬との差は惑星の摂動の分（小さい）。
#[test]
fn arrival_v_inf_follows_the_lambert_transfer() {
    let i = mars_input(97.0, "1996-11-07T00:00:00Z");
    let lambert = Maneuvers::run(&i).unwrap().maneuvers[0]
        .dated_transfer
        .clone()
        .unwrap();
    let t = &TransferCheck::run(&i).unwrap().transfers[0];
    assert_eq!(t.v_inf_arrival_lambert, lambert.v_inf_arrival);
    let diff = t.v_inf_arrival_difference.value();
    assert!((0.0..50.0).contains(&diff), "{diff}");
    assert!(
        (t.v_inf_arrival_propagated.value() - t.v_inf_arrival_lambert.value()).abs() <= diff + 1e-9
    );
    // 第三体は地球（出発の端）を除く惑星
    assert!(!t.perturbers.contains(&CentralBody::Earth));
    assert!(!t.perturbers.contains(&CentralBody::Mars));
    assert!(t.perturbers.contains(&CentralBody::Jupiter));
}

/// 軌跡の時系列: 出発の日から到着の日まで、宇宙機・出発天体・到着天体の x・y と到着天体までの距離。
#[test]
fn trajectory_series_runs_from_departure_to_arrival() {
    let t = &run_curtis().transfers[0];
    let s = &t.series;
    assert_eq!(s.t.first(), Some(&0.0));
    let tof = 309.0 * 86_400.0;
    assert!((s.t.last().unwrap() - tof).abs() < 1e-6);
    assert_eq!(s.channels.len(), 7);
    let find = |key: &str| s.channels.iter().find(|c| c.key == key).unwrap();
    for c in &s.channels {
        assert_eq!(c.values.len(), s.t.len(), "{}", c.key);
        assert_eq!(c.dimension, "Length");
    }
    // 出発では宇宙機は出発天体の中心、日心の距離は約 1 AU
    assert_eq!(
        find("spacecraft_x").values[0],
        find("departure_x").values[0]
    );
    let r0 = find("departure_x").values[0].hypot(find("departure_y").values[0]);
    assert!((r0 / 1.496e11 - 1.0).abs() < 0.03, "{r0}");
    assert_eq!(find("range_to_arrival").kind, ChannelKind::ArrivalRange);
    assert_eq!(find("spacecraft_x").kind, ChannelKind::HeliocentricX);
    assert_eq!(find("spacecraft_x").subject.as_deref(), Some("spacecraft"));
    // 到着の日の距離は、ずれ（3 次元）に一致
    let last = find("range_to_arrival").values.last().copied().unwrap();
    assert!((last - t.miss_distance.value()).abs() < 1e-3);
}

/// 日付つきの遷移が無ければ、何も当てはまらない（空の結果。誤りでも不足の入力でもない）。
#[test]
fn without_a_dated_transfer_nothing_applies() {
    let out = TransferCheck::run(&input(vec![maneuver(
        "RAISE",
        DvCategory::Insertion,
        TransferMethod::Impulsive,
        500.0,
    )]))
    .unwrap();
    assert!(out.transfers.is_empty());
    let mut i = mars_input(97.0, "1996-11-07T00:00:00Z");
    i.maneuvers[0].transfer = None;
    assert!(TransferCheck::run(&i).unwrap().transfers.is_empty());
}

/// 暦の外の日付は、`propulsion.maneuvers` と同じく解析の誤り（黙って計算しない）。
#[test]
fn dates_outside_the_ephemeris_are_errors() {
    let mut i = mars_input(97.0, "1996-11-07T00:00:00Z");
    i.maneuvers[0].transfer.as_mut().unwrap().arrival =
        Epoch::parse("2120-01-01T00:00:00Z").unwrap();
    assert!(Maneuvers::run(&i).is_err());
    assert!(TransferCheck::run(&i).is_err());
}

/// 判定（DE440 のときの規則）: ずれが影響圏の半径以内なら Ok、外なら警告。最接近も同じ。数は `quantity_text` の文字列。
#[test]
fn verdict_rule_compares_miss_and_closest_approach_with_the_soi() {
    let soi = 5.77e8;
    let ok = judge("MARS", CentralBody::Mars, (3.0e8, 1.0e8, soi));
    assert_eq!(ok.len(), 2);
    assert!(ok.iter().all(|c| c.verdict == Verdict::Ok));
    assert!(ok[0].message.contains(&quantity_text::<Length>(3.0e8)));
    let far = judge("MARS", CentralBody::Mars, (9.0e8, 8.0e8, soi));
    assert_eq!(far[0].key, "transfer_check_arrival_miss");
    assert_eq!(far[0].verdict, Verdict::Warning);
    assert_eq!(far[1].key, "transfer_check_closest_approach");
    assert_eq!(far[1].verdict, Verdict::Warning);
    assert!(far[0].message.contains("trajectory correction"));
    // ずれは外でも、最接近は影響圏の中（到着の日がずれているだけ）
    let timing = judge("MARS", CentralBody::Mars, (9.0e8, 2.0e8, soi));
    assert_eq!(timing[0].verdict, Verdict::Warning);
    assert_eq!(timing[1].verdict, Verdict::Ok);
    // 境界は含む
    assert_eq!(
        judge("M", CentralBody::Mars, (soi, soi, soi))[0].verdict,
        Verdict::Ok
    );
}

/// 登録: カタログに載り、同じ入力 `PropulsionInput`、系は推進系。
#[test]
fn registered_in_the_catalog_as_a_propulsion_analysis() {
    let info = crate::catalog::catalog()
        .into_iter()
        .find(|i| i.id == "propulsion.transfer_check")
        .expect("registered");
    assert_eq!(info.title_en, "Transfer check (n-body)");
    let json = serde_json::to_value(mars_input(97.0, "1996-11-07T00:00:00Z")).unwrap();
    let out = crate::catalog::run_json("propulsion.transfer_check", json).unwrap();
    assert_eq!(out["transfers"].as_array().unwrap().len(), 1);
}
