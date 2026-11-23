//! 日付つきの惑星間遷移（マヌーバの `transfer`）の解析。Curtis Example 8.8（1996-11-07 → 1997-09-12）の
//! 地球 → 火星を、解析式の暦（このクレートのテストは DE440 を登録しない）で、`sat_astro::transfer::dated_transfer` の
//! 値（出発の v∞ 3.162 km/s・到着 2.876 km/s）と、その v∞ からの手計算の噴射に突き合わせる。

use super::tests::{input, maneuver};
use super::*;
use crate::spec::{Analysis, Verdict};
use sat_astro::constants::{GM_MARS, MU_EARTH, R_EARTH, R_MARS};
use sat_astro::ephem::{CentralBody, EphemerisSource};
use sat_astro::time::Epoch;
use sat_units::typed::Qty;

fn close(a: f64, b: f64, tol: f64) {
    assert!(
        (a - b).abs() <= tol,
        "difference between {a} and {b} exceeds {tol}"
    );
}

fn epoch(s: &str) -> Epoch {
    Epoch::parse(s).unwrap()
}

/// 高度 300 km の駐車軌道（傾斜角 `incl_deg`）、元期は出発の 6 日前。火星の高度 300 km の円軌道へ捕獲する日付つきの遷移。
fn mars_input(incl_deg: f64, departure: &str) -> PropulsionInput {
    let mut i = input(vec![ManeuverInput {
        target: OrbitTarget {
            body: Some(CentralBody::Mars),
            altitude: Some(Qty::from_raw(300e3)),
            ..OrbitTarget::default()
        },
        transfer: Some(DatedTransferInput {
            departure: epoch(departure),
            arrival: epoch("1997-09-12T00:00:00Z"),
        }),
        ..maneuver(
            "MARS",
            DvCategory::Insertion,
            TransferMethod::Impulsive,
            0.0,
        )
    }]);
    i.initial_orbit = sat_astro::orbit::OrbitSpec::circular(
        epoch("1996-11-01T00:00:00Z"),
        Qty::from_raw(300e3),
        Qty::from_raw(incl_deg.to_radians()),
        Qty::ZERO,
    );
    i
}

/// ΔV・v∞・C3・飛行時間は `dated_transfer` の値（Curtis の v∞ は 3.1651・2.8852 km/s、解析式の暦で 3.1621・2.8756 km/s）、
/// 噴射は v∞ からの手計算 `√(v∞² + 2μ/r) − √(μ/r)`（駐車軌道・捕獲軌道とも円）。
#[test]
fn dated_transfer_follows_stage_one_and_hand_calculation() {
    let out = Maneuvers::run(&mars_input(97.0, "1996-11-07T00:00:00Z")).unwrap();
    let m = &out.maneuvers[0];
    let d = m.dated_transfer.as_ref().expect("dated transfer result");
    assert_eq!(d.ephemeris, EphemerisSource::Analytic);
    close(d.v_inf_departure.value() / 1e3, 3.1621, 0.002);
    close(d.v_inf_arrival.value() / 1e3, 2.8756, 0.002);
    close(d.time_of_flight.value() / 86_400.0, 309.0, 1e-9);
    close(d.c3.value(), d.v_inf_departure.value().powi(2), 1e-6);
    // 手計算の噴射（v∞ は上で突き合わせた値）
    let burn =
        |mu: f64, r: f64, v_inf: f64| (v_inf * v_inf + 2.0 * mu / r).sqrt() - (mu / r).sqrt();
    let dep = burn(MU_EARTH, R_EARTH + 300e3, d.v_inf_departure.value());
    let arr = burn(GM_MARS, R_MARS + 300e3, d.v_inf_arrival.value());
    close(m.burns[0].delta_v.value(), dep, 1e-6);
    close(m.burns[1].delta_v.value(), arr, 1e-6);
    close(m.delta_v.value(), dep + arr, 1e-6);
    close(
        m.burns[0].v_inf.unwrap().value(),
        d.v_inf_departure.value(),
        1e-9,
    );
    assert_eq!(m.burns[1].central, Some(CentralBody::Mars));
    assert!(m.notes.contains(&ManeuverNote::DatedLambert));
    close(
        m.transfer_time.unwrap().value(),
        d.time_of_flight.value(),
        1e-6,
    );
    assert!(m.checks.is_empty(), "{:?}", m.checks);
    // ΔV バジェットにも同じ値と結果が載る
    let budget = DvBudget::run(&mars_input(97.0, "1996-11-07T00:00:00Z")).unwrap();
    close(budget.total.value(), dep + arr, 1e-6);
    assert_eq!(budget.dated_transfers.len(), 1);
    assert_eq!(budget.dated_transfers[0].maneuver, "MARS");
}

/// 日付が無ければ従来どおりのホーマンのパッチドコニック（結果に日付の欄が無く、注記も `PatchedConic`）。
#[test]
fn without_dates_the_hohmann_estimate_is_unchanged() {
    let mut i = mars_input(97.0, "1996-11-07T00:00:00Z");
    i.maneuvers[0].transfer = None;
    let m = &Maneuvers::run(&i).unwrap().maneuvers[0];
    assert!(m.dated_transfer.is_none());
    assert!(m.notes.contains(&ManeuverNote::PatchedConic));
    assert!(!m.notes.contains(&ManeuverNote::DatedLambert));
}

/// 出発の漸近線の赤緯が駐車軌道の傾斜角より大きければ警告（面の変更は数えない）。傾斜角 0° の駐車軌道は常に超える。
#[test]
fn declination_above_the_inclination_warns() {
    let m = &Maneuvers::run(&mars_input(0.0, "1996-11-07T00:00:00Z"))
        .unwrap()
        .maneuvers[0];
    let c = m
        .checks
        .iter()
        .find(|c| c.key == "dated_transfer_plane")
        .expect("plane check");
    assert_eq!(c.verdict, Verdict::Warning);
    let budget = DvBudget::run(&mars_input(0.0, "1996-11-07T00:00:00Z")).unwrap();
    assert!(
        budget
            .checks
            .iter()
            .any(|c| c.key == "dated_transfer_plane")
    );
}

/// 前のマヌーバの終わりが出発の日より後なら警告。最初のマヌーバでも、元期が出発の日より後なら同じ。
#[test]
fn departure_before_the_orbit_is_ready_warns() {
    let mut i = mars_input(97.0, "1996-11-07T00:00:00Z");
    i.initial_orbit.epoch = epoch("1996-11-20T00:00:00Z");
    let m = &Maneuvers::run(&i).unwrap().maneuvers[0];
    assert!(m.checks.iter().any(|c| c.key == "dated_transfer_timing"));
    // 前に低推力のマヌーバがあると、その終わりから数える
    let mut j = mars_input(97.0, "1996-11-07T00:00:00Z");
    let raise = ManeuverInput {
        thruster: Some("EP".into()),
        ..maneuver(
            "RAISE",
            DvCategory::Insertion,
            TransferMethod::LowThrust,
            450.0,
        )
    };
    j.maneuvers.insert(0, raise);
    let out = Maneuvers::run(&j).unwrap();
    let late = out.maneuvers[1]
        .checks
        .iter()
        .any(|c| c.key == "dated_transfer_timing");
    // 低推力の 400 → 450 km は数日（元期 1996-11-01 から出発の 1996-11-07 までの 6 日を超える）
    let days = out.maneuvers[0].transfer_time.unwrap().value() / 86_400.0;
    assert_eq!(late, days > 6.0, "{days} d");
}

/// 暦の外の日付・順序が逆・兄弟でない天体は解析の誤り（検査が先に誤りにするが、解析も黙って計算しない）。
#[test]
fn bad_transfers_are_errors() {
    let mut i = mars_input(97.0, "1997-10-01T00:00:00Z");
    assert!(Maneuvers::run(&i).is_err(), "arrival before departure");
    i = mars_input(97.0, "1996-11-07T00:00:00Z");
    i.maneuvers[0].target.body = Some(CentralBody::Moon);
    assert!(Maneuvers::run(&i).is_err(), "Earth to Moon");
    i = mars_input(97.0, "1996-11-07T00:00:00Z");
    i.maneuvers[0].method = TransferMethod::LowThrust;
    assert!(Maneuvers::run(&i).is_err(), "low thrust between bodies");
    i = mars_input(97.0, "1996-11-07T00:00:00Z");
    i.maneuvers[0].target.body = None;
    assert!(Maneuvers::run(&i).is_err(), "dates without a target body");
}
