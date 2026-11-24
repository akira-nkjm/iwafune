//! 非定常のヒータ（ヒステリシスの帯・デューティ・周期・モードで無効・判定）のテスト。
//!
//! 共通の設定: 1 節点 `box`（熱容量 C = 900 J/K）を、G = 0.5 W/K で 250 K の境界につなぐ（外面なし、入熱 0）。
//! ヒータは 270 K を下回るとオン・280 K を上回るとオフ、オンのとき 20 W。積分の刻みは 10 s（サンプルの間隔）。
//! オフなら 250 K へ、オンなら 250 + 20/0.5 = 290 K へ近づく（時定数 τ = C/G = 1800 s の 1 次遅れ）。

use super::*;
use crate::modes::{ModeDef, Redundancy, StateAssignment};
use crate::thermal::network::{ThermalConductiveLink, tests::node};
use crate::thermal::orbit_heat::fixtures::leo_timeline;

const STEP_S: f64 = 10.0;
const TAU_S: f64 = 1800.0;

fn heater(power: f64) -> ThermalHeater {
    ThermalHeater {
        id: "H".to_owned(),
        node: "box".to_owned(),
        on_below: Qty::from_raw(270.0),
        off_above: Qty::from_raw(280.0),
        power: Qty::from_raw(power),
        count: 1,
        redundancy: Redundancy::None,
        states: Vec::new(),
    }
}

fn input(span_s: f64, initial_k: f64, power: f64) -> ThermalTransientInput {
    ThermalTransientInput {
        network: ThermalNetwork {
            plates: Vec::new(),
            mounts: None,
            nodes: vec![node("sink", Some(250.0)), node("box", None)],
            conductive: vec![ThermalConductiveLink {
                a: "sink".to_owned(),
                b: "box".to_owned(),
                conductance: Qty::from_raw(0.5),
            }],
            radiative: vec![],
            enclosure: None,
            surfaces: vec![],
        },
        timeline: leo_timeline(span_s, STEP_S),
        environment: ThermalEnvironment::default(),
        phases: vec![],
        heaters: vec![heater(power)],
        initial: ThermalInitial::Uniform {
            temperature: Qty::from_raw(initial_k),
        },
        max_step: None,
        rel_tol: None,
        abs_tol: None,
        margin: ThermalMarginPolicy::default(),
    }
}

/// 時系列の `heater:H` の値（W）。
fn power_series(out: &ThermalTransientOutput) -> &[f64] {
    &out.series
        .channels
        .iter()
        .find(|c| c.key == "heater:H")
        .unwrap()
        .values
}

/// 手計算の切り替えの時刻（1 次遅れの厳密解）。260 K から始めてオン: 280 K まで τ ln((290 − 260)/(290 − 280)) = 1800 ln 3 ≈ 1977.5 s
/// （最初のオン。以降はこれが最長）、オフ: 280 K から 270 K まで τ ln((280 − 250)/(270 − 250)) = 1800 ln 1.5 ≈ 729.8 s、
/// オン: 270 K から 280 K まで τ ln((290 − 270)/(290 − 280)) = 1800 ln 2 ≈ 1247.7 s。周期 ≈ 1977.5 s、デューティ ≈ 0.631。
fn exact_switch_times(span_s: f64) -> Vec<f64> {
    let mut times = vec![TAU_S * 3.0_f64.ln()];
    let (off, on) = (TAU_S * 1.5_f64.ln(), TAU_S * 2.0_f64.ln());
    while let Some(&last) = times.last() {
        let next = last + if times.len() % 2 == 1 { off } else { on };
        if next > span_s {
            break;
        }
        times.push(next);
    }
    times
}

/// 時刻 `t` が `switches`（最初はオンから始まる）でオンか。
fn exact_on(switches: &[f64], t: f64) -> bool {
    switches.iter().filter(|s| **s <= t).count() % 2 == 0
}

/// 手計算（1 次遅れの厳密解 `exact_switch_times`）との比較。切り替えは積分器が設定温度に届く時刻を密出力で求めるので、
/// - 標本の温度は、最初に帯に入った後は帯 [270, 280] の外に出ない（許容 `EVENT_TOL_K` = 1e-5 K。#297 の RK4 の刻み単位の切り替えでは
///   刻み 10 s の間の温度変化 0.167 K だけはみ出た）。
/// - 切り替えの回数は手計算と同じ、オン時間（デューティ）は 1e-8 s の桁（デューティで 1e-9）で手計算と一致する（#297 は ±0.02）。最長のオンは 1977.5 s に 1e-5 s で一致（#297 は ±30 s）。
/// - 発熱の系列（標本の時刻の状態）は、切り替えの時刻に近い標本を除いて、手計算のオンオフと一致する。
#[test]
fn bang_bang_period_duty_and_band_by_hand() {
    let span = 40_000.0;
    let out = ThermalTransient::run(&input(span, 260.0, 20.0)).unwrap();
    let t = &out.series.t;
    let temps = &out.series.channels[1].values;
    let p = power_series(&out);
    let switches = exact_switch_times(span);

    let first_on = TAU_S * 3.0_f64.ln();
    let settled = || t.iter().zip(temps).filter(|(t, _)| **t > first_on);
    let (lo, hi) = settled().fold((f64::MAX, f64::MIN), |(a, b), (_, v)| {
        (a.min(*v), b.max(*v))
    });
    assert!(
        lo > 270.0 - 2.0 * heater_control::EVENT_TOL_K
            && hi < 280.0 + 2.0 * heater_control::EVENT_TOL_K,
        "{lo} {hi}"
    );
    assert!(lo < 270.5 && hi > 279.5, "the band is used: {lo} {hi}");

    // 発熱の系列: 手計算のオンオフと一致（切り替えの 1e-3 s 以内の標本は除く）
    for (ti, pi) in t.iter().zip(p) {
        if switches.iter().all(|s| (s - ti).abs() > 1e-3) {
            assert_eq!(*pi > 0.0, exact_on(&switches, *ti), "t = {ti}");
        }
    }

    // 切り替えの回数とオン時間
    let h = &out.heaters[0];
    assert_eq!(h.switchings as usize, switches.len());
    let mut on_time = 0.0;
    let mut start = 0.0;
    for (k, s) in switches.iter().chain([&span]).enumerate() {
        if k % 2 == 0 {
            on_time += s - start;
        }
        start = *s;
    }
    let exact_duty = on_time / span;
    assert!(
        (h.duty_fraction.value() - exact_duty).abs() < 1e-9,
        "{} vs {exact_duty}",
        h.duty_fraction.value()
    );
    // 平均電力 = デューティ × 容量
    assert!((h.mean_power.value() - h.duty_fraction.value() * 20.0).abs() < 1e-12);
    // 最後の 1 周（軌道の周期）のデューティ: 手計算の切り替えの時刻から窓の中のオン時間を数える
    let window = leo_timeline(1.0, STEP_S).orbit().cycle().length_s();
    let (w0, mut on_window) = (span - window, 0.0);
    let edges: Vec<f64> = [0.0]
        .into_iter()
        .chain(switches.iter().copied())
        .chain([span])
        .collect();
    for (k, w) in edges.windows(2).enumerate() {
        if k % 2 == 0 {
            on_window += (w[1].min(span) - w[0].max(w0)).max(0.0);
        }
    }
    assert!(
        (h.duty_fraction_last_orbit.value() - on_window / window).abs() < 1e-9,
        "{} vs {}",
        h.duty_fraction_last_orbit.value(),
        on_window / window
    );
    // 最長のオン時間は最初のオン（260 K から 280 K まで）
    assert!(
        (h.max_on_duration.value() - first_on).abs() < 1e-5,
        "{} vs {first_on}",
        h.max_on_duration.value()
    );
    assert!(!h.saturated);
    assert!(out.checks.iter().any(|c| c.key == "heater:H"));
    // 切り替えは積分器が止めたイベントで、その数だけ止まっている
    assert_eq!(out.integration.events as usize, switches.len());
}

/// 帯の中の初期温度（275 K）はオフから始める（最初のサンプルの発熱は 0）。下回れば（265 K）オンから始める。
#[test]
fn initial_state_inside_the_band_is_off() {
    let inside = ThermalTransient::run(&input(600.0, 275.0, 20.0)).unwrap();
    assert_eq!(power_series(&inside)[0], 0.0);
    let below = ThermalTransient::run(&input(600.0, 265.0, 20.0)).unwrap();
    assert_eq!(power_series(&below)[0], 20.0);
}

/// モードの状態が `off` だけの区間はサーモスタットを切る: 発熱 0、切り替え 0、節点は 250 K へ冷える。
/// `off` に `on` の duty を書いた区間（デモの書き方）は働く。
#[test]
fn disabled_by_mode_off_state() {
    let phase = |state: &str, duty: &[(&str, f64)]| ThermalHeatPhase {
        label: "M".to_owned(),
        duration: None,
        loads: vec![],
        mode_loads: Some(ThermalModeLoads {
            mode: ModeDef {
                id: "M".into(),
                label: "M".into(),
                assignments: vec![StateAssignment {
                    part: "H".into(),
                    state: state.into(),
                    duty: duty.iter().map(|(s, f)| ((*s).into(), *f)).collect(),
                    active: None,
                    spare_state: None,
                }],
                eclipse: Vec::new(),
            },
            parts: vec![],
        }),
    };
    let mut off = input(20_000.0, 275.0, 20.0);
    off.phases = vec![phase("off", &[])];
    let out = ThermalTransient::run(&off).unwrap();
    assert!(power_series(&out).iter().all(|p| *p == 0.0));
    assert_eq!(out.heaters[0].switchings, 0);
    assert_eq!(out.heaters[0].duty_fraction.value(), 0.0);
    let last = *out.series.channels[1].values.last().unwrap();
    assert!((last - (250.0 + 25.0 * (-20_000.0_f64 / TAU_S).exp())).abs() < 1e-8);

    let mut with_duty = off.clone();
    with_duty.phases = vec![phase("off", &[("on", 0.2)])];
    let out = ThermalTransient::run(&with_duty).unwrap();
    assert!(out.heaters[0].switchings > 4);
}

/// 容量が足りない（5 W → オンでも 250 + 5/0.5 = 260 K までしか上がらず、270 K に届かない）なら、最後の 1 周ずっとオンで飽和 → 違反。
/// デューティが警告の値を超えるが飽和でない（ほぼ 20 W で 288 K 近くまで上がる設計を、警告を 0.5 にして）なら警告。
#[test]
fn saturation_is_a_violation_and_high_duty_a_warning() {
    let out = ThermalTransient::run(&input(30_000.0, 265.0, 5.0)).unwrap();
    assert!(out.heaters[0].saturated);
    let verdict = |o: &ThermalTransientOutput| {
        o.checks
            .iter()
            .find(|c| c.key == "heater:H")
            .unwrap()
            .verdict
    };
    assert_eq!(verdict(&out), Verdict::Violation);

    let mut warn = input(40_000.0, 260.0, 20.0);
    warn.margin.heater_duty_warning = Some(sat_units::typed::Fraction::new(0.5));
    let out = ThermalTransient::run(&warn).unwrap();
    assert!(!out.heaters[0].saturated);
    assert_eq!(verdict(&out), Verdict::Warning);
    // 既定の警告 0.8 ならデューティ 0.63 は OK
    let out = ThermalTransient::run(&input(40_000.0, 260.0, 20.0)).unwrap();
    assert_eq!(verdict(&out), Verdict::Ok);
}
