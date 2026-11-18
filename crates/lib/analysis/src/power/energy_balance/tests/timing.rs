//! 時刻の指定（最長継続時間の開始の選び方・並びの基準）を手計算と比べる。

use super::operation::{from_battery, mode_case, segment, with_loads};
use super::{input, pz_panel};
use crate::power::energy_balance::{EnergyBalance, EnergyBalanceInput};
use crate::spec::{Analysis, Verdict};
use sat_units::typed::Qty;

use crate::power::energy_balance::{DurationTiming, ModeTiming, SequenceTiming, TimeWindow};
use crate::power::energy_balance::{EclipseEvent, EventStart};

fn after(from: EclipseEvent, offset_s: f64) -> Option<EventStart> {
    Some(EventStart {
        from,
        offset: Qty::from_raw(offset_s),
    })
}

/// 太陽指向の +Z 面 1 m²（日照中の発電は負荷よりずっと大きい）。基本のモードは並びの最初の IDLE（0 W）、
/// 600 s だけ続けるモード B（19 W）に時刻の指定を付ける。
fn timed(timing: ModeTiming) -> EnergyBalanceInput {
    let mut case = mode_case("B", Some(600.0));
    case.timing = timing;
    let mut i = with_loads(input(vec![pz_panel()], vec![case]));
    i.sequence = vec![segment("IDLE", 3000.0), segment("A", 600.0)];
    i
}

fn duration_of(
    timing: ModeTiming,
) -> (crate::power::energy_balance::EnergyBalanceDuration, String) {
    let out = EnergyBalance::run(&timed(timing)).unwrap();
    let c = &out.cases[0];
    let basis = c
        .checks
        .iter()
        .find(|x| x.key == "dod")
        .unwrap()
        .message
        .clone();
    (c.by_max_duration.clone().unwrap(), basis)
}

/// 日陰に 600 s 置いたときの DoD（B は 19 W → 電池から 20 W、日陰に入る前は満充電）。
fn eclipse_dod() -> f64 {
    from_battery(19.0) * 600.0 / 360_000.0
}

#[test]
fn start_at_eclipse_entry_is_evaluated_only_there() {
    let start = after(EclipseEvent::EclipseEntry, 0.0);
    let (d, basis) = duration_of(ModeTiming {
        when: TimeWindow::Any,
        start,
    });
    assert_eq!(d.timing, DurationTiming::Fixed);
    assert_eq!(d.starts, 1);
    // 窓は食入りから始まり、600 s がまるごと日陰（食は 600 s より長い）
    assert!((d.eclipse_overlap.value() - 600.0).abs() < 1e-6);
    // 電池は台形則で積分する。窓の最初の刻み（日照の標本 → 窓の最初の標本）は充電側で満充電に切られ、
    // 最後の刻み（窓の最後 → 基本のモード）は半分だけ放電するので、放電は 59 刻み + 半刻み = 595 s 分
    let expect = from_battery(19.0) * 595.0 / 360_000.0;
    assert!((d.max_dod_from_start.value() - expect).abs() < 1e-9);
    // 開始が 1 つなので最良も同じ
    assert!((d.best_max_dod.value() - expect).abs() < 1e-9);
    assert!(basis.contains("start fixed at eclipse entry + "), "{basis}");
}

#[test]
fn start_at_eclipse_exit_can_be_better_than_the_worst_start() {
    // 食明けから始めれば窓は日照中。発電が負荷を上回るので SOC は下がらない（指定なしの最悪の 600 s 日陰より軽い）
    let (d, _) = duration_of(ModeTiming {
        when: TimeWindow::Any,
        start: after(EclipseEvent::EclipseExit, 0.0),
    });
    assert_eq!(d.timing, DurationTiming::Fixed);
    assert_eq!(d.eclipse_overlap.value(), 0.0);
    assert!(d.max_dod_from_start.value() < 1e-12);
}

#[test]
fn when_only_shifts_inside_the_allowed_windows() {
    let (unspecified, basis) = duration_of(ModeTiming::default());
    assert_eq!(unspecified.timing, DurationTiming::Unspecified);
    assert!(basis.contains("timing unspecified"), "{basis}");
    assert!((unspecified.max_dod_from_start.value() - eclipse_dod()).abs() < 1e-9);
    // 日照だけ: 窓が日陰に掛からない開始の中の最悪 → DoD は 0
    let (sunlit, basis) = duration_of(ModeTiming {
        when: TimeWindow::Sunlit,
        start: None,
    });
    assert_eq!(sunlit.timing, DurationTiming::Windowed);
    assert_eq!(sunlit.eclipse_overlap.value(), 0.0);
    // 空の和が -0.0 にならない（JSON に -0.0 が出ない）
    assert!(sunlit.eclipse_overlap.value().is_sign_positive());
    assert!(sunlit.max_dod_from_start.value() < 1e-12);
    assert!(sunlit.starts < unspecified.starts && sunlit.starts > 1);
    assert!(basis.contains("allowed sunlit windows"), "{basis}");
    // 日陰だけ: 窓が日陰にまるごと入る開始の中の最悪 → 日陰 600 s の DoD
    let (eclipse, _) = duration_of(ModeTiming {
        when: TimeWindow::Eclipse,
        start: None,
    });
    assert_eq!(eclipse.timing, DurationTiming::Windowed);
    assert!((eclipse.eclipse_overlap.value() - 600.0).abs() < 1e-6);
    assert!((eclipse.max_dod_from_start.value() - eclipse_dod()).abs() < 1e-9);
    assert!(eclipse.starts < unspecified.starts && eclipse.starts > 1);
}

#[test]
fn a_window_longer_than_the_allowed_window_warns_and_searches_everywhere() {
    // 日陰は 1 周の 4 割ほど。最長継続時間を日陰より長くすると収まる開始が無い
    let mut i = timed(ModeTiming {
        when: TimeWindow::Eclipse,
        start: None,
    });
    i.cases[0].max_duration = Some(Qty::from_raw(4000.0));
    let out = EnergyBalance::run(&i).unwrap();
    let c = &out.cases[0];
    assert_eq!(
        c.by_max_duration.as_ref().unwrap().timing,
        DurationTiming::Unspecified
    );
    let note = c.checks.iter().find(|x| x.key == "timing").unwrap();
    assert_eq!(note.verdict, Verdict::Warning);
    assert!(note.message.contains("does not fit in any eclipse window"));
}

#[test]
fn a_sequence_anchored_at_eclipse_exit_starts_there() {
    // 発電なし（日照・日陰の別は負荷に効かない）。A（9.5 W）600 s → IDLE（0 W）3000 s を繰り返す
    let build = |anchor| {
        let mut i = with_loads(input(Vec::new(), vec![]));
        i.sequence = vec![segment("A", 600.0), segment("IDLE", 3000.0)];
        i.sequence_timing = SequenceTiming {
            name: Some("imaging_day".to_owned()),
            anchor,
        };
        EnergyBalance::run(&i).unwrap()
    };
    let load_at = |out: &crate::power::energy_balance::EnergyBalanceOutput, t: f64| {
        let b = &out.sequence.as_ref().unwrap().balance;
        let k = b
            .series
            .t
            .iter()
            .position(|&x| (x - t).abs() < 1e-6)
            .unwrap();
        b.series
            .channels
            .iter()
            .find(|c| c.key == "load")
            .unwrap()
            .values[k]
    };
    // 基準なし: 解析の始めが並びの先頭
    let plain = build(None);
    let s = plain.sequence.as_ref().unwrap();
    assert_eq!(
        (s.name.as_deref(), s.origin.value()),
        (Some("imaging_day"), 0.0)
    );
    assert_eq!(load_at(&plain, 0.0), 9.5);
    // 食明けから 100 s 後: 最初の食明けの標本 + 100 s が先頭
    let anchored = build(after(EclipseEvent::EclipseExit, 100.0));
    let origin = anchored.sequence.as_ref().unwrap().origin.value();
    let exit = anchored.eclipses[0].end.value();
    assert!(
        (origin - (exit + 100.0)).abs() <= 10.0 + 1e-6,
        "{origin} vs {exit}"
    );
    assert_eq!(load_at(&anchored, origin), 9.5);
    assert_eq!(load_at(&anchored, origin + 590.0), 9.5);
    assert_eq!(load_at(&anchored, origin + 600.0), 0.0);
    assert_eq!(load_at(&anchored, origin - 10.0), 0.0);
    // 周期（3600 s）で繰り返す
    assert_eq!(load_at(&anchored, origin + 3600.0), 9.5);
}
