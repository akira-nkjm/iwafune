//! 熱（定常）（`thermal.steady`）。ケース（高温・低温など）ごとに節点温度の定常解を求め、動作温度範囲と比べる。
//! 解法は [`crate::thermal::network`]、入熱は [`crate::thermal::orbit_heat`]。

use crate::spec::quantity_text;
use crate::spec::{Analysis, AnalysisError, AnalysisKind, Check, System, Verdict, margin_verdict};
use crate::thermal::network::{
    Compiled, ThermalLoad, ThermalModeLoads, ThermalNetwork, ThermalTempRange,
};
use crate::thermal::orbit_heat::{ThermalEnvironment, ThermalExposure, exposure_absorbed};
use sat_units::typed::{Power, Qty, Temperature};
use serde::{Deserialize, Serialize};

/// Newton 法の初期値 [K]。
const INITIAL_GUESS_K: f64 = 290.0;

/// 定常のケース。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalSteadyCase {
    /// 例: `"hot"`、`"cold"`
    pub id: String,
    pub label: String,
    pub environment: ThermalEnvironment,
    pub exposure: ThermalExposure,
    /// 内部発熱の源（消費電力と熱になる割合。同じ節点の源は足し合わせる）
    #[serde(default)]
    pub loads: Vec<ThermalLoad>,
    /// モードの発熱（不足入力は checks に警告）。
    #[serde(default)]
    pub mode_loads: Option<ThermalModeLoads>,
}

/// 熱（定常）の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalSteadyInput {
    pub network: ThermalNetwork,
    pub cases: Vec<ThermalSteadyCase>,
}

/// 1 ケースの 1 節点の結果（熱マップの 1 セル）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalSteadyNodeResult {
    pub node: String,
    pub temperature: Qty<Temperature>,
    /// 外部入熱（太陽・アルベド・地球赤外の吸収）
    pub absorbed: Qty<Power>,
    /// 内部発熱（消費電力 × heat_fraction の合計）
    pub internal: Qty<Power>,
    /// 動作範囲の下限からの余裕 T − min（範囲が無ければ `None`）
    pub margin_low: Option<Qty<Temperature>>,
    /// 動作範囲の上限までの余裕 max − T
    pub margin_high: Option<Qty<Temperature>>,
    pub verdict: Verdict,
}

/// 1 ケースの結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalSteadyCaseResult {
    pub id: String,
    pub label: String,
    /// 入力の節点の順
    pub nodes: Vec<ThermalSteadyNodeResult>,
    pub iterations: u32,
    /// 収束時の熱収支の残差の最大
    pub residual: Qty<Power>,
}

/// 全ケースを通した節点の最高・最低。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalSteadyEnvelope {
    pub node: String,
    pub min: Qty<Temperature>,
    pub min_case: String,
    pub max: Qty<Temperature>,
    pub max_case: String,
    pub operating: Option<ThermalTempRange>,
    pub verdict: Verdict,
}

/// 熱（定常）の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalSteadyOutput {
    pub cases: Vec<ThermalSteadyCaseResult>,
    /// 入力の節点の順
    pub envelope: Vec<ThermalSteadyEnvelope>,
    /// 動作範囲の端からこの幅以内を警告にした（[`RANGE_WARNING_BAND_K`]）
    pub range_warning_band: Qty<Temperature>,
    /// 動作範囲を持つ節点ごとの判定（key は `range:<node>`）
    pub checks: Vec<Check>,
}

/// 熱（定常）。
pub struct ThermalSteady;

impl Analysis for ThermalSteady {
    const ID: &'static str = "thermal.steady";
    const TITLE: &'static str = "熱（定常）";
    const SYSTEM: System = System::Thermal;
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str = "多節点モデルの定常温度（最高・最低ケース）";
    const TITLE_EN: &'static str = "Thermal (steady state)";
    const DESCRIPTION_EN: &'static str =
        "Steady-state node temperatures of the multi-node model for hot and cold cases";
    type Input = ThermalSteadyInput;
    type Output = ThermalSteadyOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        if input.cases.is_empty() {
            return Err(AnalysisError::InvalidInput("ケースがありません".to_owned()));
        }
        let net = Compiled::new(&input.network)?;
        let mut warnings = Vec::new();
        let cases = input
            .cases
            .iter()
            .map(|c| {
                let mut case = c.clone();
                if let Some(mode) = &c.mode_loads {
                    let (loads, checks) = mode.evaluate()?;
                    case.loads.extend(loads);
                    warnings.extend(checks.into_iter().map(|mut check| {
                        check.key = format!("case:{}:{}", c.id, check.key);
                        check
                    }));
                }
                solve_case(&net, &input.network, &case)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let envelope = envelope(&input.network, &cases);
        let mut checks: Vec<Check> = envelope
            .iter()
            .filter_map(|e| range_check(&e.node, e.min.value(), e.max.value(), e.operating))
            .collect();
        checks.extend(warnings);
        Ok(ThermalSteadyOutput {
            cases,
            envelope,
            range_warning_band: Qty::from_raw(RANGE_WARNING_BAND_K),
            checks,
        })
    }
}

fn solve_case(
    net: &Compiled,
    network: &ThermalNetwork,
    case: &ThermalSteadyCase,
) -> Result<ThermalSteadyCaseResult, AnalysisError> {
    let absorbed = exposure_absorbed(net, &case.environment, &case.exposure)?;
    let internal = net.loads(&case.loads)?;
    let q: Vec<f64> = absorbed.iter().zip(&internal).map(|(a, b)| a + b).collect();
    let sol = net.solve_steady(
        &q,
        case.environment.space_temperature.value(),
        INITIAL_GUESS_K,
    )?;
    let nodes = network
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let t = sol.temps[i];
            let op = n.operating();
            ThermalSteadyNodeResult {
                node: n.id.clone(),
                temperature: Qty::from_raw(t),
                absorbed: Qty::from_raw(absorbed[i]),
                internal: Qty::from_raw(internal[i]),
                margin_low: op.map(|r| Qty::from_raw(t - r.min.value())),
                margin_high: op.map(|r| Qty::from_raw(r.max.value() - t)),
                verdict: range_verdict(t, t, op),
            }
        })
        .collect();
    Ok(ThermalSteadyCaseResult {
        id: case.id.clone(),
        label: case.label.clone(),
        nodes,
        iterations: sol.iterations,
        residual: Qty::from_raw(sol.residual_w),
    })
}

fn envelope(
    network: &ThermalNetwork,
    cases: &[ThermalSteadyCaseResult],
) -> Vec<ThermalSteadyEnvelope> {
    network
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let temp = |c: &ThermalSteadyCaseResult| c.nodes[i].temperature.value();
            let lo = cases.iter().min_by(|a, b| temp(a).total_cmp(&temp(b)));
            let hi = cases.iter().max_by(|a, b| temp(a).total_cmp(&temp(b)));
            let (lo, hi) = (lo.map_or(f64::NAN, temp), hi.map_or(f64::NAN, temp));
            let case_id = |t: f64| {
                cases
                    .iter()
                    .find(|c| temp(c) == t)
                    .map_or(String::new(), |c| c.id.clone())
            };
            ThermalSteadyEnvelope {
                node: n.id.clone(),
                min: Qty::from_raw(lo),
                min_case: case_id(lo),
                max: Qty::from_raw(hi),
                max_case: case_id(hi),
                operating: n.operating(),
                verdict: range_verdict(lo, hi, n.operating()),
            }
        })
        .collect()
}

/// 動作範囲の端からこの幅 [K] 以内なら警告にする。出力の `range_warning_band` にも出す（Web はこれを使う）。
pub const RANGE_WARNING_BAND_K: f64 = 5.0;

/// 最低 `lo`・最高 `hi` [K] を動作範囲と比べる（端から [`RANGE_WARNING_BAND_K`] 以内は警告）。範囲が無ければ Ok。
pub(crate) fn range_verdict(lo: f64, hi: f64, range: Option<ThermalTempRange>) -> Verdict {
    let Some(r) = range else {
        return Verdict::Ok;
    };
    let margin = (lo - r.min.value()).min(r.max.value() - hi);
    margin_verdict(margin, RANGE_WARNING_BAND_K)
}

/// 動作範囲との比較の判定（範囲が無ければ `None`）。温度の表示は sat-units に委譲する。
pub(crate) fn range_check(
    node: &str,
    lo: f64,
    hi: f64,
    range: Option<ThermalTempRange>,
) -> Option<Check> {
    let r = range?;
    Some(Check::new(
        &format!("range:{node}"),
        &format!("{node} の温度"),
        range_verdict(lo, hi, range),
        format!(
            "{}〜{}（動作範囲 {}〜{}）",
            quantity_text(lo, "Temperature"),
            quantity_text(hi, "Temperature"),
            quantity_text(r.min.value(), "Temperature"),
            quantity_text(r.max.value(), "Temperature")
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thermal::network::{ThermalConductiveLink, ThermalSurface, tests::node};
    use sat_astro::constants::SIGMA_SB;
    use sat_astro::math::Vec3;

    fn input() -> ThermalSteadyInput {
        let mut panel = node("panel", None);
        panel.parts[0].operating = Some(ThermalTempRange {
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
                    absorptivity: 0.5,
                    emissivity: 0.8,
                }],
            },
            cases: vec![ThermalSteadyCase {
                mode_loads: None,
                id: "hot".to_owned(),
                label: "高温".to_owned(),
                environment: ThermalEnvironment {
                    solar_flux: Some(Qty::from_raw(1400.0)),
                    space_temperature: Qty::from_raw(0.0),
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
                    heat_fraction: Some(1.0),
                }],
            }],
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

    /// 太陽光だけ（アルベド・地球赤外 0）の慣性固定の立方体: 平均吸収 = α S A Σ|sᵢ| × 日照率 を
    /// 時系列から直接平均した値と、定常温度 T = (Q / (ε σ 6A))^¼ が一致する。
    /// 地球を入れた高温ケースは低温ケースより熱く、地球の入熱が無いケースが最も冷たい。
    #[test]
    fn orbit_average_cube() {
        use crate::thermal::orbit_heat::fixtures::{cube, leo_timeline};
        let mut tl = leo_timeline(1.0, 20.0);
        let period = tl.orbit.period_s();
        tl.span = Qty::from_raw(period);
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
            albedo,
            earth_ir: Qty::from_raw(ir),
            space_temperature: Qty::from_raw(0.0),
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
}
