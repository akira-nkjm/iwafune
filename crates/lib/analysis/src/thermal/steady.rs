//! 熱（定常）（`thermal.steady`）。ケース（高温・低温など）ごとに節点温度の定常解を求め、保管・動作・性能の温度範囲と比べる。
//! 解法は [`crate::thermal::network`]、入熱は [`crate::thermal::orbit_heat`]。

use crate::spec::{Analysis, AnalysisArea, AnalysisError, AnalysisKind, Check, System, Verdict};
use crate::thermal::network::{
    Compiled, INITIAL_GUESS_K, ThermalLoad, ThermalModeLoads, ThermalNetwork,
};
use crate::thermal::orbit_heat::{
    ThermalEnvironment, ThermalEnvironmentUsed, ThermalExposure, exposure_absorbed,
};
use crate::thermal::range::{
    ThermalMarginPolicy, ThermalMarginUsed, ThermalRangeMargin, envelope_margins, range_checks,
    range_margins, worst_verdict,
};
use sat_units::typed::{Power, Qty, Temperature};
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
    /// 内部発熱（`ThermalLoad::dissipation` の合計）
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
    /// 温度範囲を持つ節点の種類ごとの判定（key は `range:<node>:<storage|operating|performance>`）
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
            return Err(AnalysisError::InvalidInput("ケースがありません".to_owned()));
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
        checks.extend(warnings);
        Ok(ThermalSteadyOutput {
            cases,
            envelope,
            margin,
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
    let absorbed = exposure_absorbed(net, &case.environment, &case.exposure)?;
    let internal = net.loads(&case.loads)?;
    let q: Vec<f64> = absorbed.iter().zip(&internal).map(|(a, b)| a + b).collect();
    let sol = net.solve_steady(
        &q,
        case.environment.space_temperature().value(),
        INITIAL_GUESS_K,
    )?;
    let modes = [case.mode_loads.as_ref().map(|l| &l.mode)];
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
        iterations: sol.iterations,
        residual: Qty::from_raw(sol.residual_w),
    })
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
mod tests;
