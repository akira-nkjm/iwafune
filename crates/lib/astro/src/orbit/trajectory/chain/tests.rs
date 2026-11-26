//! 弧の連鎖の確認。一続きの軌道を途中で割った連鎖は、割らない軌道と同じ状態を返す（弧ごとの評価・表・付け替えが連続の場合に一致する）。
//! 速度の跳ぶ連鎖は、折れ目の前後で跳びが残る（表が折れ目をまたいで補間しない）。

use super::*;
use crate::force::{ForceConfig, GravityModel};
use crate::orbit::{
    Cycle, KeplerElements, OrbitElements, OrbitSpec, Propagator, osculating_elements,
};
use sat_units::typed::Qty;

fn epoch() -> Epoch {
    Epoch::parse("2027-04-01T00:00:00Z").expect("epoch")
}

/// 地球を回る、離心率 0.01 の低軌道。
fn orbit(propagator: Propagator) -> OrbitSpec {
    let mut o = OrbitSpec::circular_about(
        CentralBody::Earth,
        epoch(),
        Qty::from_raw(550e3),
        Qty::from_raw(1.1),
        Qty::from_raw(0.4),
    );
    o.elements.eccentricity = 0.01;
    o.elements.mean_anomaly = Qty::from_raw(0.7);
    o.propagator = propagator;
    o
}

const TOTAL_S: f64 = 20_000.0;
/// 割る時刻（周期やサンプルの刻みの倍数でない値）
const SPLIT_S: f64 = 7_345.6;

/// `o` を `split_s` で割った 2 本の弧の連鎖（後の弧は同じ軌道を `split_s` に付け替えたもの）と、割らない 1 本の弧。
fn split(o: &OrbitSpec, split_s: f64) -> (Trajectory, Trajectory) {
    let whole = Trajectory::Elements(o.clone()).with_span(TOTAL_S);
    let later = o.rebased_at(epoch().plus_seconds(split_s)).expect("rebase");
    let chain = Trajectory::chain(vec![
        Trajectory::Elements(o.clone()).with_span(split_s),
        Trajectory::Elements(later).with_span(TOTAL_S - split_s),
    ])
    .expect("chain");
    (chain, whole)
}

fn assert_same(a: &OrbitState, b: &OrbitState, tol_m: f64, tol_m_s: f64, what: &str) {
    let (dr, dv) = ((a.r_eci - b.r_eci).norm(), (a.v_eci - b.v_eci).norm());
    assert!(dr <= tol_m && dv <= tol_m_s, "{what}: {dr} m, {dv} m/s");
}

/// 時間 [s] の一致。`Epoch` の差（`seconds_since`）は 1e-7 s 程度の丸めを含むので、許容は 1e-6 s。
fn assert_close_s(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-6, "{a} vs {b}");
}

fn times() -> Vec<f64> {
    vec![
        -50.0,
        0.0,
        1.0,
        1_000.0,
        SPLIT_S - 1e-6,
        SPLIT_S,
        SPLIT_S + 1e-6,
        9_000.0,
        TOTAL_S - 1.0,
        TOTAL_S,
        TOTAL_S + 100.0,
    ]
}

/// 解析的な伝播（Kepler・J2）: 割った連鎖は割らない軌道と同じ状態。許容は付け替えのテスト（`rebase.rs`）と同じ 1e-6 m
/// （解析式なので桁落ちの範囲）、速度は軌道速度 7.6 km/s の同じ相対（1e-6 / 7e6 倍）で 1e-9 m/s。
#[test]
fn a_split_analytic_orbit_equals_the_unsplit_orbit() {
    for p in [Propagator::Kepler, Propagator::J2] {
        let (chain, whole) = split(&orbit(p.clone()), SPLIT_S);
        assert!(matches!(chain, Trajectory::Arcs(_)));
        assert_eq!(chain.epoch(), whole.epoch());
        assert_eq!(chain.central(), whole.central());
        assert_close_s(chain.span_s().unwrap(), whole.span_s().unwrap());
        assert_eq!(chain.breakpoints_s().len(), 1);
        assert_close_s(chain.breakpoints_s()[0], SPLIT_S);
        let (a, b) = (
            chain.states_at(&times()).unwrap(),
            whole.states_at(&times()).unwrap(),
        );
        for ((x, y), t) in a.iter().zip(&b).zip(times()) {
            assert_same(x, y, 1e-6, 1e-9, &format!("{p:?} states_at {t}"));
            assert_same(
                &chain.state_at(t).unwrap(),
                y,
                1e-6,
                1e-9,
                &format!("{p:?} state_at {t}"),
            );
        }
        // 表（解析的な伝播は表を使わず、弧の解析式）
        let (ta, tb) = (
            chain.track(TOTAL_S, 30.0).unwrap(),
            whole.track(TOTAL_S, 30.0).unwrap(),
        );
        for t in times() {
            assert_same(
                &ta.state_at(t),
                &tb.state_at(t),
                1e-6,
                1e-9,
                &format!("{p:?} track {t}"),
            );
        }
    }
}

/// 付け替え: 始めがどの弧の中でも、残りの軌道は割らない軌道を同じ時刻に付け替えたものと同じ状態・期間・元期。
/// 境ちょうどは後の弧（残りは 1 本の弧になり、連鎖でなくなる）、最初の弧の中は残りの弧と後の弧の連鎖。
#[test]
fn starting_at_inside_each_arc_equals_starting_the_unsplit_orbit() {
    let (chain, whole) = split(&orbit(Propagator::Kepler), SPLIT_S);
    for (start_s, arcs_left) in [(0.0, 2), (2_000.0, 2), (SPLIT_S, 1), (12_000.0, 1)] {
        let start = epoch().plus_seconds(start_s);
        let (a, b) = (
            chain.starting_at(start).unwrap(),
            whole.starting_at(start).unwrap(),
        );
        assert_eq!(a.epoch(), start);
        assert_close_s(a.span_s().unwrap(), b.span_s().unwrap());
        assert_eq!(a.breakpoints_s().len(), arcs_left - 1, "start {start_s}");
        let want_break = (SPLIT_S - start_s).max(0.0);
        if arcs_left == 2 {
            assert_close_s(a.breakpoints_s()[0], want_break);
        }
        for t in [0.0, 500.0, 3_000.0, TOTAL_S - start_s - 1.0] {
            assert_same(
                &a.state_at(t).unwrap(),
                &b.state_at(t).unwrap(),
                1e-6,
                1e-9,
                &format!("start {start_s} t {t}"),
            );
        }
    }
    // 全体の範囲の外は付け替えられない（弧 1 本の期間と同じ）
    assert!(chain.starting_at(epoch().plus_seconds(-1.0)).is_err());
    assert!(
        chain
            .starting_at(epoch().plus_seconds(TOTAL_S + 1.0))
            .is_err()
    );
}

/// 境ちょうどの時刻は後の弧（出ていく弧）で評価する。速度の跳びのある連鎖で、境の前（直前）は前の弧、境ちょうどは後の弧の速度を返す。
#[test]
fn a_time_exactly_at_a_break_belongs_to_the_outgoing_arc() {
    let chain = kicked_chain(Propagator::Kepler, 100.0);
    let at = chain.breakpoints_s()[0];
    let Trajectory::Arcs(c) = &chain else {
        panic!("a chain is expected");
    };
    let (first, second) = (&c.arcs()[0], &c.arcs()[1]);
    let end_first = first.state_at(first.span_s().unwrap()).unwrap();
    let start_second = second.state_at(0.0).unwrap();
    assert_eq!(chain.state_at(at).unwrap(), start_second);
    assert_eq!(chain.states_at(&[at]).unwrap()[0], start_second);
    let before = chain.state_at(at - 1e-6).unwrap();
    assert!((before.v_eci - end_first.v_eci).norm() < 1e-3);
    // 位置は連続、速度は与えた跳び（100 m/s）だけ違う
    let miss = (end_first.r_eci - start_second.r_eci).norm();
    assert!(miss < 1e-6, "{miss}");
    let jump = (start_second.v_eci - end_first.v_eci).norm();
    assert!((jump - 100.0).abs() < 1e-6, "{jump}");
}

/// 弧 A を `at_s` まで進めた状態に、速度の向きへ `dv` [m/s] の跳びを足した状態から始まる弧 B をつなげた連鎖（数値伝播のときは弧ごとに積分）。
fn kicked_chain(p: Propagator, dv: f64) -> Trajectory {
    let a = orbit(p.clone());
    let end = a.state_at(SPLIT_S).unwrap();
    let kicked = OrbitState {
        r_eci: end.r_eci,
        v_eci: end.v_eci + end.v_eci.normalized().expect("speed") * dv,
    };
    let el: KeplerElements = osculating_elements(&kicked, CentralBody::Earth.gm()).unwrap();
    let mut b = OrbitSpec::new(epoch().plus_seconds(SPLIT_S), elements_from(&el), p);
    b.central = CentralBody::Earth;
    Trajectory::chain(vec![
        Trajectory::Elements(a).with_span(SPLIT_S),
        Trajectory::Elements(b).with_span(TOTAL_S - SPLIT_S),
    ])
    .expect("chain")
}

/// 数値伝播の弧の表は弧ごとで、折れ目をまたいで補間しない: 折れ目の前後で速度の跳び（100 m/s）が残り、位置は連続。
/// 表の許容は `track.rs` の補間のテストと同じ（位置 1 m・速度 1 cm/s）で、直接の積分（`states_at`）と比べる。
#[test]
fn numerical_tables_keep_the_velocity_jump_at_a_break() {
    let numerical = Propagator::Numerical {
        forces: ForceConfig::gravity_only(GravityModel::J2),
    };
    let chain = kicked_chain(numerical, 100.0);
    let at = chain.breakpoints_s()[0];
    let track = chain.track(TOTAL_S, 30.0).unwrap();
    let ts = [
        0.0,
        45.3,
        at - 20.0,
        at - 1e-6,
        at,
        at + 1e-6,
        at + 20.0,
        15_000.5,
        TOTAL_S,
    ];
    let direct = chain.states_at(&ts).unwrap();
    for (t, d) in ts.iter().zip(&direct) {
        assert_same(&track.state_at(*t), d, 1.0, 1e-2, &format!("t {t}"));
    }
    let (before, after) = (track.state_at(at - 1e-6), track.state_at(at));
    // 各側の表の許容（速度 1e-2 m/s）の 2 つ分
    let jump = (after.v_eci - before.v_eci).norm();
    assert!((jump - 100.0).abs() < 2.0e-2, "{jump}");
    assert!((after.r_eci - before.r_eci).norm() < 1.0);
}

/// 弧が 1 本ならその弧そのもの（連鎖の変種を作らない）。弧が無い・期間が無い・入れ子は断る。
#[test]
fn one_arc_is_the_arc_itself_and_invalid_chains_are_refused() {
    let one = Trajectory::Elements(orbit(Propagator::Kepler)).with_span(TOTAL_S);
    let same = Trajectory::chain(vec![one.clone()]).unwrap();
    assert!(matches!(same, Trajectory::Spanned { .. }));
    assert_eq!(
        serde_json::to_value(&same).unwrap(),
        serde_json::to_value(&one).unwrap()
    );
    assert!(Trajectory::chain(Vec::new()).is_err());
    let periodic = Trajectory::Elements(orbit(Propagator::Kepler));
    assert!(Trajectory::chain(vec![periodic.clone()]).is_err());
    assert!(Trajectory::chain(vec![one.clone(), periodic]).is_err());
    let (chain, _) = split(&orbit(Propagator::Kepler), SPLIT_S);
    let nested = Trajectory::chain(vec![chain, one]).unwrap_err().to_string();
    assert!(nested.contains("do not nest"), "{nested}");
}

/// 隣り合う弧がつながっていない連鎖は作らない: 時間の隙間（許容 1 ms を超える）・位置の食い違い（許容 1 m を超える）・中心天体の違い。
#[test]
fn arcs_that_do_not_meet_are_refused() {
    let o = orbit(Propagator::Kepler);
    let arc = |o: &OrbitSpec, from_s: f64, span: f64| {
        let r = o.rebased_at(epoch().plus_seconds(from_s)).unwrap();
        Trajectory::Elements(r).with_span(span)
    };
    let ok = Trajectory::chain(vec![arc(&o, 0.0, 1000.0), arc(&o, 1000.0, 500.0)]);
    assert!(ok.is_ok());
    // 時間の隙間 2 ms（許容 1 ms）
    let late = Trajectory::chain(vec![arc(&o, 0.0, 1000.0), arc(&o, 1000.002, 500.0)]);
    assert!(late.unwrap_err().to_string().contains("must meet"));
    // 位置の食い違い（同じ時刻の別の軌道）。1 m を超える
    let mut other = o.clone();
    other.elements.mean_anomaly = Qty::from_raw(0.7 + 1e-6);
    let jump = Trajectory::chain(vec![arc(&o, 0.0, 1000.0), arc(&other, 1000.0, 500.0)]);
    let e = jump.unwrap_err().to_string();
    assert!(
        e.contains("differs by") && e.contains("tolerance 1 m"),
        "{e}"
    );
    // 中心天体の違い
    let mut moon = o.clone();
    moon.central = CentralBody::Moon;
    let e = Trajectory::chain(vec![arc(&o, 0.0, 1000.0), arc(&moon, 1000.0, 500.0)]).unwrap_err();
    assert!(e.to_string().contains("Moon"), "{e}");
}

/// 弧の連鎖の入口: 繰り返しは全体で 1 回限り、要素は無く、高度も無い。`with_span` は全体の期間を置き換える（外の弧は落とす）。
#[test]
fn a_chain_is_one_off_over_all_arcs_without_elements() {
    let (chain, _) = split(&orbit(Propagator::Kepler), SPLIT_S);
    let Cycle::OneOff { span_s } = chain.cycle() else {
        panic!("a chain is one-off");
    };
    assert_close_s(span_s, TOTAL_S);
    assert!(chain.elements().is_none() && chain.altitude_m().is_none());
    // 弧ごとの刻み（弧の期間の 1/100）の最小
    assert_close_s(chain.table_step_s(), SPLIT_S.min(TOTAL_S - SPLIT_S) / 100.0);
    let shorter = chain.clone().with_span(SPLIT_S + 100.0);
    assert_eq!(shorter.span_s(), Some(SPLIT_S + 100.0));
    assert_eq!(shorter.breakpoints_s().len(), 1);
    let first_only = chain.clone().with_span(SPLIT_S - 1.0);
    assert!(matches!(first_only, Trajectory::Spanned { .. }));
    assert_eq!(first_only.span_s(), Some(SPLIT_S - 1.0));
    let longer = chain.with_span(TOTAL_S + 500.0);
    assert_eq!(longer.span_s(), Some(TOTAL_S + 500.0));
}

/// JSON: 2 本以上の弧は `source = "arcs"` と弧の並び。書いたものを読み戻すと同じ。弧が 1 本の `arcs` は従来の 1 本の弧の形で書き出す。
/// 期間の無い弧・つながらない弧は読むときに理由を返す。
#[test]
fn a_chain_round_trips_and_a_one_arc_chain_writes_the_old_shape() {
    let (chain, _) = split(&orbit(Propagator::J2), SPLIT_S);
    let v = serde_json::to_value(&chain).unwrap();
    assert_eq!(v["source"], "arcs");
    assert_eq!(v["arcs"].as_array().unwrap().len(), 2);
    assert_eq!(v["arcs"][0]["span_s"], SPLIT_S);
    let back: Trajectory = serde_json::from_value(v.clone()).unwrap();
    assert_eq!(serde_json::to_value(&back).unwrap(), v);
    assert_eq!(back.breakpoints_s(), chain.breakpoints_s());
    let one = v["arcs"][0].clone();
    let single: Trajectory =
        serde_json::from_value(serde_json::json!({"source": "arcs", "arcs": [one.clone()]}))
            .unwrap();
    assert_eq!(serde_json::to_value(&single).unwrap(), one);
    let mut no_span = v.clone();
    no_span["arcs"][1].as_object_mut().unwrap().remove("span_s");
    let e = serde_json::from_value::<Trajectory>(no_span)
        .unwrap_err()
        .to_string();
    assert!(e.contains("needs span_s"), "{e}");
    let mut gap = v;
    gap["arcs"][1]["epoch"] = "2027-04-01T05:00:00Z".into();
    let e = serde_json::from_value::<Trajectory>(gap)
        .unwrap_err()
        .to_string();
    assert!(e.contains("must meet"), "{e}");
}

/// 古典軌道要素（接触要素）を軌道の欄にする。
fn elements_from(el: &KeplerElements) -> OrbitElements {
    OrbitElements {
        semi_major_axis: Qty::from_raw(el.a_m),
        eccentricity: el.e,
        inclination: Qty::from_raw(el.i_rad),
        raan: Qty::from_raw(el.raan_rad),
        arg_perigee: Qty::from_raw(el.argp_rad),
        mean_anomaly: Qty::from_raw(el.mean_anomaly_rad),
    }
}
