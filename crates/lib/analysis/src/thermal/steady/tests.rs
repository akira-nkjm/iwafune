//! `thermal.steady` のテスト（板と箱の手計算・3 種の範囲と不確かさ・軌道平均の立方体）。

use super::*;
use crate::thermal::network::{ThermalConductiveLink, ThermalSurface, tests::node};
use crate::thermal::range::ThermalTempRange;
use sat_astro::constants::SIGMA_SB;
use sat_astro::math::Vec3;
use sat_units::typed::Fraction;

fn input() -> ThermalSteadyInput {
    let mut panel = node("panel", None);
    panel.parts[0].ranges.operating = Some(ThermalTempRange {
        min: Qty::from_raw(233.0),
        max: Qty::from_raw(350.0),
    });
    ThermalSteadyInput {
        network: ThermalNetwork {
            mounts: None,
            nodes: vec![panel, node("box", None)],
            conductive: vec![ThermalConductiveLink {
                a: "panel".to_owned(),
                b: "box".to_owned(),
                conductance: Qty::from_raw(1.0),
            }],
            radiative: vec![],
            surfaces: vec![ThermalSurface {
                id: "px".to_owned(),
                node: "panel".to_owned(),
                area: Qty::from_raw(0.1),
                normal: Vec3::X,
                count: 1,
                absorptivity: Some(0.5),
                emissivity: Some(0.8),
                coatings: vec![],
            }],
        },
        cases: vec![ThermalSteadyCase {
            mode_loads: None,
            id: "hot".to_owned(),
            label: "高温".to_owned(),
            environment: ThermalEnvironment {
                solar_flux: Some(Qty::from_raw(1400.0)),
                space_temperature: Some(Qty::from_raw(0.0)),
                ..ThermalEnvironment::default()
            },
            exposure: ThermalExposure::Fixed {
                sun_dir_body: Some(Vec3::X),
                nadir_body: None,
                altitude: Qty::from_raw(500.0e3),
            },
            loads: vec![ThermalLoad {
                count: 1,
                node: "box".to_owned(),
                power: Qty::from_raw(5.0),
                rf_output: None,
                heat_fraction: Some(1.0),
            }],
        }],
        margin: ThermalMarginPolicy::default(),
    }
}

/// 手計算: 吸収 0.5·1400·0.1 = 70 W ＋ 内部 5 W を 0.8σ·0.1 T⁴ で捨てる → T_panel = (75/(0.08σ))^¼、
/// 箱は 5 W を G = 1 W/K で流すので T_box = T_panel + 5 K。
#[test]
fn panel_and_box_hand_calculation() {
    let out = ThermalSteady::run(&input()).unwrap();
    let t_panel = (75.0 / (0.08 * SIGMA_SB)).powf(0.25);
    let nodes = &out.cases[0].nodes;
    assert!((nodes[0].temperature.value() - t_panel).abs() < 1e-6);
    assert!((nodes[1].temperature.value() - (t_panel + 5.0)).abs() < 1e-6);
    assert!((nodes[0].absorbed.value() - 70.0).abs() < 1e-9);
    assert_eq!(out.envelope[0].max_case, "hot");
    assert_eq!(out.checks.len(), 1);
    // T_panel ≈ 358.6 K > 350 K
    assert_eq!(out.checks[0].verdict, Verdict::Violation);
}

/// 同じ板（T_panel ≈ 358.6 K）に保管範囲 223〜373 K を足す: 保管は上端まで 14.4 K で Ok、動作（〜350 K）は違反。
/// 不確かさ 10 K を入れると保管の余裕は 4.4 K（< 5 K）で警告。節点の判定は最も悪い違反、余裕は不確かさの前の値。
#[test]
fn storage_range_and_uncertainty_by_hand() {
    let mut i = input();
    i.network.nodes[0].parts[0].ranges.storage = Some(ThermalTempRange {
        min: Qty::from_raw(223.0),
        max: Qty::from_raw(373.0),
    });
    let verdicts = |i: &ThermalSteadyInput| -> Vec<(String, Verdict)> {
        let out = ThermalSteady::run(i).unwrap();
        out.checks.into_iter().map(|c| (c.key, c.verdict)).collect()
    };
    let key = |k: &str| format!("range:panel:{k}");
    assert_eq!(
        verdicts(&i),
        [
            (key("storage"), Verdict::Ok),
            (key("operating"), Verdict::Violation)
        ]
    );
    i.margin.uncertainty = Some(Qty::from_raw(10.0));
    assert_eq!(verdicts(&i)[0], (key("storage"), Verdict::Warning));
    let out = ThermalSteady::run(&i).unwrap();
    let t_panel = (75.0 / (0.08 * SIGMA_SB)).powf(0.25);
    let storage = &out.cases[0].nodes[0].ranges[0];
    assert!((storage.margin_high.value() - (373.0 - t_panel)).abs() < 1e-6);
    assert_eq!(out.cases[0].nodes[0].verdict, Verdict::Violation);
    assert_eq!(out.margin.uncertainty.value(), 10.0);
    assert_eq!(out.margin.warning_band.value(), 5.0);
}

/// 太陽光だけ（アルベド・地球赤外 0）の慣性固定の立方体: 平均吸収 = α S A Σ|sᵢ| × 日照率 を
/// 時系列から直接平均した値と、定常温度 T = (Q / (ε σ 6A))^¼ が一致する。
/// 地球を入れた高温ケースは低温ケースより熱く、地球の入熱が無いケースが最も冷たい。
#[test]
fn orbit_average_cube() {
    use crate::thermal::orbit_heat::fixtures::{cube, leo_timeline};
    let mut tl = leo_timeline(1.0, 20.0);
    let period = tl.orbit().period_s();
    tl.sampling.span = Qty::from_raw(period);
    let samples = sat_astro::timeline::build(&tl).unwrap();
    let t: Vec<f64> = samples.iter().map(|s| s.t_s).collect();
    // 1 周回（両端を含む）で台形則の時間平均を取る。
    let r = crate::spec::orbit_ranges(&t, period)[0].clone();
    let idx = r.start..=r.end.min(t.len() - 1);
    let q = |i: usize| {
        let d = samples[i].sun_dir_body();
        0.6 * 1361.0 * 0.01 * (d.x.abs() + d.y.abs() + d.z.abs()) * samples[i].sunlit
    };
    let (first, last) = (*idx.start(), *idx.end());
    let q_expected: f64 = (first..last)
        .map(|i| 0.5 * (q(i) + q(i + 1)) * (t[i + 1] - t[i]))
        .sum::<f64>()
        / (t[last] - t[first]);
    let env = |flux: f64, albedo: f64, ir: f64| ThermalEnvironment {
        solar_flux: Some(Qty::from_raw(flux)),
        albedo: Some(Fraction::new(albedo)),
        earth_ir: Some(Qty::from_raw(ir)),
        space_temperature: Some(Qty::from_raw(0.0)),
        preset: None,
    };
    let case = |id: &str, e: ThermalEnvironment| ThermalSteadyCase {
        mode_loads: None,
        id: id.to_owned(),
        label: String::new(),
        environment: e,
        exposure: ThermalExposure::OrbitAverage {
            timeline: tl.clone(),
        },
        loads: vec![],
    };
    let out = ThermalSteady::run(&ThermalSteadyInput {
        network: cube(1.0),
        cases: vec![
            case("sun", env(1361.0, 0.0, 0.0)),
            case("hot", env(1414.0, 0.35, 258.0)),
            case("cold", env(1322.0, 0.25, 217.0)),
        ],
        margin: ThermalMarginPolicy::default(),
    })
    .unwrap();
    let sun = &out.cases[0].nodes[0];
    assert!((sun.absorbed.value() - q_expected).abs() < 1e-9 * q_expected);
    let t_expected = (q_expected / (0.8 * SIGMA_SB * 0.06)).powf(0.25);
    assert!((sun.temperature.value() - t_expected).abs() < 1e-6);
    let (hot, cold) = (&out.cases[1].nodes[0], &out.cases[2].nodes[0]);
    assert!(hot.temperature.value() > cold.temperature.value());
    assert_eq!(out.envelope[0].max_case, "hot");
    // 地球の入熱が無い「sun」が最も冷たい。
    assert_eq!(out.envelope[0].min_case, "sun");
}
