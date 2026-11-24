//! 巡航の区間・弧の連鎖（`cruise_legs`・`cruise_trajectory`）の確認。暦は解析式。
//! 地球 → 金星（通過）→ 火星の 2 脚: 境（金星の通過の日）で位置が連続（金星の位置を通る）、速度の跳びは 2 つの Lambert の速度の差。

use super::*;
use crate::ephem::CentralBody;
use crate::orbit::Trajectory;
use crate::time::Epoch;

fn epoch(s: &str) -> Epoch {
    Epoch::parse(s).expect("epoch")
}

const DEPARTURE: &str = "2028-10-01T00:00:00Z";
const FLYBY: &str = "2028-12-30T00:00:00Z";
const ARRIVAL: &str = "2030-02-23T00:00:00Z";

fn venus() -> FlybyStop {
    FlybyStop {
        body: CentralBody::Venus,
        date: epoch(FLYBY),
        min_altitude_m: None,
    }
}

fn chain() -> Trajectory {
    cruise_trajectory(
        CentralBody::Earth,
        CentralBody::Mars,
        epoch(DEPARTURE),
        epoch(ARRIVAL),
        &[venus()],
    )
    .expect("chain")
}

fn arcs(t: &Trajectory) -> &[Trajectory] {
    let Trajectory::Arcs(c) = t else {
        panic!("a chain of two arcs is expected");
    };
    c.arcs()
}

/// 境は金星の通過の日で、前の弧の終わりも後の弧の始めも、その日の金星の位置（暦）を通る。許容は 1 m（連鎖の検査の許容
/// `BREAK_POSITION_TOL_M` と同じ。Lambert の二分法の幅 `1e-13` は 30 km/s で 0.1 m 未満）。
#[test]
fn both_arcs_pass_through_the_flyby_body_at_the_flyby_date() {
    let t = chain();
    let (departure, flyby) = (epoch(DEPARTURE), epoch(FLYBY));
    let a = arcs(&t);
    assert_eq!(a.len(), 2);
    assert_eq!(t.central(), CentralBody::Sun);
    let at = t.breakpoints_s()[0];
    let want = flyby.seconds_since(t.epoch());
    assert!((at - want).abs() < 1e-6, "{at} vs {want}");
    let venus = CentralBody::Venus.state_about_parent(flyby).expect("venus");
    let incoming = a[0].state_at(a[0].span_s().expect("span")).expect("state");
    let outgoing = t.state_at(at).expect("state");
    assert!((incoming.r_eci - venus.r_eci).norm() < 1.0);
    assert!((outgoing.r_eci - venus.r_eci).norm() < 1.0);
    // 脚の始めの弧は出発の日の後（影響圏を出てから）に始まる
    assert!(t.epoch().seconds_since(departure) > 0.0);
}

/// 速度の跳びは、2 つの脚の Lambert の速度の差（出ていく脚の出発の速度 − 入る脚の到着の速度）。接触要素への往復の丸めの範囲
/// （相対 1e-12 × 4e4 m/s）で一致する。通過の天体の速度に対する超過速度の差は `flyby_geometry` が入り・出の超過速度として使う量。
#[test]
fn the_velocity_jump_is_the_difference_of_the_two_lambert_velocities() {
    let t = chain();
    let legs = lambert_chain(
        CentralBody::Earth,
        CentralBody::Mars,
        epoch(DEPARTURE),
        epoch(ARRIVAL),
        &[venus()],
    )
    .expect("legs");
    let a = arcs(&t);
    let v_in = a[0]
        .state_at(a[0].span_s().expect("span"))
        .expect("state")
        .v_eci;
    let v_out = a[1].state_at(0.0).expect("state").v_eci;
    let want = legs[1].v_departure_m_s - legs[0].v_arrival_m_s;
    let jump = v_out - v_in;
    assert!((jump - want).norm() < 1e-6, "{:?} vs {want:?}", jump);
    assert!(
        jump.norm() > 1.0e3,
        "a flyby changes the heliocentric velocity"
    );
    // 入る側・出る側の速度は、それぞれの脚の Lambert の速度そのもの
    assert!((v_in - legs[0].v_arrival_m_s).norm() < 1e-6);
    assert!((v_out - legs[1].v_departure_m_s).norm() < 1e-6);
}

/// 最初の弧の始めは出発の天体（地球）の影響圏の境、最後の弧の終わりは到着の天体（火星）の影響圏の境（許容 100 m。
/// 境の時刻を二分法で 1 ms まで詰めるため。既存の `cruise_leg` と同じ根拠）。通過の日の側は影響圏で切らない。
#[test]
fn only_the_outer_ends_are_clipped_by_the_spheres_of_influence() {
    let t = chain();
    let a = arcs(&t);
    let first = a[0].state_at(0.0).expect("state");
    let earth = CentralBody::Earth
        .state_about_parent(t.epoch())
        .expect("earth");
    let soi = |b: CentralBody| b.soi_radius_m().expect("soi");
    assert!(((first.r_eci - earth.r_eci).norm() - soi(CentralBody::Earth)).abs() < 100.0);
    let last = &a[1];
    let end = last.state_at(last.span_s().expect("span")).expect("state");
    let end_epoch = last.epoch().plus_seconds(last.span_s().expect("span"));
    let mars = CentralBody::Mars
        .state_about_parent(end_epoch)
        .expect("mars");
    assert!(((end.r_eci - mars.r_eci).norm() - soi(CentralBody::Mars)).abs() < 100.0);
    // 全体の長さは、元期から最後の弧の終わりまで
    let total = end_epoch.seconds_since(t.epoch());
    assert!((t.span_s().expect("span") - total).abs() < 1e-6);
    // 通過の日の側は切らない: 前の弧は通過の日まで、後の弧は通過の日から
    assert_eq!(a[1].epoch(), epoch(FLYBY));
}

/// 通過のない遷移は弧 1 本で、従来の `cruise_leg` の軌道に期間を付けたものとビットまで同じ。
#[test]
fn a_transfer_without_flybys_is_the_single_leg_as_before() {
    let (from, to) = (CentralBody::Earth, CentralBody::Mars);
    let (d, a) = (epoch("2028-12-10T00:00:00Z"), epoch("2029-09-25T00:00:00Z"));
    let leg = cruise_leg(from, to, d, a).expect("leg");
    let old = Trajectory::Elements(leg.orbit.clone()).with_span(leg.span_s);
    let new = cruise_trajectory(from, to, d, a, &[]).expect("trajectory");
    assert!(matches!(new, Trajectory::Spanned { .. }));
    assert_eq!(
        serde_json::to_string(&new).unwrap(),
        serde_json::to_string(&old).unwrap()
    );
    let times = [0.0, 1.0e5, 1.0e7, leg.span_s];
    assert_eq!(
        new.states_at(&times).unwrap(),
        old.states_at(&times).unwrap()
    );
}

/// 通過で隣り合う天体が同じなら断る（同じ天体の脚は周回数 0 の弧にならない）。通過の日が出発の前なら断る。
#[test]
fn invalid_flyby_sequences_are_refused() {
    let same = FlybyStop {
        body: CentralBody::Earth,
        date: epoch(FLYBY),
        min_altitude_m: None,
    };
    let r = cruise_trajectory(
        CentralBody::Earth,
        CentralBody::Mars,
        epoch(DEPARTURE),
        epoch(ARRIVAL),
        &[same],
    );
    assert!(r.unwrap_err().to_string().contains("must differ"));
    let early = FlybyStop {
        date: epoch("2028-09-01T00:00:00Z"),
        ..venus()
    };
    assert!(
        cruise_trajectory(
            CentralBody::Earth,
            CentralBody::Mars,
            epoch(DEPARTURE),
            epoch(ARRIVAL),
            &[early]
        )
        .is_err()
    );
}
