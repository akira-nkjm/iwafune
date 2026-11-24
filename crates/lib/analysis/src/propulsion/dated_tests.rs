//! 日付つきの惑星間遷移（マヌーバの `transfer`）の解析。Curtis Example 8.8（1996-11-07 → 1997-09-12）の
//! 地球 → 火星を、解析式の暦（このクレートのテストは DE440 を登録しない）で、`sat_astro::transfer::dated_transfer` の
//! 値（出発の v∞ 3.162 km/s・到着 2.876 km/s）と、その v∞ からの手計算の噴射に突き合わせる。

use super::tests::{DRY, input, maneuver};
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
pub(super) fn mars_input(incl_deg: f64, departure: &str) -> PropulsionInput {
    let mut i = input(vec![ManeuverInput {
        target: OrbitTarget {
            body: Some(CentralBody::Mars),
            altitude: Some(Qty::from_raw(300e3)),
            ..OrbitTarget::default()
        },
        transfer: Some(DatedTransferInput {
            departure: epoch(departure),
            arrival: epoch("1997-09-12T00:00:00Z"),
            departure_by: DepartureProvider::Spacecraft,
            flybys: Vec::new(),
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

/// 出発の噴射を打ち上げ機が行う遷移の入力（`mars_input` の出発を打ち上げ機に）。
fn launcher_input(incl_deg: f64) -> PropulsionInput {
    let mut i = mars_input(incl_deg, "1996-11-07T00:00:00Z");
    i.maneuvers[0].transfer.as_mut().unwrap().departure_by = DepartureProvider::Launcher;
    i
}

/// 打ち上げ機が出発を行うと、噴射の列から出発の噴射だけが消える: 合計 ΔV は衛星が出発するときから出発の噴射を引いた値
/// （同じ解析の出力どうしの差）、推薬は捕獲の噴射だけのロケット方程式 `m_p = m_dry (e^(Δv/v_e) − 1)`（残る推薬 0・マージン 0 の公称）、
/// C3・超過速度・赤緯は変わらない（打ち上げの要求として出る）。
#[test]
fn launcher_departure_drops_the_departure_burn_only() {
    let own = Maneuvers::run(&mars_input(97.0, "1996-11-07T00:00:00Z")).unwrap();
    let by_launcher = Maneuvers::run(&launcher_input(97.0)).unwrap();
    let (a, b) = (&own.maneuvers[0], &by_launcher.maneuvers[0]);
    assert_eq!(a.burns.len(), 2);
    assert_eq!(b.burns.len(), 1);
    let departure_dv = a.burns[0].delta_v.value();
    close(b.burns[0].delta_v.value(), a.burns[1].delta_v.value(), 1e-9);
    close(
        by_launcher.total_delta_v.value(),
        own.total_delta_v.value() - departure_dv,
        1e-9,
    );
    close(
        b.ideal_delta_v.value(),
        a.ideal_delta_v.value() - departure_dv,
        1e-9,
    );
    // 推薬（乾燥 10 kg・220 s の捕獲だけ）
    let ve = sat_astro::rocket::exhaust_velocity(220.0);
    let hand = DRY * ((b.burns[0].delta_v.value() / ve).exp() - 1.0);
    close(by_launcher.total_propellant.value(), hand, 1e-9);
    close(by_launcher.initial_mass.value(), DRY + hand, 1e-9);
    assert!(by_launcher.total_propellant.value() < own.total_propellant.value());
    // C3 などは同じ結果で、出発を誰が行うかだけが違う
    let (da, db) = (
        a.dated_transfer.as_ref().unwrap(),
        b.dated_transfer.as_ref().unwrap(),
    );
    assert_eq!(da.departure_by, DepartureProvider::Spacecraft);
    assert_eq!(db.departure_by, DepartureProvider::Launcher);
    close(da.c3.value(), db.c3.value(), 0.0);
    close(
        da.departure_declination.value(),
        db.departure_declination.value(),
        0.0,
    );
    close(
        a.transfer_time.unwrap().value(),
        b.transfer_time.unwrap().value(),
        1e-6,
    );
    assert!(b.notes.contains(&ManeuverNote::DepartureByLauncher));
    assert!(!a.notes.contains(&ManeuverNote::DepartureByLauncher));
    // ΔV バジェットも同じ（合計から出発の噴射を引いた値）
    let budget = DvBudget::run(&launcher_input(97.0)).unwrap();
    let own_budget = DvBudget::run(&mars_input(97.0, "1996-11-07T00:00:00Z")).unwrap();
    close(
        budget.total.value(),
        own_budget.total.value() - departure_dv,
        1e-9,
    );
    assert_eq!(
        budget.dated_transfers[0].departure_by,
        DepartureProvider::Launcher
    );
}

/// 打ち上げ機が出発を行うなら、駐車軌道の傾斜角（赤緯の警告）も前のマヌーバが終わる時刻（日付の警告）も問わない。
#[test]
fn launcher_departure_has_no_parking_orbit_warnings() {
    let mut i = launcher_input(0.0);
    i.initial_orbit.epoch = epoch("1996-11-20T00:00:00Z");
    let m = &Maneuvers::run(&i).unwrap().maneuvers[0];
    assert!(m.checks.is_empty(), "{:?}", m.checks);
    // 衛星が出発するなら、同じ入力は両方の警告
    i.maneuvers[0].transfer.as_mut().unwrap().departure_by = DepartureProvider::Spacecraft;
    let keys: Vec<_> = Maneuvers::run(&i).unwrap().maneuvers[0]
        .checks
        .iter()
        .map(|c| c.key.clone())
        .collect();
    assert!(
        keys.contains(&"dated_transfer_plane".to_owned()),
        "{keys:?}"
    );
    assert!(
        keys.contains(&"dated_transfer_timing".to_owned()),
        "{keys:?}"
    );
}

/// 有限燃焼でも、出発の噴射が無いので飛行の時間は「慣性飛行 + 捕獲の噴射の半分」（衛星が出発するときは最初と最後の半分ずつ）。
#[test]
fn launcher_departure_transfer_time_with_a_finite_burn() {
    let mut i = launcher_input(97.0);
    i.thrusters[0].thrust = Qty::from_raw(500.0);
    i.maneuvers[0].method = TransferMethod::FiniteBurn;
    let out = Maneuvers::run(&i).unwrap();
    let m = &out.maneuvers[0];
    assert!(
        !m.notes.contains(&ManeuverNote::FiniteBurnTooLong),
        "{:?}",
        m.notes
    );
    let tof = m.dated_transfer.as_ref().unwrap().time_of_flight.value();
    close(
        m.transfer_time.unwrap().value(),
        tof + 0.5 * m.burns[0].burn_time.unwrap().value(),
        1e-6,
    );
}
