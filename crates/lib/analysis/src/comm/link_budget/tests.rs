use super::*;
use crate::spec::Verdict;
use sat_astro::constants::{C_LIGHT, SECONDS_PER_DAY};
use sat_astro::orbit::OrbitSpec;
use sat_astro::time::Epoch;
use sat_units::typed::DecibelPerKelvin;

fn db(v: f64) -> Decibel {
    Decibel::new(v)
}

fn uhf_down() -> LinkBudgetLink {
    LinkBudgetLink {
        name: "UHF down".to_owned(),
        station: "A".to_owned(),
        direction: LinkBudgetDirection::Downlink,
        rf: RfLinkParams {
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
        },
        data_rate: Qty::from_raw(9600.0),
        required_eb_n0_db: db(10.0),
        altitude: Some(Qty::from_raw(1.0e6)),
        central: CentralBody::Earth,
        elevation: Qty::from_raw(std::f64::consts::FRAC_PI_2),
        required_margin_db: None,
        pfd_limit_db: None,
        pfd_reference_bandwidth: None,
        occupied_bandwidth: None,
    }
}

/// 手計算: EIRP = 0 + 0 − 1 = −1 dBW、FSPL = 145.25 dB、その他 4 dB、G/T = 12 − 26.99 = −14.99 dB/K、
/// C/N0 = −1 − 149.25 − 14.99 + 228.60 = 63.36 dBHz、Eb/N0 = 63.36 − 39.82 = 23.54 dB、余裕 13.54 dB。
#[test]
fn hand_calculated_downlink() {
    let out = LinkBudget::run(&LinkBudgetInput {
        links: vec![uhf_down()],
        required_margin_db: None,
        orbit: None,
        duration: None,
    })
    .unwrap();
    let r = &out.links[0];
    assert!((r.slant_range.value() - 1.0e6).abs() < 1e-3);
    assert!((r.eirp_dbw.value() + 1.0).abs() < 1e-9);
    assert!((r.fspl_db.value() - 145.25).abs() < 0.02);
    assert!((r.g_over_t_dbk.value() + 14.99).abs() < 0.01);
    assert!(
        (r.c_over_n0_dbhz.value() - 63.36).abs() < 0.03,
        "{}",
        r.c_over_n0_dbhz.value()
    );
    assert!(
        (r.eb_n0_db.value() - 23.54).abs() < 0.03,
        "{}",
        r.eb_n0_db.value()
    );
    assert!((r.margin_db.value() - 13.54).abs() < 0.03);
    assert_eq!(out.checks[0].verdict, Verdict::Ok);
    // 最大レート: 10^((63.36 − 10 − 3)/10) ≈ 108 kbit/s
    let max = r.max_data_rate.value();
    assert!((max / 1.08e5 - 1.0).abs() < 0.01, "{max}");
}

#[test]
fn low_margin_is_flagged() {
    let mut link = uhf_down();
    link.rf.tx_power = Qty::from_raw(0.01); // −20 dB
    link.elevation = Qty::from_raw(0.0);
    let out = LinkBudget::run(&LinkBudgetInput {
        links: vec![link],
        required_margin_db: None,
        orbit: None,
        duration: None,
    })
    .unwrap();
    assert!(out.links[0].margin_db.value() < 0.0);
    assert_eq!(out.checks[0].verdict, Verdict::Violation);
}

/// 地球を回る衛星は局から見た斜距離: 高度 1000 km の最大は地平線 √(7378.1² − 6378.1²) km ≈ 3709.0 km、最小は天頂の 1000 km。
#[test]
fn earth_orbit_range_is_the_slant_range() {
    let mut link = uhf_down();
    link.elevation = Qty::from_raw(0.0);
    let input = LinkBudgetInput {
        links: vec![link],
        required_margin_db: None,
        orbit: None,
        duration: None,
    };
    let r = &LinkBudget::run(&input).unwrap().links[0];
    assert_eq!(r.distance_model, LinkDistanceModel::EarthOrbit);
    assert!((r.slant_range.value() - 3709.0e3).abs() < 0.5e3);
    assert!((r.range_min.value() - 1.0e6).abs() < 1e-3);
    assert!(r.range_min_epoch.is_none() && r.range_max_epoch.is_none());
    // 光の時間 = 距離 / c = 3709.0e3 / 299 792 458 = 12.37 ms
    assert!((r.light_time.value() - 0.012_37).abs() < 1e-5);
}

/// 月を回る衛星（高度 100 km）: 距離は地球（地心）から衛星まで。月の地心距離 3.56〜4.07 億 m に軌道半径 1.84e6 m を足し引きした範囲、
/// 最大の距離で回線余裕を判定し（FSPL は最大距離の値）、光の時間は 1.2〜1.4 s（3.84e8 m / c = 1.28 s）。
#[test]
fn moon_orbiter_uses_the_earth_to_moon_range() {
    let epoch = Epoch::parse("2027-04-01T00:00:00Z").unwrap();
    let orbit = OrbitSpec::circular_about(
        CentralBody::Moon,
        epoch,
        Qty::from_raw(100.0e3),
        Qty::from_raw(0.0),
        Qty::from_raw(0.0),
    );
    let mut link = uhf_down();
    link.central = CentralBody::Moon;
    link.altitude = Some(Qty::from_raw(100.0e3));
    let input = LinkBudgetInput {
        links: vec![link],
        required_margin_db: None,
        orbit: Some(orbit.into()),
        duration: Some(Qty::from_raw(30.0 * SECONDS_PER_DAY)),
    };
    let r = &LinkBudget::run(&input).unwrap().links[0];
    assert_eq!(r.distance_model, LinkDistanceModel::EarthToBody);
    let (min, max) = (r.range_min.value(), r.slant_range.value());
    assert!(3.5e8 < min && min < max && max < 4.1e8, "{min} {max}");
    assert!(r.range_min_epoch.is_some() && r.range_max_epoch.is_some());
    let expected_fspl = crate::comm::link::fspl_db(max, 437.0e6);
    assert!((r.fspl_db.value() - expected_fspl).abs() < 1e-9);
    assert!((r.light_time.value() - max / C_LIGHT).abs() < 1e-12);
    assert!((1.2..1.4).contains(&r.light_time.value()));
}

/// 中心天体が地球でないのに軌道が無ければ入力の誤り（地球の斜距離にすり替えない）。
#[test]
fn another_body_needs_the_orbit() {
    let mut link = uhf_down();
    link.central = CentralBody::Mars;
    let input = LinkBudgetInput {
        links: vec![link],
        required_margin_db: None,
        orbit: None,
        duration: None,
    };
    assert!(LinkBudget::run(&input).is_err());
}

/// PFD の手計算（高度 1000 km の天頂 d = 1000 km、EIRP = −1 dBW）: 4π d² = 1.2566e13 m² → 130.99 dB。
/// 占有 1 MHz・基準 4 kHz なら 23.98 dB 割り戻して PFD = −1 − 130.99 − 23.98 = −155.97 dBW/m²/4 kHz。
/// 限度 −150 は余裕 5.97 dB で ok、−155.5 は 0.47 dB で warning（1 dB 未満）、−160 は −4.03 dB で violation。
/// 限度が無ければ値だけ出して判定は作らない。上りは PFD を出さない。
#[test]
fn pfd_is_checked_only_when_a_limit_is_written() {
    let run = |limit: Option<f64>, direction| {
        let mut l = uhf_down();
        l.direction = direction;
        l.occupied_bandwidth = Some(Qty::from_raw(1.0e6));
        l.pfd_limit_db = limit.map(Decibel::new);
        LinkBudget::run(&LinkBudgetInput {
            links: vec![l],
            required_margin_db: None,
            orbit: None,
            duration: None,
        })
        .unwrap()
    };
    let down = LinkBudgetDirection::Downlink;
    let none = run(None, down);
    let pfd = none.links[0].pfd_dbw_m2.unwrap().value();
    assert!((pfd + 155.97).abs() < 0.01, "{pfd}");
    assert_eq!(
        none.links[0].pfd_reference_bandwidth.unwrap().value(),
        4_000.0
    );
    assert_eq!(none.checks.len(), 1, "only the margin check");
    for (limit, margin, verdict) in [
        (-150.0, 5.97, Verdict::Ok),
        (-155.5, 0.47, Verdict::Warning),
        (-160.0, -4.03, Verdict::Violation),
    ] {
        let out = run(Some(limit), down);
        let r = &out.links[0];
        assert!((r.pfd_margin_db.unwrap().value() - margin).abs() < 0.01);
        assert_eq!(out.checks.len(), 2);
        assert_eq!(out.checks[1].key, "pfd:UHF down@A");
        assert_eq!(out.checks[1].verdict, verdict);
    }
    let up = run(Some(-150.0), LinkBudgetDirection::Uplink);
    assert!(up.links[0].pfd_dbw_m2.is_none() && up.checks.len() == 1);
}

/// 地上局 2 つ × 下り回線 2 本（UHF 437 MHz / S 帯 2.2 GHz）の 4 組がすべて結果と判定に出る。
/// 手計算（高度 1000 km・仰角 90° の斜距離 1000 km）:
/// - UHF: FSPL 145.25 dB。局 A（G = 12 dBi, T = 500 K）の余裕は 13.54 dB（上のテストと同じ）。
///   局 B（G = 6 dBi）は G/T が 6 dB 下がるので 7.54 dB。
/// - S 帯（2.2 GHz）: FSPL = 145.25 + 20 log10(2200/437) = 145.25 + 14.04 = 159.29 dB。
///   局 A の余裕は UHF から (−14.04 dB) = −0.50 dB、局 B は −6.50 dB。
///
/// 余裕 3 dB を要求するので、UHF の 2 組は ok、S 帯の 2 組は violation。
#[test]
fn every_link_and_station_pair_is_evaluated() {
    let mut links = Vec::new();
    for (name, freq) in [("UHF down", 437.0e6), ("S down", 2.2e9)] {
        for (station, gain) in [("A", 12.0), ("B", 6.0)] {
            let mut l = uhf_down();
            l.name = name.to_owned();
            l.station = station.to_owned();
            l.rf.frequency = Qty::from_raw(freq);
            l.rf.rx_gain_db = Some(db(gain));
            links.push(l);
        }
    }
    let out = LinkBudget::run(&LinkBudgetInput {
        links,
        required_margin_db: None,
        orbit: None,
        duration: None,
    })
    .unwrap();
    let margins: Vec<(String, f64)> = out
        .links
        .iter()
        .map(|r| {
            let st = &r.station;
            (format!("{}@{st}", r.name), r.margin_db.value())
        })
        .collect();
    let expect = [
        ("UHF down@A", 13.54),
        ("UHF down@B", 7.54),
        ("S down@A", -0.50),
        ("S down@B", -6.50),
    ];
    assert_eq!(margins.len(), 4);
    for ((got_name, got), (name, want)) in margins.iter().zip(expect) {
        assert_eq!(got_name, name);
        assert!((got - want).abs() < 0.05, "{name}: {got}");
    }
    let verdicts: Vec<_> = out
        .checks
        .iter()
        .map(|c| (c.key.as_str(), c.verdict))
        .collect();
    assert_eq!(
        verdicts,
        [
            ("margin:UHF down@A", Verdict::Ok),
            ("margin:UHF down@B", Verdict::Ok),
            ("margin:S down@A", Verdict::Violation),
            ("margin:S down@B", Verdict::Violation),
        ]
    );
    assert!(out.checks[2].label.contains("S down@A"));
}

/// 与えた G/T を使う。−10 dB/K（導出値 −14.99 dB/K より 4.99 dB 高い）なら余裕も 4.99 dB 増えて 18.53 dB。
#[test]
fn given_g_over_t_is_used_and_reported() {
    let run = |g_t: Option<f64>| {
        let mut l = uhf_down();
        l.rf.rx_g_over_t_dbk = g_t.map(DecibelPerKelvin::new);
        LinkBudget::run(&LinkBudgetInput {
            links: vec![l],
            required_margin_db: None,
            orbit: None,
            duration: None,
        })
        .unwrap()
        .links
        .remove(0)
    };
    let (derived, given) = (run(None), run(Some(-10.0)));
    assert!(!derived.g_over_t_given && given.g_over_t_given);
    assert_eq!(given.g_over_t_dbk.value(), -10.0);
    assert!((given.margin_db.value() - 18.53).abs() < 0.03);
    assert!((given.margin_db.value() - derived.margin_db.value() - 4.99).abs() < 0.01);
}

/// 受信系は、G/T を与えるか、利得と雑音温度の両方を与えるかのどちらかが要る（どちらも無ければ入力の誤り）。
/// G/T だけで足りるときは利得・雑音温度が無くても、与えた G/T どおりに計算できる。
#[test]
fn receiver_needs_g_over_t_or_gain_and_noise() {
    let run = |gain: Option<f64>, noise: Option<f64>, g_t: Option<f64>| {
        let mut l = uhf_down();
        l.rf.rx_gain_db = gain.map(db);
        l.rf.system_noise_temperature = noise.map(Qty::from_raw);
        l.rf.rx_g_over_t_dbk = g_t.map(DecibelPerKelvin::new);
        LinkBudget::run(&LinkBudgetInput {
            links: vec![l],
            required_margin_db: None,
            orbit: None,
            duration: None,
        })
    };
    assert!(run(None, None, None).is_err());
    assert!(run(Some(12.0), None, None).is_err());
    assert!(run(None, Some(500.0), None).is_err());
    let alone = run(None, None, Some(-10.0)).unwrap();
    assert!((alone.links[0].margin_db.value() - 18.53).abs() < 0.03);
    assert!(alone.links[0].g_over_t_given);
}
