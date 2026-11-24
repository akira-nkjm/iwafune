//! ほかの天体を回る衛星の回線（両端の距離・距離ごとのデータレート・合・光の伝搬時間）のテスト。
//! 期待値は導出（式・手計算・恒等式）だけから決め、許容誤差を値に合わせて調整しない。

use super::*;
use crate::spec::Verdict;
use sat_astro::constants::{AU, C_LIGHT, SECONDS_PER_DAY};
use sat_astro::ephem::{Body, eci_to_ecliptic_j2000, ecliptic_normal_eci, position_eci_m};
use sat_astro::orbit::{OrbitElements, OrbitSpec, Propagator};
use sat_astro::timeline::geometry::light_time_s;
use std::f64::consts::PI;

fn epoch() -> Epoch {
    Epoch::parse("2027-04-01T00:00:00Z").expect("epoch")
}

fn db(v: f64) -> Decibel {
    Decibel::new(v)
}

/// 太陽中心の黄道面の円軌道で、地球の黄道経度 + `offset` から始まる（半径 `a_m`）。
fn heliocentric_circle(a_m: f64, offset: f64) -> OrbitSpec {
    let s = position_eci_m(Body::Sun, epoch());
    let e = eci_to_ecliptic_j2000(-s);
    let elements = OrbitElements {
        semi_major_axis: Qty::from_raw(a_m),
        eccentricity: 0.0,
        inclination: Qty::from_raw(ecliptic_normal_eci().z.acos()),
        raan: Qty::from_raw(0.0),
        arg_perigee: Qty::from_raw(0.0),
        mean_anomaly: Qty::from_raw(e.y.atan2(e.x) + offset),
    };
    let mut o = OrbitSpec::new(epoch(), elements, Propagator::Kepler);
    o.central = CentralBody::Sun;
    o
}

/// X 帯の下り（深宇宙の探査機）の例: 8.4 GHz、10 W、利得 30 dBi、地上の G/T 50 dB/K を直接与える。
fn deep_link() -> LinkBudgetLink {
    LinkBudgetLink {
        name: "X down".to_owned(),
        station: "A".to_owned(),
        direction: LinkBudgetDirection::Downlink,
        rf: RfLinkParams {
            frequency: Qty::from_raw(8.4e9),
            tx_power: Qty::from_raw(10.0),
            tx_gain_db: db(30.0),
            tx_loss_db: None,
            rx_gain_db: None,
            rx_loss_db: None,
            system_noise_temperature: None,
            rx_g_over_t_dbk: Some(DecibelPerKelvin::new(50.0)),
            atmospheric_loss_db: None,
            polarization_loss_db: None,
            pointing_loss_db: None,
            other_loss_db: None,
        },
        data_rate: Qty::from_raw(1000.0),
        required_eb_n0_db: db(2.0),
        altitude: Some(Qty::from_raw(1.0e9)),
        central: CentralBody::Sun,
        elevation: Qty::from_raw(0.0),
        required_margin_db: None,
        pfd_limit_db: None,
        pfd_reference_bandwidth: None,
        occupied_bandwidth: None,
    }
}

fn run_deep(offset: f64, days: f64) -> LinkBudgetOutput {
    LinkBudget::run(&LinkBudgetInput {
        links: vec![deep_link()],
        required_margin_db: None,
        orbit: Some(heliocentric_circle(1.5 * AU, offset).into()),
        duration: Some(Qty::from_raw(days * SECONDS_PER_DAY)),
    })
    .expect("run")
}

/// 距離が 2 倍になると、同じデータレートの余裕は 20 log10(2) = 6.0206 dB 下がり、通せる最大のデータレートは 1/4 になる。
#[test]
fn doubling_the_distance_costs_6_db_and_a_factor_of_four_in_rate() {
    let link = deep_link();
    let required = db(3.0);
    let near = at_range::at_range(&link, required, 1.0e11);
    let far = at_range::at_range(&link, required, 2.0e11);
    assert!((near.margin_db - far.margin_db - 20.0 * 2.0_f64.log10()).abs() < 1e-9);
    assert!((near.max_data_rate_bps / far.max_data_rate_bps - 4.0).abs() < 1e-9);
}

/// 通せる最大のデータレートをそのまま回線のデータレートにすると、余裕は要求余裕にぴったり一致する。
#[test]
fn the_supportable_rate_gives_exactly_the_required_margin() {
    let mut link = deep_link();
    let required = db(3.0);
    for range in [5.0e10, 4.0e11] {
        link.data_rate =
            Qty::from_raw(at_range::at_range(&link, required, range).max_data_rate_bps);
        let m = at_range::at_range(&link, required, range).margin_db;
        assert!((m - required.value()).abs() < 1e-9, "{m}");
    }
}

/// 1 au の片道の光の伝搬時間は 499.005 s（距離 / 光速）。
#[test]
fn light_time_of_one_au_is_499_seconds() {
    assert!((light_time_s(AU) - 499.005).abs() < 1e-3);
    assert!((light_time_s(AU) - AU / C_LIGHT).abs() < 1e-12);
}

/// 両端: 最小の距離の余裕は最大の距離より 20 log10(最大 / 最小) だけ大きく、通せるレートは (最大 / 最小)² 倍、
/// 光の伝搬時間は距離 / 光速。最大の距離の値は従来のフィールドと一致する。
#[test]
fn both_ends_of_the_range_follow_the_inverse_square_law() {
    let out = run_deep(PI, 400.0);
    let r = &out.links[0];
    let (min, max) = (r.range_min.value(), r.slant_range.value());
    assert!(min < max);
    let gain = r
        .margin_db_at_range_min
        .expect("margin at the nearest")
        .value()
        - r.margin_db.value();
    assert!((gain - 20.0 * (max / min).log10()).abs() < 1e-9, "{gain}");
    let rate = r
        .max_data_rate_at_range_min
        .expect("rate at the nearest")
        .value()
        / r.max_data_rate.value();
    assert!((rate / (max / min).powi(2) - 1.0).abs() < 1e-9, "{rate}");
    let lt = r.light_time_min.expect("light time at the nearest").value();
    assert!((lt - min / C_LIGHT).abs() < 1e-9);
    assert!((r.light_time.value() - max / C_LIGHT).abs() < 1e-9);
}

/// 外合（太陽の向こう側）から始まる 1.5 AU の円軌道: SEP の最小は 0（許容 1e-4 rad は地球の黄道緯度の上限に余裕を見た値）で、
/// しきい値 3° を下回る時間がある。判定は警告（key `solar_conjunction`）。
#[test]
fn a_probe_behind_the_sun_reports_zero_sep_and_a_warning() {
    let out = run_deep(PI, 100.0);
    let c = out.solar_conjunction.as_ref().expect("conjunction");
    assert!(c.sep_min.value() < 1e-4, "{}", c.sep_min.value());
    assert_eq!(c.threshold, DEFAULT_CONJUNCTION_ANGLE);
    assert!(c.time_below_threshold.value() > 0.0);
    let check = out
        .checks
        .iter()
        .find(|c| c.key == "solar_conjunction")
        .expect("check");
    assert_eq!(check.verdict, Verdict::Warning);
}

/// 地球と直角の向き（太陽での角が 90°）から 5 日間: SEP は atan(1.5 / r_E) ≈ 56° から大きく動かず、しきい値を下回らない（判定は Ok）。
#[test]
fn a_probe_at_quadrature_is_not_in_conjunction() {
    let out = run_deep(PI / 2.0, 5.0);
    let c = out.solar_conjunction.as_ref().expect("conjunction");
    let expected = (1.5 * AU / position_eci_m(Body::Sun, epoch()).norm()).atan();
    assert!(
        (c.sep_min.value() - expected).abs() < 0.1,
        "{}",
        c.sep_min.value()
    );
    assert_eq!(c.time_below_threshold.value(), 0.0);
    let check = out
        .checks
        .iter()
        .find(|c| c.key == "solar_conjunction")
        .expect("check");
    assert_eq!(check.verdict, Verdict::Ok);
}

/// 地球を回る衛星の出力には、新しいキーが出ない（JSON は従来と同じ）。
#[test]
fn an_earth_orbit_output_has_no_deep_space_keys() {
    let mut link = deep_link();
    link.central = CentralBody::Earth;
    link.altitude = Some(Qty::from_raw(1.0e6));
    link.elevation = Qty::from_raw(PI / 2.0);
    let out = LinkBudget::run(&LinkBudgetInput {
        links: vec![link],
        required_margin_db: None,
        orbit: None,
        duration: None,
    })
    .expect("run");
    let text = serde_json::to_string(&out).expect("json");
    for key in ["solar_conjunction", "light_time_min", "at_range_min"] {
        assert!(!text.contains(key), "{key}");
    }
    assert!(out.checks.iter().all(|c| c.key != "solar_conjunction"));
}
