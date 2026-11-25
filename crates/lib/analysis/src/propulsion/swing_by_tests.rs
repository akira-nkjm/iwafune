//! 途中のスイングバイ（`transfer.flybys`）のある日付つきの遷移の解析: 地球 → 金星の通過 → 火星。
//! 数値の手計算の突き合わせは `sat_astro::transfer::flyby_tests`、ここは噴射の列・合計・結果・判定・探索の拒否のつなぎ。
//! 暦は解析式（このクレートのテストは DE440 を登録しない）。

use super::dated_tests::mars_input;
use super::*;
use crate::spec::{Analysis, Verdict};
use sat_astro::ephem::CentralBody;
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

pub(super) fn swing_input(flybys: Vec<FlybyInput>) -> PropulsionInput {
    let mut i = mars_input(97.0, "2028-10-01T00:00:00Z");
    let t = i.maneuvers[0].transfer.as_mut().unwrap();
    t.departure = epoch("2028-10-01T00:00:00Z");
    t.arrival = epoch("2029-12-01T00:00:00Z");
    t.flybys = flybys;
    i.initial_orbit.epoch = epoch("2028-09-25T00:00:00Z");
    i
}

pub(super) fn venus(date: &str) -> FlybyInput {
    FlybyInput {
        body: CentralBody::Venus,
        date: epoch(date),
        min_altitude: None,
        powered: true,
    }
}

/// 通過が実現できる日付（2028-10-01 出発・2029-01-10 金星・2030-03-01 到着。回頭 5.8° は 300 km の下限での最大 11.9° 以内）。
fn feasible_input() -> PropulsionInput {
    let mut i = swing_input(vec![venus("2029-01-10T00:00:00Z")]);
    i.maneuvers[0].transfer.as_mut().unwrap().arrival = epoch("2030-03-01T00:00:00Z");
    i
}

/// 噴射の列は 出発 → 金星の近点 → 火星の捕獲。合計は 3 つの和で、金星の噴射は通過の結果の近点の噴射と同じ。
/// ΔV バジェットにも結果が載り、全体の飛行時間は出発から到着まで。
#[test]
fn swing_by_burn_joins_the_departure_and_capture() {
    let out = Maneuvers::run(&feasible_input()).unwrap();
    let m = &out.maneuvers[0];
    let d = m.dated_transfer.as_ref().unwrap();
    assert_eq!(d.flybys.len(), 1);
    let f = &d.flybys[0];
    assert!(f.feasible && f.powered && f.turn_angle.value() < f.max_turn_angle.value());
    assert_eq!(f.body, CentralBody::Venus);
    assert_eq!(m.burns.len(), 3);
    assert_eq!(m.burns[1].central, Some(CentralBody::Venus));
    close(m.burns[1].delta_v.value(), f.delta_v.value(), 1e-9);
    close(m.burns[1].v_inf.unwrap().value(), f.v_inf_in.value(), 1e-9);
    close(
        m.delta_v.value(),
        m.burns.iter().map(|b| b.delta_v.value()).sum::<f64>(),
        1e-9,
    );
    // 近点の高度は半径 − 天体の半径、下限以上
    let alt = f.periapsis_altitude.unwrap().value();
    close(
        alt,
        f.periapsis_radius.unwrap().value() - CentralBody::Venus.radius_m(),
        1e-6,
    );
    assert!(alt >= f.min_altitude.value() - 1e-6);
    close(f.min_altitude.value(), 300e3, 0.0);
    assert!(m.notes.contains(&ManeuverNote::SwingBy));
    close(
        m.transfer_time.unwrap().value(),
        epoch("2030-03-01T00:00:00Z").seconds_since(epoch("2028-10-01T00:00:00Z")),
        1e-6,
    );
    assert!(
        m.checks.iter().all(|c| c.key != "swing_by_turn"),
        "{:?}",
        m.checks
    );
    let budget = DvBudget::run(&feasible_input()).unwrap();
    close(budget.total.value(), out.total_delta_v.value(), 1e-9);
    assert_eq!(budget.dated_transfers[0].flybys.len(), 1);
}

/// 動力なしと宣言した通過（`powered = false`）は噴射に数えず、差があれば警告（差は結果に残る）。
#[test]
fn an_unpowered_flyby_is_not_counted_and_warns_when_the_speeds_differ() {
    let own = Maneuvers::run(&feasible_input()).unwrap();
    let mut i = feasible_input();
    i.maneuvers[0].transfer.as_mut().unwrap().flybys[0].powered = false;
    let out = Maneuvers::run(&i).unwrap();
    let m = &out.maneuvers[0];
    assert_eq!(m.burns.len(), 2);
    let f = &m.dated_transfer.as_ref().unwrap().flybys[0];
    assert!(!f.powered && f.delta_v.value() > 1.0);
    close(
        out.total_delta_v.value(),
        own.total_delta_v.value() - f.delta_v.value(),
        1e-9,
    );
    let c = m
        .checks
        .iter()
        .find(|c| c.key == "swing_by_unpowered")
        .expect("warning");
    assert_eq!(c.verdict, Verdict::Warning);
}

/// 下限の高度でも曲がりきれない日付は警告。足りない回頭の分の ΔV は噴射に含める（近点の噴射 + 足りない分）。
#[test]
fn an_infeasible_turn_warns_and_charges_the_shortfall() {
    let i = swing_input(vec![venus("2029-02-01T00:00:00Z")]);
    let out = Maneuvers::run(&i).unwrap();
    let m = &out.maneuvers[0];
    let f = &m.dated_transfer.as_ref().unwrap().flybys[0];
    assert!(!f.feasible && f.shortfall_delta_v.value() > 0.0);
    // 必要な近点は下限より低い（この日付は金星の表面より下を通ることになる）
    assert!(f.periapsis_altitude.unwrap().value() < f.min_altitude.value());
    close(
        m.burns[1].delta_v.value(),
        f.delta_v.value() + f.shortfall_delta_v.value(),
        1e-9,
    );
    let c = m
        .checks
        .iter()
        .find(|c| c.key == "swing_by_turn")
        .expect("warning");
    assert_eq!(c.verdict, Verdict::Warning);
}

/// 有限燃焼の遷移の時間は「慣性飛行 + 出発と捕獲の噴射の半分ずつ」。途中の通過の噴射は数えない。
#[test]
fn finite_burn_time_ignores_the_flyby_burn() {
    let mut i = feasible_input();
    i.thrusters[0].thrust = Qty::from_raw(1.0e4);
    i.thrusters[0].isp = Qty::from_raw(8000.0);
    i.maneuvers[0].method = TransferMethod::FiniteBurn;
    let out = Maneuvers::run(&i).unwrap();
    let m = &out.maneuvers[0];
    assert!(
        !m.notes.contains(&ManeuverNote::FiniteBurnTooLong),
        "{:?}",
        m.notes
    );
    assert_eq!(m.burns.len(), 3);
    let tof = m.dated_transfer.as_ref().unwrap().time_of_flight.value();
    let (first, last) = (
        m.burns[0].burn_time.unwrap().value(),
        m.burns[2].burn_time.unwrap().value(),
    );
    close(
        m.transfer_time.unwrap().value(),
        tof + 0.5 * (first + last),
        1e-6,
    );
}

/// 打ち上げ機が出発するときは列の最初が通過の噴射になるが、それは飛行の途中なので、時間は「慣性飛行 + 捕獲の噴射の半分」。
#[test]
fn finite_burn_time_with_a_launcher_departure_ignores_the_flyby_burn() {
    let mut i = feasible_input();
    i.thrusters[0].thrust = Qty::from_raw(1.0e4);
    i.thrusters[0].isp = Qty::from_raw(8000.0);
    i.maneuvers[0].method = TransferMethod::FiniteBurn;
    i.maneuvers[0].transfer.as_mut().unwrap().departure_by = DepartureProvider::Launcher;
    let m = &Maneuvers::run(&i).unwrap().maneuvers[0];
    assert_eq!(m.burns.len(), 2);
    let tof = m.dated_transfer.as_ref().unwrap().time_of_flight.value();
    close(
        m.transfer_time.unwrap().value(),
        tof + 0.5 * m.burns[1].burn_time.unwrap().value(),
        1e-6,
    );
}

/// 多体の確認は脚ごと（通過の前後はつなげない）: 脚の番号・天体・日付が脚のもので、注記が付く。
#[test]
fn transfer_check_runs_leg_by_leg() {
    let out = TransferCheck::run(&feasible_input()).unwrap();
    assert_eq!(out.transfers.len(), 2);
    let (a, b) = (&out.transfers[0], &out.transfers[1]);
    assert_eq!((a.leg, b.leg), (Some(1), Some(2)));
    assert_eq!((a.from, a.to), (CentralBody::Earth, CentralBody::Venus));
    assert_eq!((b.from, b.to), (CentralBody::Venus, CentralBody::Mars));
    assert_eq!(a.arrival, epoch("2029-01-10T00:00:00Z"));
    assert_eq!(b.departure, a.arrival);
    assert!(a.notes.contains(&TransferCheckNote::LegsNotChained));
    // 通過のない遷移は脚の番号を持たない
    let direct = TransferCheck::run(&mars_input(97.0, "1996-11-07T00:00:00Z")).unwrap();
    assert_eq!(direct.transfers[0].leg, None);
}
