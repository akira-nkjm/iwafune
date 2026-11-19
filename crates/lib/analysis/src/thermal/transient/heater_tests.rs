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

/// 手計算（1 次遅れの厳密解）: 280 K から 270 K へ冷える時間 t_off = τ ln((280 − 250)/(270 − 250)) = 1800 ln 1.5 ≈ 729.8 s、
/// 270 K から 280 K へ温まる時間 t_on = τ ln((290 − 270)/(290 − 280)) = 1800 ln 2 ≈ 1247.7 s。周期 ≈ 1977.5 s、デューティ ≈ 0.631。
/// 線形化の見積り（帯の中央 275 K の損失 P_loss = G (275 − 250) = 12.5 W）は
/// C ΔT/(P − P_loss) + C ΔT/P_loss = 900·10/7.5 + 900·10/12.5 = 1200 + 720 = 1920 s、デューティ P_loss/P = 0.625。
/// 刻み 10 s の間の温度変化は最大で (G (280 − 250)/C) · 10 s = 0.167 K なので、帯から出るのはその分まで。
#[test]
fn bang_bang_period_duty_and_band_by_hand() {
    let out = ThermalTransient::run(&input(40_000.0, 260.0, 20.0)).unwrap();
    let t = &out.series.t;
    let temps = &out.series.channels[1].values;
    let p = power_series(&out);

    // 最初に切り替わった後（最初の周期の後）は、帯 [270 − δ, 280 + δ] の中に収まる（δ = 0.167 K）
    let delta = 0.5 * 30.0 / 900.0 * STEP_S;
    let settled: Vec<f64> = t
        .iter()
        .zip(temps)
        .filter(|(t, _)| **t > 4_000.0)
        .map(|(_, v)| *v)
        .collect();
    let (lo, hi) = settled
        .iter()
        .fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
    assert!(
        lo > 270.0 - 2.0 * delta && hi < 280.0 + 2.0 * delta,
        "{lo} {hi}"
    );
    assert!(lo < 270.5 && hi > 279.5, "the band is used: {lo} {hi}");

    // 周期: オンへの立ち上がりの間隔の平均が厳密解 1977.5 s の 2 % 以内
    let rising: Vec<f64> = (1..p.len())
        .filter(|&i| p[i] > 0.0 && p[i - 1] == 0.0)
        .map(|i| t[i])
        .collect();
    assert!(rising.len() >= 15, "{}", rising.len());
    let period = (rising[rising.len() - 1] - rising[0]) / (rising.len() - 1) as f64;
    let exact = TAU_S * (1.5_f64.ln() + 2.0_f64.ln());
    assert!((period - exact).abs() < 0.02 * exact, "{period} vs {exact}");
    // 線形化の見積りとは 5 % 以内
    assert!((period - 1920.0).abs() < 0.05 * 1920.0, "{period}");

    // デューティ: 全体（初期の立ち上がり込み）は厳密 0.631 の ±0.02、線形化 0.625 の ±0.03
    let h = &out.heaters[0];
    let duty_exact = 2.0_f64.ln() / (1.5_f64.ln() + 2.0_f64.ln());
    assert!(
        (h.duty_fraction.value() - duty_exact).abs() < 0.02,
        "{}",
        h.duty_fraction.value()
    );
    assert!((h.duty_fraction.value() - 0.625).abs() < 0.03);
    // 最後の 1 周（周期の整数倍でない）は ±0.07
    assert!((h.duty_fraction_last_orbit.value() - duty_exact).abs() < 0.07);
    // 平均電力 = デューティ × 容量。時系列の平均と一致する（刻みとサンプルの間隔が同じで、サンプルの始めの状態が刻みの間の状態）
    assert!((h.mean_power.value() - h.duty_fraction.value() * 20.0).abs() < 1e-12);
    let mean_series = p[..p.len() - 1].iter().sum::<f64>() / (p.len() - 1) as f64;
    assert!(
        (mean_series - h.mean_power.value()).abs() < 1e-9,
        "{mean_series}"
    );
    // 切り替えは 1 周期に 2 回
    let expected = 2.0 * 40_000.0 / exact;
    assert!(
        (f64::from(h.switchings) - expected).abs() <= 3.0,
        "{}",
        h.switchings
    );
    // 最長のオン時間は 260 K から 280 K までの最初のオン: τ ln((290 − 260)/(290 − 280)) = 1800 ln 3 ≈ 1977.5 s
    let first_on = TAU_S * 3.0_f64.ln();
    assert!((h.max_on_duration.value() - first_on).abs() < 30.0);
    assert!(!h.saturated);
    assert!(out.checks.iter().any(|c| c.key == "heater:H"));
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
    assert!((last - (250.0 + 25.0 * (-20_000.0_f64 / TAU_S).exp())).abs() < 1e-3);

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
