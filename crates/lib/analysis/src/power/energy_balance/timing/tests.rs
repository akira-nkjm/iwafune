//! 開始の選び方（指定の開始・許された範囲でずらす・指定なし）を、標本の格子の上で手計算と比べる。

use super::*;

const DT: f64 = 10.0;
const PERIOD: f64 = 600.0;

/// 刻み 10 s・3000 s、周期 600 s。周期の中の 0〜400 s が日照・400〜600 s が日陰。
/// 食明けは 600・1200・1800・2400・3000 s、食入りは 400・1000・1600・2200・2800 s。
fn grid() -> (Vec<f64>, Vec<bool>) {
    let t: Vec<f64> = (0..=300).map(|k| f64::from(k) * DT).collect();
    let lit = t.iter().map(|x| x.rem_euclid(PERIOD) < 400.0).collect();
    (t, lit)
}

fn at(from: EclipseEvent, offset: f64) -> EventStart {
    EventStart {
        from,
        offset: Qty::from_raw(offset),
    }
}

fn timing(when: TimeWindow, start: Option<EventStart>) -> ModeTiming {
    ModeTiming { when, start }
}

#[test]
fn events_are_the_exit_and_entry_samples() {
    let (t, lit) = grid();
    let times = |from| -> Vec<f64> {
        event_samples(&lit, from)
            .into_iter()
            .map(|k| t[k])
            .collect()
    };
    assert_eq!(
        times(EclipseEvent::EclipseExit),
        [600.0, 1200.0, 1800.0, 2400.0, 3000.0]
    );
    assert_eq!(
        times(EclipseEvent::EclipseEntry),
        [400.0, 1000.0, 1600.0, 2200.0, 2800.0]
    );
}

#[test]
fn start_samples_cover_one_orbit_before_the_recovery_orbit() {
    // 窓 100 s: 終わり 3000 − 100 − 2×600 = 1700 s から 600 s 分（1700 以上 2300 未満）
    let (t, _) = grid();
    let s = start_samples(&t, PERIOD, 100.0);
    assert_eq!(
        (t[s[0]], t[*s.last().unwrap()], s.len()),
        (1700.0, 2290.0, 60)
    );
}

#[test]
fn a_fixed_start_is_evaluated_only_there() {
    let (t, lit) = grid();
    // 食明け 1800 s + 50 s → 1850 s（1700〜2300 s に入る食明けはこの 1 つだけ）
    let s = select_starts(
        &t,
        &lit,
        PERIOD,
        100.0,
        &timing(TimeWindow::Any, Some(at(EclipseEvent::EclipseExit, 50.0))),
    );
    assert_eq!(s.timing, DurationTiming::Fixed);
    assert_eq!(s.starts.iter().map(|&k| t[k]).collect::<Vec<_>>(), [1850.0]);
    assert_eq!(s.note, None);
    // 食入り 2200 s、オフセット 0
    let s = select_starts(
        &t,
        &lit,
        PERIOD,
        100.0,
        &timing(TimeWindow::Any, Some(at(EclipseEvent::EclipseEntry, 0.0))),
    );
    assert_eq!(s.starts.iter().map(|&k| t[k]).collect::<Vec<_>>(), [2200.0]);
    // 負のオフセット: 1800 − 100 = 1700 s（ずらす 1 周の最初の標本）
    let s = select_starts(
        &t,
        &lit,
        PERIOD,
        100.0,
        &timing(TimeWindow::Any, Some(at(EclipseEvent::EclipseExit, -100.0))),
    );
    assert_eq!(s.starts.iter().map(|&k| t[k]).collect::<Vec<_>>(), [1700.0]);
}

#[test]
fn a_window_only_shifts_inside_the_allowed_windows() {
    let (t, lit) = grid();
    let times = |w: TimeWindow| -> Vec<f64> {
        let s = select_starts(&t, &lit, PERIOD, 100.0, &timing(w, None));
        assert_eq!(s.timing, DurationTiming::Windowed);
        assert_eq!(s.note, None);
        s.starts.iter().map(|&k| t[k]).collect()
    };
    // 日陰: 1600〜1800 s に収まるのは 1700 s だけ（1710 s からは窓の終わりが日照に掛かる）と、2200〜2400 s の 2200〜2290 s
    let eclipse = times(TimeWindow::Eclipse);
    assert_eq!(eclipse.len(), 11);
    assert_eq!(
        (eclipse[0], eclipse[1], *eclipse.last().unwrap()),
        (1700.0, 2200.0, 2290.0)
    );
    // 日照: 1800〜2200 s に収まるのは 1800〜2100 s の 31 点
    let sunlit = times(TimeWindow::Sunlit);
    assert_eq!(sunlit.len(), 31);
    assert_eq!((sunlit[0], *sunlit.last().unwrap()), (1800.0, 2100.0));
}

#[test]
fn no_timing_searches_the_whole_orbit_and_says_so() {
    let (t, lit) = grid();
    let s = select_starts(&t, &lit, PERIOD, 100.0, &ModeTiming::default());
    assert_eq!(s.timing, DurationTiming::Unspecified);
    assert_eq!(s.starts.len(), 60);
    assert_eq!(s.note, None);
    assert!(ModeTiming::default().is_unspecified());
    // `when = any` だけも指定なし
    assert!(timing(TimeWindow::Any, None).is_unspecified());
    assert!(!timing(TimeWindow::Eclipse, None).is_unspecified());
}

#[test]
fn unsatisfiable_timing_falls_back_to_the_search_with_a_reason() {
    let (t, lit) = grid();
    // 日照は 400 s しかないので、500 s の窓は収まらない
    let s = select_starts(&t, &lit, PERIOD, 500.0, &timing(TimeWindow::Sunlit, None));
    assert_eq!(s.timing, DurationTiming::Unspecified);
    assert!(
        s.note
            .as_deref()
            .unwrap()
            .contains("does not fit in any sunlit window")
    );
    // 食が無ければ食明けは決まらない
    let all_lit = vec![true; t.len()];
    let s = select_starts(
        &t,
        &all_lit,
        PERIOD,
        100.0,
        &timing(TimeWindow::Any, Some(at(EclipseEvent::EclipseExit, 0.0))),
    );
    assert_eq!(s.timing, DurationTiming::Unspecified);
    assert!(s.note.as_deref().unwrap().contains("eclipse exit"));
}

#[test]
fn a_fixed_start_outside_the_window_is_flagged() {
    let (t, lit) = grid();
    // 食明けは日照の始まりなので、日陰の窓（eclipse）とは合わない
    let s = select_starts(
        &t,
        &lit,
        PERIOD,
        100.0,
        &timing(
            TimeWindow::Eclipse,
            Some(at(EclipseEvent::EclipseExit, 0.0)),
        ),
    );
    assert_eq!(s.timing, DurationTiming::Fixed);
    assert!(
        s.note
            .as_deref()
            .unwrap()
            .contains("not inside the eclipse window")
    );
}

#[test]
fn the_sequence_origin_follows_the_first_event() {
    let (t, lit) = grid();
    assert_eq!(sequence_origin(&t, &lit, None), (0.0, None));
    // 最初の食明け 600 s + 120 s
    let (o, note) = sequence_origin(&t, &lit, Some(&at(EclipseEvent::EclipseExit, 120.0)));
    assert_eq!((o, note), (720.0, None));
    let (o, _) = sequence_origin(&t, &lit, Some(&at(EclipseEvent::EclipseEntry, -30.0)));
    assert_eq!(o, 370.0);
    // 食が無ければ解析の始めに戻して理由を返す
    let (o, note) = sequence_origin(
        &t,
        &vec![true; t.len()],
        Some(&at(EclipseEvent::EclipseExit, 0.0)),
    );
    assert_eq!(o, 0.0);
    assert!(note.unwrap().contains("eclipse exit"));
}
