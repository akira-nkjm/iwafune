//! 通過（スイングバイ）のある遷移の日付の探索のテスト。突き合わせ先は、同じ日付を書いたマヌーバの `propulsion.maneuvers`
//! （連鎖の解き方も ΔV の数え方もそこと同じ）。暦は解析式（このクレートのテストは DE440 を登録しない）。
//! 地球 → 金星の通過 → 火星の遷移は本物の航法の候補ではなく、解析の突き合わせのための設定（概念設計の既定の高度の下限で解く）。
//! 文献の数値との突き合わせは、通過のある遷移については用意していない（出典まで確かめられる数値が無いため。docs に書いた）。
//! 許容誤差と期待値は合わせて調整しない。

use super::super::swing_by_tests::{swing_input, venus};
use super::super::{DepartureProvider, DvBudget, FlybyInput, Maneuvers};
use super::*;
use sat_astro::constants::SECONDS_PER_DAY;
use sat_astro::ephem::CentralBody;
use sat_astro::time::Epoch;

fn epoch(s: &str) -> Epoch {
    Epoch::parse(s).unwrap()
}

/// 2028-10-01 出発・2029-01-10 金星・2030-03-01 到着（`swing_by_tests` の実現できる日付）で、金星が 1 つ。
fn one_flyby() -> PropulsionInput {
    let mut i = swing_input(vec![venus("2029-01-10T00:00:00Z")]);
    i.maneuvers[0].transfer.as_mut().unwrap().arrival = epoch("2030-03-01T00:00:00Z");
    i
}

fn earth(date: &str) -> FlybyInput {
    FlybyInput {
        body: CentralBody::Earth,
        date: epoch(date),
        min_altitude: None,
        powered: true,
    }
}

fn run(i: &PropulsionInput, settings: LaunchWindowSettings) -> LaunchWindowResult {
    LaunchWindow::run(&LaunchWindowInput {
        propulsion: i.clone(),
        settings,
    })
    .unwrap()
    .windows
    .remove(0)
}

/// 点の日付（出発・通過・到着）を書いたマヌーバの入力。
fn with_dates(i: &PropulsionInput, p: &LaunchWindowPoint) -> PropulsionInput {
    let mut j = i.clone();
    let t = j.maneuvers[0].transfer.as_mut().unwrap();
    t.departure = p.departure;
    t.arrival = p.arrival;
    for (f, d) in t.flybys.iter_mut().zip(&p.flyby_dates) {
        f.date = *d;
    }
    j
}

/// 最小の点の値は、同じ日付を書いたマヌーバの `propulsion.maneuvers` と一致する（C3・v∞・赤緯・ΔV が完全に同じ）。
/// 通過はどれも実現できる（`feasible`）。`dv_budget` の遷移の結果も同じ日付で同じ C3。
#[test]
fn the_minimum_equals_the_maneuvers_analysis_at_those_dates() {
    let i = one_flyby();
    let w = run(&i, LaunchWindowSettings::default());
    let min = w.minimum_delta_v.clone().unwrap();
    assert_eq!(min.flyby_dates.len(), 1);
    let j = with_dates(&i, &min);
    let m = Maneuvers::run(&j).unwrap().maneuvers.remove(0);
    let d = m.dated_transfer.clone().unwrap();
    assert!(d.flybys.iter().all(|f| f.feasible));
    assert_eq!(min.delta_v.value(), m.ideal_delta_v.value());
    assert_eq!(min.c3.value(), d.c3.value());
    assert_eq!(min.v_inf_departure.value(), d.v_inf_departure.value());
    assert_eq!(min.v_inf_arrival.value(), d.v_inf_arrival.value());
    assert_eq!(
        min.departure_declination.value(),
        d.departure_declination.value()
    );
    assert_eq!(min.time_of_flight.value(), d.time_of_flight.value());
    let b = DvBudget::run(&j).unwrap();
    assert_eq!(b.dated_transfers[0].c3.value(), min.c3.value());
    assert_eq!(b.dated_transfers[0].flybys.len(), 1);
    // 通過の日は出発と到着のあいだ
    assert!(min.departure.seconds_since(min.flyby_dates[0]) < 0.0);
    assert!(min.flyby_dates[0].seconds_since(min.arrival) < 0.0);
}

/// 最小は格子のどの点より大きくなく、書かれた日付が実現できるならその値より大きくない。
/// 書かれた日付の点は、通過の日も書かれたもので `propulsion.maneuvers` と一致する。
#[test]
fn the_minimum_is_below_the_grid_and_the_written_dates() {
    let i = one_flyby();
    let w = run(&i, LaunchWindowSettings::default());
    let min = w.minimum_delta_v.clone().unwrap().delta_v.value();
    let grid_min = layer(&w, LaunchWindowQuantity::DeltaV)
        .iter()
        .flatten()
        .fold(f64::INFINITY, |a, b| a.min(*b));
    assert!(min <= grid_min, "{min} vs {grid_min}");
    let wr = w.written.clone().unwrap().point.unwrap();
    let reference = Maneuvers::run(&i).unwrap().maneuvers.remove(0);
    assert_eq!(wr.delta_v.value(), reference.ideal_delta_v.value());
    assert_eq!(wr.flyby_dates, vec![epoch("2029-01-10T00:00:00Z")]);
    assert!(min <= wr.delta_v.value());
}

fn layer(w: &LaunchWindowResult, kind: LaunchWindowQuantity) -> &[Option<f64>] {
    &w.grid
        .layers
        .iter()
        .find(|l| l.kind == kind)
        .unwrap()
        .values
}

/// 実現できない通過の日付は除いて数える: 解こうとした点 = 解けた点 + 実現できる日付が無かった点（この設定では解けないエラーは無い）、
/// 除いた回数は連鎖を解いた回数の内数で、1 つ以上ある。最小の点の通過は実現できる。
#[test]
fn infeasible_flybys_are_excluded_and_counted() {
    let w = run(&one_flyby(), LaunchWindowSettings::default());
    let s = w.flyby_search.clone().unwrap();
    assert_eq!(s.bodies, vec![CentralBody::Venus]);
    assert!(s.infeasible_solves > 0 && s.infeasible_solves < s.chain_solves);
    assert!(s.infeasible_cells > 0);
    assert_eq!(w.evaluated_cells, w.solved_cells + s.infeasible_cells);
    assert!(s.refine_evaluations > 0 && s.refine_evaluations < s.chain_solves);
    assert!(!w.notes.contains(&LaunchWindowNote::RefinementIncomplete));
}

/// 通過のある探索の費用: 既定は 60 × 60 の格子で、連鎖を解く回数は 2 桁万回に収まる（実測は docs）。
#[test]
fn the_default_flyby_grid_and_solve_count_are_bounded() {
    let w = run(&one_flyby(), LaunchWindowSettings::default());
    assert_eq!(w.span.departure_steps, DEFAULT_FLYBY_STEPS[1]);
    assert_eq!(w.span.arrival_steps, DEFAULT_FLYBY_STEPS[1]);
    let solves = w.flyby_search.unwrap().chain_solves;
    assert!((1_000..200_000).contains(&solves), "{solves}");
    // 飛行時間の範囲は書かれた飛行時間（517 日）の 0.5〜1.5 倍
    let written = epoch("2030-03-01T00:00:00Z").seconds_since(epoch("2028-10-01T00:00:00Z"));
    assert_eq!(w.span.time_of_flight_min.value(), (0.5 * written).round());
    assert_eq!(w.span.time_of_flight_max.value(), (1.5 * written).round());
}

/// 最小の点のまわりを一様に探っても（固定の乱数、±3 日、4000 点）、実現できる点の ΔV の改善は 0.1 % に収まる。
/// 探索は実現できる範囲の縁の細い谷を乱数で仕上げるので、厳密な局所の最小ではなく、この精度（実測は標準出力に出す）が主張できる範囲。
#[test]
fn sampled_neighbours_of_the_minimum_improve_by_under_a_tenth_of_a_percent() {
    let i = one_flyby();
    let w = run(&i, LaunchWindowSettings::default());
    let min = w.minimum_delta_v.clone().unwrap();
    let mut state = 0x1234_5678_9ABC_DEF1_u64;
    let mut uniform = || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        (state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
    };
    let mut around = |p: &LaunchWindowPoint, r: f64| {
        let mut q = p.clone();
        q.departure = p.departure.plus_seconds((uniform() * r).round());
        q.arrival = p.arrival.plus_seconds((uniform() * r).round());
        q.flyby_dates = p
            .flyby_dates
            .iter()
            .map(|d| d.plus_seconds((uniform() * r).round()))
            .collect();
        q
    };
    let mut best_feasible = f64::INFINITY;
    for _ in 0..4000 {
        let q = around(&min, 3.0 * SECONDS_PER_DAY);
        let Ok(out) = Maneuvers::run(&with_dates(&i, &q)) else {
            continue;
        };
        let m = &out.maneuvers[0];
        if m.dated_transfer.as_ref().unwrap().flybys[0].feasible {
            best_feasible = best_feasible.min(m.ideal_delta_v.value());
        }
    }
    eprintln!(
        "best sampled {best_feasible} vs minimum {}",
        min.delta_v.value()
    );
    assert!(
        best_feasible >= 0.999 * min.delta_v.value(),
        "{best_feasible} vs {}",
        min.delta_v.value()
    );
}

/// 動力なしと書かれた通過: 探索は近点の噴射（超過速度の差）が小さくなる日付を選ぶ（その分は報告の ΔV に足さない）。
/// 弾道（差が `BALLISTIC_DV_M_S` 以下）の点に着くか、着かなければ注記と残りが出る。注記が無ければ、
/// 最小の点の日付を書いたマヌーバに `swing_by_unpowered` の警告は付かない。報告の ΔV は `propulsion.maneuvers` と同じ。
#[test]
fn an_unpowered_flyby_is_driven_to_ballistic_dates_or_noted() {
    let mut i = one_flyby();
    i.maneuvers[0].transfer.as_mut().unwrap().flybys[0].powered = false;
    let w = run(&i, LaunchWindowSettings::default());
    let min = w.minimum_delta_v.clone().unwrap();
    let s = w.flyby_search.clone().unwrap();
    let noted = w
        .notes
        .contains(&LaunchWindowNote::UnpoweredFlybyNotBallistic);
    eprintln!(
        "UNPOWERED min {} residual {} noted {noted} refine {} solves {}",
        min.delta_v.value(),
        s.unpowered_residual.value(),
        s.refine_evaluations,
        s.chain_solves
    );
    let m = Maneuvers::run(&with_dates(&i, &min))
        .unwrap()
        .maneuvers
        .remove(0);
    assert_eq!(m.ideal_delta_v.value(), min.delta_v.value());
    let warned = m.checks.iter().any(|c| c.key == "swing_by_unpowered");
    assert_eq!(warned, noted);
    assert_eq!(
        noted,
        s.unpowered_residual.value() > sat_astro::transfer::BALLISTIC_DV_M_S
    );
    // 速度の差を基準に入れない探索（ΔV だけを最小にする）では、この設定の最小は 6922 m/s で残りが 3401 m/s だった（実測）。
    // 入れた探索の残りはその 100 分の 1 未満（1 s まで詰める最後の段で弾道の範囲に入る）
    assert!(
        s.unpowered_residual.value() < 34.0,
        "{}",
        s.unpowered_residual.value()
    );
    let f = &m.dated_transfer.unwrap().flybys[0];
    assert_eq!(f.delta_v.value(), s.unpowered_residual.value());
}

/// 通過が 2 つ（金星 → 地球）でも同じ経路: 最小の点は同じ日付の `propulsion.maneuvers` と一致し、通過の日は増えていく順。
#[test]
fn two_flybys_use_the_same_chain() {
    let mut i = swing_input(vec![
        venus("2029-01-10T00:00:00Z"),
        earth("2029-10-01T00:00:00Z"),
    ]);
    i.maneuvers[0].transfer.as_mut().unwrap().arrival = epoch("2030-10-01T00:00:00Z");
    let settings = LaunchWindowSettings {
        departure_steps: Some(20),
        arrival_steps: Some(20),
        ..Default::default()
    };
    let w = run(&i, settings);
    let min = w.minimum_delta_v.clone().unwrap();
    assert_eq!(min.flyby_dates.len(), 2);
    let ordered = [
        min.departure,
        min.flyby_dates[0],
        min.flyby_dates[1],
        min.arrival,
    ];
    assert!(ordered.windows(2).all(|p| p[0].seconds_since(p[1]) < 0.0));
    let m = Maneuvers::run(&with_dates(&i, &min))
        .unwrap()
        .maneuvers
        .remove(0);
    assert_eq!(m.ideal_delta_v.value(), min.delta_v.value());
    assert!(m.dated_transfer.unwrap().flybys.iter().all(|f| f.feasible));
    assert_eq!(
        w.flyby_search.unwrap().bodies,
        vec![CentralBody::Venus, CentralBody::Earth]
    );
}

/// 直接の遷移の出力に通過の欄は出ない（JSON は通過のない探索のまま）。
#[test]
fn the_direct_output_has_no_flyby_fields() {
    let mut i = one_flyby();
    i.maneuvers[0].transfer.as_mut().unwrap().flybys.clear();
    let out = LaunchWindow::run(&LaunchWindowInput {
        propulsion: i,
        settings: LaunchWindowSettings::default(),
    })
    .unwrap();
    let text = serde_json::to_string(&out).unwrap();
    assert!(!text.contains("flyby"), "{text}");
    assert_eq!(out.windows[0].span.departure_steps, DEFAULT_STEPS);
}

/// 通過の数の上限と格子の点の数の上限は断る。出発を打ち上げ機がする遷移も同じ（書かれた `departure_by` を保つ）。
#[test]
fn limits_are_refused_and_departure_by_is_kept() {
    let mut four = one_flyby();
    four.maneuvers[0].transfer.as_mut().unwrap().flybys = vec![
        venus("2028-12-01T00:00:00Z"),
        earth("2029-03-01T00:00:00Z"),
        venus("2029-06-01T00:00:00Z"),
        earth("2029-09-01T00:00:00Z"),
    ];
    let e = LaunchWindow::run(&LaunchWindowInput {
        propulsion: four,
        settings: LaunchWindowSettings::default(),
    })
    .unwrap_err();
    assert!(e.to_string().contains("at most 3 flybys"), "{e}");
    let e = LaunchWindow::run(&LaunchWindowInput {
        propulsion: one_flyby(),
        settings: LaunchWindowSettings {
            departure_steps: Some(101),
            arrival_steps: Some(100),
            ..Default::default()
        },
    })
    .unwrap_err();
    assert!(e.to_string().contains("with flybys"), "{e}");
    let mut i = one_flyby();
    i.maneuvers[0].transfer.as_mut().unwrap().departure_by = DepartureProvider::Launcher;
    let w = run(&i, LaunchWindowSettings::default());
    let min = w.minimum_delta_v.unwrap();
    let m = Maneuvers::run(&with_dates(&i, &min))
        .unwrap()
        .maneuvers
        .remove(0);
    assert_eq!(m.ideal_delta_v.value(), min.delta_v.value());
}
