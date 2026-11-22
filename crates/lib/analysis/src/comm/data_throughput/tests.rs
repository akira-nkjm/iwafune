//! `comm.data_throughput` のテスト（手計算・送信可能量の保存）。

use super::*;
use crate::comm::passes::fixtures::{sso_orbit, tokyo};
use crate::spec::Verdict;
use sat_astro::ground::GroundStation as Station;

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
        station_links: Vec::new(),
        stations_without_band: Vec::new(),
        downlink: None,
        other_downlinks: Vec::new(),
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
        rx_gain_db: Some(db(12.0)),
        rx_loss_db: Some(db(0.0)),
        system_noise_temperature: Some(Qty::from_raw(500.0)),
        rx_g_over_t_dbk: None,
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
    with_link.station_links = vec![DataThroughputStationLink {
        station: "TKY".to_owned(),
        link: DataThroughputLink {
            rf: uhf(),
            required_eb_n0_db: Decibel::new(20.0),
        },
    }];
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
    if let Some(link) = never.station_links.first_mut() {
        link.link.required_eb_n0_db = Decibel::new(40.0);
    }
    let out = DataThroughput::run(&never).unwrap();
    assert_eq!(out.link_closure, DataThroughputLinkClosure::NeverCloses);
    assert!(out.passes.is_empty());
    let c = out.checks.iter().find(|c| c.key == "link_closes").unwrap();
    assert_eq!(c.verdict, Verdict::Violation);
}

/// 東京とは経度の違う 2 局目（最低仰角 10°）。
fn second_station() -> Station {
    let mut st = tokyo();
    st.id = "SVB".to_owned();
    st.latitude = Qty::from_raw(78.2_f64.to_radians());
    st.longitude = Qty::from_raw(15.4_f64.to_radians());
    st
}

/// 2 局を渡すと両方のパスを数え、同時に見える時間は二重に数えない（割り当てた時間の合計 = 区間の和集合の長さ）。
/// 高緯度の局が加わるので、東京だけより数える時間は増える。
#[test]
fn two_stations_are_counted_without_double_counting() {
    let one = DataThroughput::run(&input()).unwrap();
    let mut two_in = input();
    two_in.stations.push(second_station());
    let two = DataThroughput::run(&two_in).unwrap();
    let stations: std::collections::BTreeSet<_> =
        two.passes.iter().map(|p| p.station.as_str()).collect();
    assert_eq!(stations.len(), 2, "{stations:?}");
    let total = |o: &DataThroughputOutput| o.passes.iter().map(|p| p.assigned.value()).sum::<f64>();
    assert!(total(&two) > total(&one));
    let w = crate::comm::passes::search_window(&two_in.sampling).unwrap();
    let mut spans: Vec<(f64, f64)> = two
        .passes
        .iter()
        .map(|p| {
            let at = |e: Epoch| (e.j2000_s - w.start.j2000_s).clamp(0.0, w.span_s);
            (at(p.aos), at(p.los))
        })
        .collect();
    spans.sort_by(|a, b| a.0.total_cmp(&b.0));
    let union: f64 = crate::comm::passes::union_sorted(&spans)
        .iter()
        .map(|(s, e)| e - s)
        .sum();
    assert!(
        (total(&two) - union).abs() < 1e-6,
        "{} vs {union}",
        total(&two)
    );
}

/// 局ごとの回線の条件: 東京は閉じる回線（所要 20 dB）、2 局目は閉じない回線（所要 40 dB）。
/// 2 局目は数えず（警告で名前を出す）、全体の閉じ方は閉じる局のものになる。
#[test]
fn station_links_close_per_station() {
    let mut i = input();
    i.stations.push(second_station());
    let link = |req: f64| DataThroughputLink {
        rf: uhf(),
        required_eb_n0_db: Decibel::new(req),
    };
    i.station_links = vec![
        DataThroughputStationLink {
            station: "TKY".to_owned(),
            link: link(20.0),
        },
        DataThroughputStationLink {
            station: "SVB".to_owned(),
            link: link(40.0),
        },
    ];
    let out = DataThroughput::run(&i).unwrap();
    assert!(matches!(
        out.link_closure,
        DataThroughputLinkClosure::ClosesAbove { .. }
    ));
    assert_eq!(out.station_closures.len(), 2);
    assert_eq!(
        out.station_closures[1].closure,
        DataThroughputLinkClosure::NeverCloses
    );
    assert!(out.passes.iter().all(|p| p.station == "TKY"));
    let c = out
        .checks
        .iter()
        .find(|c| c.key == "link_closes:SVB")
        .unwrap();
    assert_eq!(c.verdict, Verdict::Warning);
    assert!(out.checks.iter().all(|c| c.key != "link_closes"));
}

/// 数えていない下り回線は判定（警告）で名前が分かる。足し合わせはしない。
#[test]
fn uncounted_downlinks_are_reported() {
    let mut i = input();
    i.downlink = Some("s_down".to_owned());
    i.other_downlinks = vec!["uhf_down".to_owned()];
    let out = DataThroughput::run(&i).unwrap();
    let c = out
        .checks
        .iter()
        .find(|c| c.key == "downlinks_not_counted")
        .unwrap();
    assert_eq!(c.verdict, Verdict::Ok);
    assert!(c.message.contains("uhf_down") && c.message.contains("s_down"));
    // 数える量は入力のレートのまま（回線を足し合わせていない）
    let base = DataThroughput::run(&input()).unwrap();
    assert_eq!(out.capacity_per_day.value(), base.capacity_per_day.value());
}

/// 帯域の設定が無い局は数えず、警告も出さず、`station_closures` に `no_band_settings` で残す。
#[test]
fn stations_without_the_band_are_listed_not_counted() {
    let mut i = input();
    i.stations_without_band = vec!["SVB".to_owned()];
    let out = DataThroughput::run(&i).unwrap();
    let last = out.station_closures.last().unwrap();
    assert_eq!(last.station, "SVB");
    assert_eq!(last.closure, DataThroughputLinkClosure::NoBandSettings);
    assert!(out.passes.iter().all(|p| p.station == "TKY"));
    assert!(out.checks.iter().all(|c| !c.key.starts_with("link_closes")));
    assert_eq!(out.link_closure, DataThroughputLinkClosure::NotEvaluated);
}

/// 月を回る衛星: 距離は地球から衛星まで（約 3.6〜4.1 億 m）で仰角によらないので、回線は「水平線から閉じる」か
/// 「閉じない」かの二択になる。手計算（最大距離 4e8 m）: FSPL = 20 log10(4π × 4e8 × 437e6 / c) = 197.3 dB、
/// C/N0 = −1（EIRP）− 197.3 − 4（その他の損失）− 14.99（G/T）+ 228.6 = 11.3 dBHz、Eb/N0 = 11.3 − 39.8 = −28.5 dB。
/// 余裕 3 dB を残すには所要 −31.5 dB 以下が要るので、所要 −60 dB なら閉じ（水平線 0°）、所要 20 dB なら閉じない。
#[test]
fn moon_orbiter_closure_uses_the_earth_distance() {
    let orbit = sat_astro::orbit::OrbitSpec::circular_about(
        sat_astro::ephem::CentralBody::Moon,
        sso_orbit().epoch,
        Qty::from_raw(100.0e3),
        Qty::from_raw(1.0),
        Qty::from_raw(0.0),
    );
    let run = |required: f64| {
        let mut i = input();
        i.sampling = OrbitSampling::new(
            orbit.clone(),
            Qty::from_raw(SECONDS_PER_DAY),
            Qty::from_raw(60.0),
        );
        i.station_links = vec![DataThroughputStationLink {
            station: "TKY".to_owned(),
            link: DataThroughputLink {
                rf: uhf(),
                required_eb_n0_db: Decibel::new(required),
            },
        }];
        DataThroughput::run(&i).unwrap()
    };
    let closes = run(-60.0);
    assert_eq!(
        closes.link_closure,
        DataThroughputLinkClosure::ClosesAbove {
            min_elevation: Qty::ZERO
        }
    );
    assert!(!closes.passes.is_empty());
    let never = run(20.0);
    assert_eq!(never.link_closure, DataThroughputLinkClosure::NeverCloses);
}
