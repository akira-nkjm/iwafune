//! `thermal.steady` のヒータ（出力の組み立て・判定・ケースをまたぐ最悪）のテスト。物理は `network::heater` のテスト。

use super::*;
use crate::modes::{ModeDef, Redundancy, StateAssignment};
use crate::thermal::network::{ThermalSurface, tests::node};
use sat_astro::constants::SIGMA_SB;
use sat_astro::math::Vec3;

fn heater(power: f64) -> ThermalHeater {
    ThermalHeater {
        id: "H".to_owned(),
        node: "n".to_owned(),
        on_below: Qty::from_raw(280.0),
        off_above: Qty::from_raw(285.0),
        power: Qty::from_raw(power),
        count: 1,
        redundancy: Redundancy::None,
        states: Vec::new(),
    }
}

/// 面積 0.1 m²・ε 0.8 で宇宙 0 K へ放射する 1 節点。太陽・中心天体の入熱は 0（日陰で、中心天体を見ない）。
fn case(id: &str, other_w: f64, heaters: Vec<ThermalHeater>) -> ThermalSteadyCase {
    ThermalSteadyCase {
        id: id.to_owned(),
        label: id.to_owned(),
        environment: ThermalEnvironment {
            space_temperature: Some(Qty::from_raw(0.0)),
            ..ThermalEnvironment::default()
        },
        exposure: ThermalExposure::Fixed {
            sun_dir_body: None,
            nadir_body: None,
            altitude: Qty::from_raw(500.0e3),
            central: sat_astro::ephem::CentralBody::Earth,
        },
        loads: vec![ThermalLoad {
            count: 1,
            node: "n".to_owned(),
            power: Qty::from_raw(other_w),
            rf_output: None,
            heat_fraction: Some(1.0),
        }],
        mode_loads: None,
        heaters,
    }
}

fn run(cases: Vec<ThermalSteadyCase>) -> ThermalSteadyOutput {
    ThermalSteady::run(&ThermalSteadyInput {
        network: ThermalNetwork {
            plates: Vec::new(),
            mounts: None,
            nodes: vec![node("n", None)],
            conductive: vec![],
            radiative: vec![],
            enclosure: None,
            surfaces: vec![ThermalSurface {
                id: "s".to_owned(),
                node: "n".to_owned(),
                area: Qty::from_raw(0.1),
                normal: Vec3::X,
                count: 1,
                absorptivity: Some(0.5),
                emissivity: Some(0.8),
                coatings: vec![],
            }],
        },
        cases,
        margin: ThermalMarginPolicy::default(),
    })
    .unwrap()
}

fn heater_check(o: &ThermalSteadyOutput) -> &Check {
    o.checks.iter().find(|c| c.key == "heater:H").unwrap()
}

/// 手計算: 他の発熱 3 W、ε σ A T⁴（T = 280 K）= 27.88 W → 電力 24.88 W、容量 30 W でデューティ 0.829（> 0.8 で警告）、
/// 容量 40 W ならデューティ 0.622（OK）。節点の内部発熱は他の発熱 + ヒータの電力（二重に数えない）= 27.88 W。
#[test]
fn regulating_heater_duty_and_heat_balance_by_hand() {
    let required = 0.8 * SIGMA_SB * 0.1 * 280.0_f64.powi(4) - 3.0;
    let out = run(vec![case("cold", 3.0, vec![heater(30.0)])]);
    let c = &out.cases[0];
    let h = &c.heaters[0];
    assert_eq!(h.state, crate::thermal::ThermalHeaterState::Regulating);
    assert!((h.power.value() - required).abs() < 1e-6);
    assert!((h.duty_fraction.value() - required / 30.0).abs() < 1e-9);
    assert!((c.nodes[0].temperature.value() - 280.0).abs() < 1e-9);
    assert!((c.nodes[0].internal.value() - (3.0 + required)).abs() < 1e-6);
    assert_eq!(heater_check(&out).verdict, Verdict::Warning);

    let out = run(vec![case("cold", 3.0, vec![heater(40.0)])]);
    assert_eq!(heater_check(&out).verdict, Verdict::Ok);
}

/// 熱いケース（他の発熱 40 W → オフ）と冷たいケース（他の発熱 1 W・容量 10 W → 飽和）では、判定は飽和のケースで違反になる。
#[test]
fn check_takes_the_worst_case() {
    let out = run(vec![
        case("hot", 40.0, vec![heater(10.0)]),
        case("cold", 1.0, vec![heater(10.0)]),
    ]);
    assert_eq!(
        out.cases[0].heaters[0].state,
        crate::thermal::ThermalHeaterState::Off
    );
    assert_eq!(out.cases[0].heaters[0].power.value(), 0.0);
    let check = heater_check(&out);
    assert_eq!(check.verdict, Verdict::Violation);
    assert!(check.message.contains("Case cold"), "{}", check.message);
    assert_eq!(out.checks.iter().filter(|c| c.key == "heater:H").count(), 1);
}

/// ケースのモードでヒータが `off` なら働かない（0 W）。
#[test]
fn heater_is_off_in_a_mode_that_disables_it() {
    let mut c = case("safe", 3.0, vec![heater(30.0)]);
    c.mode_loads = Some(ThermalModeLoads {
        mode: ModeDef {
            id: "SAFE".into(),
            label: "SAFE".into(),
            assignments: vec![StateAssignment {
                part: "H".into(),
                state: "off".into(),
                duty: Default::default(),
                active: None,
                spare_state: None,
            }],
            eclipse: Vec::new(),
        },
        parts: vec![],
    });
    let out = run(vec![c]);
    assert_eq!(
        out.cases[0].heaters[0].state,
        crate::thermal::ThermalHeaterState::Disabled
    );
    let t_free = (3.0 / (0.8 * SIGMA_SB * 0.1)).powf(0.25);
    assert!((out.cases[0].nodes[0].temperature.value() - t_free).abs() < 1e-6);
    assert_eq!(heater_check(&out).verdict, Verdict::Ok);
}
