//! `thermal.transient` のテスト（1 次遅れの解析解・軌道上の周期解）。

use super::*;
use crate::thermal::network::{ThermalConductiveLink, tests::node};
use crate::thermal::orbit_heat::fixtures::{cube, leo_timeline};

/// 1 次遅れ: C = 900 J/K の節点を G = 1 W/K で 300 K の境界につなぎ、350 K から冷やす。
/// T(t) = 300 + 50·exp(−t/900)。外面なしなので入熱は 0。
#[test]
fn first_order_cooling() {
    let input = ThermalTransientInput {
        network: ThermalNetwork {
            mounts: None,
            nodes: vec![node("sink", Some(300.0)), node("box", None)],
            conductive: vec![ThermalConductiveLink {
                a: "sink".to_owned(),
                b: "box".to_owned(),
                conductance: Qty::from_raw(1.0),
            }],
            radiative: vec![],
            enclosure: None,
            surfaces: vec![],
            shadowing: None,
        },
        timeline: leo_timeline(1800.0, 60.0),
        environment: ThermalEnvironment::default(),
        phases: vec![],
        heaters: vec![],
        initial: ThermalInitial::Uniform {
            temperature: Qty::from_raw(350.0),
        },
        max_step: None,
        rel_tol: None,
        abs_tol: None,
        margin: ThermalMarginPolicy::default(),
    };
    let out = ThermalTransient::run(&input).unwrap();
    let ch = &out.series.channels[1];
    for (t, v) in out.series.t.iter().zip(&ch.values) {
        let expected = 300.0 + 50.0 * (-t / 900.0).exp();
        assert!((v - expected).abs() < 1e-8, "t={t}: {v} vs {expected}");
    }
    assert!((out.nodes[1].max.value() - 350.0).abs() < 1e-9);
    assert_eq!(out.nodes[0].min.value(), 300.0);
}

#[test]
fn phases_cycle() {
    let net = Compiled::new(&ThermalNetwork {
        mounts: None,
        nodes: vec![node("a", None)],
        conductive: vec![],
        radiative: vec![],
        enclosure: None,
        surfaces: vec![],
        shadowing: None,
    })
    .unwrap();
    let load = |w: f64| {
        vec![ThermalLoad {
            count: 1,
            node: "a".to_owned(),
            power: Qty::from_raw(w),
            rf_output: None,
            heat_fraction: Some(1.0),
        }]
    };
    let p = Phases::new(
        &net,
        &[
            ThermalHeatPhase {
                mode_loads: None,
                label: "A".to_owned(),
                duration: Some(Qty::from_raw(10.0)),
                loads: load(1.0),
            },
            ThermalHeatPhase {
                mode_loads: None,
                label: "B".to_owned(),
                duration: Some(Qty::from_raw(30.0)),
                loads: load(5.0),
            },
        ],
        &[],
    )
    .unwrap();
    assert_eq!(p.at(5.0), &[1.0]);
    assert_eq!(p.at(15.0), &[5.0]);
    assert_eq!(p.at(45.0), &[1.0]);
    assert!((p.mean()[0] - 4.0).abs() < 1e-12);
}

/// 熱容量の大きい立方体を軌道平均の定常解から始めると、1 周回の平均は定常解に近く、
/// 日照・日陰で定常解をはさんで上下し、1 周回後にほぼ元の温度に戻る。
#[test]
fn orbit_average_start_is_near_periodic() {
    let mut tl = leo_timeline(1.0, 30.0);
    let period = tl.orbit().cycle().length_s();
    tl.sampling.span = Qty::from_raw(period);
    let loads = vec![ThermalLoad {
        count: 1,
        node: "cube".to_owned(),
        power: Qty::from_raw(2.0),
        rf_output: None,
        heat_fraction: Some(1.0),
    }];
    let input = ThermalTransientInput {
        network: cube(5.0),
        timeline: tl.clone(),
        environment: ThermalEnvironment::default(),
        heaters: vec![],
        phases: vec![ThermalHeatPhase {
            mode_loads: None,
            label: "NOM".to_owned(),
            duration: Some(Qty::from_raw(1000.0)),
            loads: loads.clone(),
        }],
        initial: ThermalInitial::OrbitAverageSteady,
        max_step: None,
        rel_tol: None,
        abs_tol: None,
        margin: ThermalMarginPolicy::default(),
    };
    let out = ThermalTransient::run(&input).unwrap();
    let steady = crate::thermal::steady::ThermalSteady::run(&crate::thermal::ThermalSteadyInput {
        network: cube(5.0),
        cases: vec![crate::thermal::steady::ThermalSteadyCase {
            mode_loads: None,
            heaters: vec![],
            id: "avg".to_owned(),
            label: String::new(),
            environment: ThermalEnvironment::default(),
            exposure: crate::thermal::ThermalExposure::OrbitAverage { timeline: tl },
            loads,
        }],
        margin: ThermalMarginPolicy::default(),
    })
    .unwrap();
    let t_ss = steady.cases[0].nodes[0].temperature.value();
    let r = &out.nodes[0];
    let values = &out.series.channels[0].values;
    assert!((values[0] - t_ss).abs() < 1e-6);
    assert!(
        r.min.value() < t_ss && t_ss < r.max.value(),
        "{r:?} vs {t_ss}"
    );
    assert!(
        (r.mean.value() - t_ss).abs() < 2.0,
        "{} vs {t_ss}",
        r.mean.value()
    );
    assert!((values[values.len() - 1] - values[0]).abs() < 2.0);
    // 入熱のチャンネルは日陰で地球赤外だけになる（0 より大きく、日照時より小さい）。
    let absorbed = &out.series.channels[1].values;
    assert!(absorbed.iter().all(|q| *q > 0.0));
}

/// 区間の長さを書かなければ既定（1 日）。区間が 1 つなら発熱は一定なので、長さを書いたときと結果が同じ。
#[test]
fn single_phase_duration_defaults_without_changing_the_result() {
    let run = |duration: Option<Qty<Time>>| {
        let loads = vec![ThermalLoad {
            count: 1,
            node: "cube".to_owned(),
            power: Qty::from_raw(2.0),
            rf_output: None,
            heat_fraction: Some(1.0),
        }];
        ThermalTransient::run(&ThermalTransientInput {
            network: cube(5.0),
            timeline: leo_timeline(1.0, 30.0),
            environment: ThermalEnvironment::default(),
            heaters: vec![],
            phases: vec![ThermalHeatPhase {
                mode_loads: None,
                label: "NOM".to_owned(),
                duration,
                loads,
            }],
            initial: ThermalInitial::OrbitAverageSteady,
            max_step: None,
            rel_tol: None,
            abs_tol: None,
            margin: ThermalMarginPolicy::default(),
        })
        .unwrap()
    };
    let omitted = run(None);
    let written = run(Some(Qty::from_raw(DEFAULT_PHASE_DURATION_S)));
    assert_eq!(DEFAULT_PHASE_DURATION_S, 86_400.0);
    assert_eq!(omitted.nodes[0].max, written.nodes[0].max);
    assert_eq!(omitted.nodes[0].min, written.nodes[0].min);
}

/// 発熱の区間の切り替わりの時刻（周期の中の区間の終わり + 周期の整数倍）を、区間 `(a, b)` の中の分だけ返す。
#[test]
fn phase_boundaries_inside_an_interval() {
    let net = Compiled::new(&ThermalNetwork {
        mounts: None,
        nodes: vec![node("a", None)],
        conductive: vec![],
        radiative: vec![],
        enclosure: None,
        surfaces: vec![],
        shadowing: None,
    })
    .unwrap();
    let phase = |label: &str, duration: f64| ThermalHeatPhase {
        mode_loads: None,
        label: label.to_owned(),
        duration: Some(Qty::from_raw(duration)),
        loads: vec![],
    };
    let p = Phases::new(&net, &[phase("A", 10.0), phase("B", 30.0)], &[]).unwrap();
    // 周期 40 s: 切り替わりは 10, 40, 50, 80, 90, …
    assert_eq!(p.boundaries(0.0, 45.0), vec![10.0, 40.0]);
    assert_eq!(p.boundaries(40.0, 90.0), vec![50.0, 80.0]);
    assert!(p.boundaries(11.0, 39.0).is_empty());
    // 区間が無ければ切り替わりも無い
    let none = Phases::new(&net, &[], &[]).unwrap();
    assert!(none.boundaries(0.0, 1e6).is_empty());
}

/// 積分の設定と費用の報告: 許容誤差の既定値と入力の上書きが報告に出て、最大刻みは刻みの上限になる。
/// 値の検査（負の許容誤差・0 の最大刻み）は入力の誤りとして止まる。
#[test]
fn integration_settings_and_cost_are_reported() {
    let base = || ThermalTransientInput {
        network: cube(5.0),
        timeline: leo_timeline(600.0, 60.0),
        environment: ThermalEnvironment::default(),
        phases: vec![],
        heaters: vec![],
        initial: ThermalInitial::Uniform {
            temperature: Qty::from_raw(290.0),
        },
        max_step: None,
        rel_tol: None,
        abs_tol: None,
        margin: ThermalMarginPolicy::default(),
    };
    let out = ThermalTransient::run(&base()).unwrap();
    let report = &out.integration;
    assert_eq!(report.rel_tol, DEFAULT_REL_TOL);
    assert_eq!(report.abs_tol.value(), DEFAULT_ABS_TOL_K);
    assert!(report.max_step.is_none() && report.events == 0);
    assert!(report.smooth_segments >= 10 && report.accepted_steps >= report.smooth_segments);
    assert!(report.rhs_evaluations >= 12 * report.accepted_steps);

    let mut tight = base();
    tight.rel_tol = Some(1e-11);
    tight.abs_tol = Some(Qty::from_raw(1e-9));
    tight.max_step = Some(Qty::from_raw(5.0));
    let out = ThermalTransient::run(&tight).unwrap();
    assert_eq!(out.integration.rel_tol, 1e-11);
    assert_eq!(out.integration.abs_tol.value(), 1e-9);
    // 600 s を 5 s 以下の刻みで進む
    assert!(out.integration.accepted_steps >= 120);

    let mut bad = base();
    bad.rel_tol = Some(-1.0);
    assert!(ThermalTransient::run(&bad).is_err());
    let mut bad = base();
    bad.max_step = Some(Qty::from_raw(0.0));
    assert!(ThermalTransient::run(&bad).is_err());
}
