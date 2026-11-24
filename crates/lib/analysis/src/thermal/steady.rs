//! 熱（定常）（`thermal.steady`）。ケース（高温・低温など）ごとに節点温度の定常解を求め、保管・動作・性能の温度範囲と比べる。
//! 解法は [`crate::thermal::network`]、入熱は [`crate::thermal::orbit_heat`]。

use crate::spec::{Analysis, AnalysisArea, AnalysisError, AnalysisKind, Check, System, Verdict};
use crate::thermal::heater::{ThermalHeaterSteady, steady_checks};
use crate::thermal::network::{
    Compiled, INITIAL_GUESS_K, ThermalCouplings, ThermalHeater, ThermalLoad, ThermalModeLoads,
    ThermalNetwork, enabled_flags,
};
use crate::thermal::network::{HeaterSpec, HeaterSteady};
use crate::thermal::orbit_heat::{
    ThermalEnvironment, ThermalEnvironmentUsed, ThermalExposure, exposure_heat, shadow_loss_check,
};
use crate::thermal::range::{
    ThermalMarginPolicy, ThermalMarginUsed, ThermalRangeMargin, envelope_margins, range_checks,
    range_margins, worst_verdict,
};
use sat_units::typed::{Fraction, Power, Qty, Temperature};
use serde::{Deserialize, Serialize};

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
    /// サーモスタット付きのヒータ。サーモスタットが働くかは `mode_loads` のモードで決まる（モードが無ければ働く）。
    /// 定常では、下側の設定温度に節点を保つ電力を出す（定義は `network::heater_steady`）。`loads` や `mode_loads` に同じ部品を入れない
    #[serde(default)]
    pub heaters: Vec<ThermalHeater>,
}

/// 熱（定常）の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalSteadyInput {
    pub network: ThermalNetwork,
    pub cases: Vec<ThermalSteadyCase>,
    /// 解析マージンの方針（書かれていない値は解析の既定値）
    #[serde(default)]
    pub margin: ThermalMarginPolicy,
}

/// 1 ケースの 1 節点の結果（熱マップの 1 セル）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalSteadyNodeResult {
    pub node: String,
    pub temperature: Qty<Temperature>,
    /// 外部入熱（太陽・アルベド・地球赤外の吸収）
    pub absorbed: Qty<Power>,
    /// 内部発熱（`ThermalLoad::dissipation` の合計とヒータの電力）
    pub internal: Qty<Power>,
    /// 温度範囲（保管・動作・性能の順。モードがあれば部品の電源の状態で選ぶ）との比較
    pub ranges: Vec<ThermalRangeMargin>,
    /// 範囲との比較のうち最も悪い判定（範囲が無ければ Ok）
    pub verdict: Verdict,
}

/// 1 ケースの結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalSteadyCaseResult {
    pub id: String,
    pub label: String,
    /// 使った熱環境（入力に無い値は既定値）
    pub environment: ThermalEnvironmentUsed,
    /// 入力の節点の順
    pub nodes: Vec<ThermalSteadyNodeResult>,
    /// ヒータごとの結果（入力の順。ヒータが無ければ空）
    #[serde(default)]
    pub heaters: Vec<ThermalHeaterSteady>,
    pub iterations: u32,
    /// 収束時の熱収支の残差の最大
    pub residual: Qty<Power>,
    /// 外面の日照割合（ほかの部品の影を引いた割合。直達日射にだけ掛ける）の、日照中の標本での最小と平均。影を数えたときだけ付く
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadowing: Option<crate::structure::external_geometry::ShadowingReport>,
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
    /// 種類ごとの全ケースの比較（その範囲を当てたケースのうち、下限・上限の側で余裕の最も小さいもの。
    /// 範囲がケースで変わらなければ全ケースの最低・最高と範囲の比較）
    pub ranges: Vec<ThermalRangeMargin>,
    /// 範囲との比較のうち最も悪い判定（範囲が無ければ Ok）
    pub verdict: Verdict,
}

/// 熱（定常）の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalSteadyOutput {
    pub cases: Vec<ThermalSteadyCaseResult>,
    /// 入力の節点の順
    pub envelope: Vec<ThermalSteadyEnvelope>,
    /// 使った解析マージン（不確かさと警告の幅）
    pub margin: ThermalMarginUsed,
    /// 使った節点間の結合（伝導・放射と出どころ。外殻の放射を求めたときは形態係数）
    #[serde(default)]
    pub couplings: ThermalCouplings,
    /// 温度範囲を持つ節点の種類ごとの判定（key は `range_high|range_low:<node>:<storage|operating|performance>`）と、
    /// ヒータごとの判定（key は `heater:<部品>`。ケースをまたいで最悪のケース）
    pub checks: Vec<Check>,
}

/// 熱（定常）。
pub struct ThermalSteady;

impl Analysis for ThermalSteady {
    const ID: &'static str = "thermal.steady";
    const TITLE: &'static str = "熱（定常）";
    const AREA: AnalysisArea = AnalysisArea::System(System::Thermal);
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str = "多節点モデルの定常温度（最高・最低ケース）";
    const TITLE_EN: &'static str = "Thermal (steady state)";
    const DESCRIPTION_EN: &'static str =
        "Steady-state node temperatures of the multi-node model for hot and cold cases";
    type Input = ThermalSteadyInput;
    type Output = ThermalSteadyOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        if input.cases.is_empty() {
            return Err(AnalysisError::InvalidInput("No cases".to_owned()));
        }
        let net = Compiled::new(&input.network)?;
        let margin = input.margin.used();
        margin.validate()?;
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
                solve_case(&net, &input.network, &case, &margin)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let envelope = envelope(&input.network, &cases, &margin);
        let mut checks: Vec<Check> = envelope
            .iter()
            .flat_map(|e| range_checks(&e.node, &e.ranges, &margin))
            .collect();
        let per_case: Vec<_> = cases
            .iter()
            .map(|c| (c.id.as_str(), c.heaters.as_slice()))
            .collect();
        checks.extend(steady_checks(&per_case, margin.heater_duty_warning.value()));
        checks.extend(warnings);
        // 影の損失の警告は 1 つ（警告が出るケースのうち、割合が最大のケース）
        let worst = cases
            .iter()
            .filter_map(|c| {
                let report = c.shadowing.as_ref()?;
                let check = shadow_loss_check(report, "shadow_loss")?;
                Some((report.lost.as_ref()?.fraction.value(), c.id.as_str(), check))
            })
            .max_by(|a, b| a.0.total_cmp(&b.0));
        checks.extend(worst.map(|(_, id, mut check)| {
            check.message = format!("Case {id}: {}", check.message);
            check
        }));
        Ok(ThermalSteadyOutput {
            cases,
            envelope,
            margin,
            couplings: net.couplings,
            checks,
        })
    }
}

fn solve_case(
    net: &Compiled,
    network: &ThermalNetwork,
    case: &ThermalSteadyCase,
    margin: &ThermalMarginUsed,
) -> Result<ThermalSteadyCaseResult, AnalysisError> {
    let heat = exposure_heat(net, &case.environment, &case.exposure)?;
    let absorbed = heat.q;
    let mut internal = net.loads(&case.loads)?;
    let q: Vec<f64> = absorbed.iter().zip(&internal).map(|(a, b)| a + b).collect();
    let modes = [case.mode_loads.as_ref().map(|l| &l.mode)];
    let specs = net.heaters(&case.heaters)?;
    let enabled = enabled_flags(&case.heaters, modes[0])?;
    let heated = net.solve_steady_heated(
        &q,
        case.environment.space_temperature().value(),
        INITIAL_GUESS_K,
        &specs,
        &enabled,
    )?;
    let sol = heated.solution;
    let heaters = heater_results(&specs, &heated.heaters, &case.heaters, &sol.temps);
    for (spec, h) in specs.iter().zip(&heated.heaters) {
        internal[spec.node] += h.power_w;
    }
    let nodes = network
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let t = sol.temps[i];
            let ranges = range_margins(t, t, &n.ranges_in(&modes)?, margin);
            Ok(ThermalSteadyNodeResult {
                node: n.id.clone(),
                temperature: Qty::from_raw(t),
                absorbed: Qty::from_raw(absorbed[i]),
                internal: Qty::from_raw(internal[i]),
                verdict: worst_verdict(&ranges),
                ranges,
            })
        })
        .collect::<Result<_, AnalysisError>>()?;
    Ok(ThermalSteadyCaseResult {
        id: case.id.clone(),
        label: case.label.clone(),
        environment: case.environment.used(case.exposure.central())?,
        nodes,
        heaters,
        iterations: sol.iterations,
        residual: Qty::from_raw(sol.residual_w),
        shadowing: heat.shadowing,
    })
}

/// ヒータごとの結果（`specs`・`solved`・`inputs` は同じ順）。
fn heater_results(
    specs: &[HeaterSpec],
    solved: &[HeaterSteady],
    inputs: &[ThermalHeater],
    temps: &[f64],
) -> Vec<ThermalHeaterSteady> {
    specs
        .iter()
        .zip(solved)
        .zip(inputs)
        .map(|((spec, h), input)| ThermalHeaterSteady {
            id: input.id.clone(),
            node: input.node.clone(),
            state: h.state,
            power: Qty::from_raw(h.power_w),
            capacity: Qty::from_raw(spec.max_w),
            duty_fraction: Fraction::new(h.power_w / spec.max_w),
            set_point: Qty::from_raw(spec.on_below),
            node_temperature: Qty::from_raw(temps[spec.node]),
        })
        .collect()
}

fn envelope(
    network: &ThermalNetwork,
    cases: &[ThermalSteadyCaseResult],
    margin: &ThermalMarginUsed,
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
            let ranges =
                envelope_margins(cases.iter().map(|c| c.nodes[i].ranges.as_slice()), margin);
            ThermalSteadyEnvelope {
                node: n.id.clone(),
                min: Qty::from_raw(lo),
                min_case: case_id(lo),
                max: Qty::from_raw(hi),
                max_case: case_id(hi),
                verdict: worst_verdict(&ranges),
                ranges,
            }
        })
        .collect()
}

#[cfg(test)]
mod heater_tests;
#[cfg(test)]
mod tests;
