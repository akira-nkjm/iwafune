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
        },
        timeline: leo_timeline(1800.0, 60.0),
        environment: ThermalEnvironment::default(),
        phases: vec![],
        initial: ThermalInitial::Uniform {
            temperature: Qty::from_raw(350.0),
        },
        max_step: None,
        margin: ThermalMarginPolicy::default(),
    };
    let out = ThermalTransient::run(&input).unwrap();
    let ch = &out.series.channels[1];
    for (t, v) in out.series.t.iter().zip(&ch.values) {
        let expected = 300.0 + 50.0 * (-t / 900.0).exp();
        assert!((v - expected).abs() < 1e-4, "t={t}: {v} vs {expected}");
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
    let period = tl.orbit().period_s();
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
        phases: vec![ThermalHeatPhase {
            mode_loads: None,
            label: "NOM".to_owned(),
            duration: Some(Qty::from_raw(1000.0)),
            loads: loads.clone(),
        }],
        initial: ThermalInitial::OrbitAverageSteady,
        max_step: None,
        margin: ThermalMarginPolicy::default(),
    };
    let out = ThermalTransient::run(&input).unwrap();
    let steady = crate::thermal::steady::ThermalSteady::run(&crate::thermal::ThermalSteadyInput {
        network: cube(5.0),
        cases: vec![crate::thermal::steady::ThermalSteadyCase {
            mode_loads: None,
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
            phases: vec![ThermalHeatPhase {
                mode_loads: None,
                label: "NOM".to_owned(),
                duration,
                loads,
            }],
            initial: ThermalInitial::OrbitAverageSteady,
            max_step: None,
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
