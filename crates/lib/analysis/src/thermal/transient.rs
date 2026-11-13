//! 熱（非定常）（`thermal.transient`）。軌道上の入熱と内部発熱の区間（モードなど）で節点温度を時間積分する。
//!
//! - 入熱は `sat_astro::timeline` の各サンプルで求め、次のサンプルまで一定とする（sample-and-hold）。
//! - 内部発熱は `phases` を順に繰り返す（1 区間だけなら一定）。区間はサンプルの時刻で切り替える。
//! - 積分は `sat_astro::ode::integrate`（固定刻み RK4）。刻みは安定条件の目安と `max_step` の小さい方。

use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Channel, ChannelKind, Check, Series,
    System, Verdict,
};
use crate::thermal::network::{
    Compiled, INITIAL_GUESS_K, ThermalLoad, ThermalModeLoads, ThermalNetwork, ThermalTempRange,
};
use crate::thermal::orbit_heat::{
    ThermalEnvironment, ThermalEnvironmentUsed, absorbed, mean_absorbed, timeline_geometry,
};
use crate::thermal::range::{RANGE_WARNING_BAND_K, range_check, range_verdict};
use sat_astro::ode::integrate;
use sat_astro::timeline::TimelineSpec;
use sat_astro::timeline::time_mean;
use sat_units::typed::{Power, Qty, Temperature, Time};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 内部発熱の 1 区間（例: モード 1 つ）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalHeatPhase {
    /// 例: モードの ID
    pub label: String,
    pub duration: Qty<Time>,
    #[serde(default)]
    pub loads: Vec<ThermalLoad>,
    /// モード発熱。未割り当て・値不足は出力 checks に警告。
    #[serde(default)]
    pub mode_loads: Option<ThermalModeLoads>,
}

/// 初期温度の与え方。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ThermalInitial {
    /// すべての節点を同じ温度にする
    Uniform { temperature: Qty<Temperature> },
    /// 節点ごと（書いていない節点は `default`）
    PerNode {
        temperatures: BTreeMap<String, Qty<Temperature>>,
        default: Qty<Temperature>,
    },
    /// 軌道平均の入熱と区間平均の発熱での定常解から始める
    OrbitAverageSteady,
}

/// 熱（非定常）の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalTransientInput {
    pub network: ThermalNetwork,
    /// 軌道・姿勢則・期間・刻み（入熱を求める時刻と、出力の時刻）。β 角の指定は `timeline` の `beta`
    pub timeline: TimelineSpec,
    #[serde(default)]
    pub environment: ThermalEnvironment,
    /// 内部発熱の区間（順に繰り返す。空なら発熱なし）
    #[serde(default)]
    pub phases: Vec<ThermalHeatPhase>,
    pub initial: ThermalInitial,
    /// 積分の刻みの上限（既定は安定条件の目安だけで決める）
    #[serde(default)]
    pub max_step: Option<Qty<Time>>,
}

/// 節点ごとの最高・最低。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalTransientNodeResult {
    pub node: String,
    pub min: Qty<Temperature>,
    pub max: Qty<Temperature>,
    pub mean: Qty<Temperature>,
    /// 最低になった時刻（開始からの経過）
    pub time_of_min: Qty<Time>,
    pub time_of_max: Qty<Time>,
    pub operating: Option<ThermalTempRange>,
    pub verdict: Verdict,
}

/// 熱（非定常）の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalTransientOutput {
    /// 節点ごとの温度（key は節点 ID、K）と全外部入熱（key `absorbed`、W）
    pub series: Series,
    /// 使った熱環境（入力に無い値は既定値）
    pub environment: ThermalEnvironmentUsed,
    /// 入力の節点の順
    pub nodes: Vec<ThermalTransientNodeResult>,
    /// 動作範囲の端からこの幅以内を警告にした（`thermal::range::RANGE_WARNING_BAND_K`）
    pub range_warning_band: Qty<Temperature>,
    /// 動作範囲を持つ節点ごとの判定（key は `range:<node>`）
    pub checks: Vec<Check>,
}

/// 熱（非定常）。
pub struct ThermalTransient;

impl Analysis for ThermalTransient {
    const ID: &'static str = "thermal.transient";
    const TITLE: &'static str = "熱（非定常）";
    const AREA: AnalysisArea = AnalysisArea::System(System::Thermal);
    const KIND: AnalysisKind = AnalysisKind::TimeSeries;
    const DESCRIPTION: &'static str = "軌道上の多節点モデルの温度の時間変化";
    const TITLE_EN: &'static str = "Thermal (transient)";
    const DESCRIPTION_EN: &'static str = "Time history of node temperatures over the orbit";
    type Input = ThermalTransientInput;
    type Output = ThermalTransientOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        let net = Compiled::new(&input.network)?;
        let phases = Phases::new(&net, &input.phases)?;
        let timeline = &input.timeline;
        let (t, geo) = timeline_geometry(timeline)?;
        let space_k = input.environment.space_temperature().value();
        let mut temps = initial(&net, input, timeline, &phases)?;
        let dt = step_s(&net, input, &temps);
        let mut history: Vec<Vec<f64>> = Vec::with_capacity(t.len());
        let mut absorbed_total = Vec::with_capacity(t.len());
        for i in 0..t.len() {
            history.push(temps.clone());
            let q_ext = absorbed(&net, &input.environment, &geo[i]);
            absorbed_total.push(q_ext.iter().sum());
            let Some(&t_next) = t.get(i + 1) else { break };
            let q: Vec<f64> = q_ext
                .iter()
                .zip(phases.at(t[i]))
                .map(|(a, b)| a + b)
                .collect();
            let f = |_: f64, x: &Vec<f64>| net.derivative(x, &q, space_k);
            temps = integrate(f, t[i], temps, t_next, dt, |_, _| {});
        }
        let mut output = build_output(
            &input.network,
            t,
            &history,
            absorbed_total,
            input.environment.used(),
        );
        output.checks.extend(phases.checks);
        Ok(output)
    }
}

/// 内部発熱の区間を節点ごとの配列にしたもの。
struct Phases {
    /// (区間の終わり（周期の中の時刻）[s], 節点ごとの発熱 [W])
    steps: Vec<(f64, Vec<f64>)>,
    cycle_s: f64,
    zero: Vec<f64>,
    checks: Vec<Check>,
}

impl Phases {
    fn new(net: &Compiled, phases: &[ThermalHeatPhase]) -> Result<Self, AnalysisError> {
        let mut steps = Vec::with_capacity(phases.len());
        let mut end = 0.0;
        let mut checks = Vec::new();
        for (i, p) in phases.iter().enumerate() {
            if p.duration.value().is_nan() || p.duration.value() <= 0.0 {
                return Err(AnalysisError::InvalidInput(format!(
                    "発熱の区間 {} の長さは正にしてください",
                    p.label
                )));
            }
            end += p.duration.value();
            let mut loads = p.loads.clone();
            if let Some(mode) = &p.mode_loads {
                let (mode_loads, warnings) = mode.evaluate()?;
                loads.extend(mode_loads);
                checks.extend(warnings.into_iter().map(|mut check| {
                    check.key = format!("phase:{i}:{}", check.key);
                    check
                }));
            }
            steps.push((end, net.loads(&loads)?));
        }
        Ok(Self {
            steps,
            checks,
            cycle_s: end,
            zero: vec![0.0; net.len()],
        })
    }

    /// 時刻 `t_s` の発熱。
    fn at(&self, t_s: f64) -> &[f64] {
        if self.steps.is_empty() {
            return &self.zero;
        }
        let tau = t_s.rem_euclid(self.cycle_s);
        self.steps
            .iter()
            .find(|(end, _)| tau < *end)
            .or(self.steps.last())
            .map_or(&self.zero, |(_, q)| q)
    }

    /// 周期で平均した発熱。
    fn mean(&self) -> Vec<f64> {
        let mut out = self.zero.clone();
        let mut start = 0.0;
        for (end, q) in &self.steps {
            for (o, x) in out.iter_mut().zip(q) {
                *o += x * (end - start) / self.cycle_s;
            }
            start = *end;
        }
        out
    }
}

fn initial(
    net: &Compiled,
    input: &ThermalTransientInput,
    timeline: &TimelineSpec,
    phases: &Phases,
) -> Result<Vec<f64>, AnalysisError> {
    let mut t = match &input.initial {
        ThermalInitial::Uniform { temperature } => vec![temperature.value(); net.len()],
        ThermalInitial::PerNode {
            temperatures,
            default,
        } => {
            let mut t = vec![default.value(); net.len()];
            for (id, v) in temperatures {
                t[net.index(id)?] = v.value();
            }
            t
        }
        ThermalInitial::OrbitAverageSteady => {
            let ext = mean_absorbed(net, &input.environment, timeline)?;
            let q: Vec<f64> = ext.iter().zip(phases.mean()).map(|(a, b)| a + b).collect();
            let space_k = input.environment.space_temperature().value();
            net.solve_steady(&q, space_k, INITIAL_GUESS_K)?.temps
        }
    };
    if t.iter().any(|x| x.is_nan() || *x <= 0.0) {
        return Err(AnalysisError::InvalidInput(
            "初期温度は正にしてください".to_owned(),
        ));
    }
    for (ti, f) in t.iter_mut().zip(&net.fixed) {
        if let Some(v) = f {
            *ti = *v;
        }
    }
    Ok(t)
}

/// 安定な刻みの目安を求める温度の下限 [K]。放射の線形化の剛性は温度の 3 乗で増えるので、初期温度がこれより低くても
/// 軌道上で上がりうる温度で見積もる。
const STABLE_STEP_MIN_REFERENCE_K: f64 = 400.0;

/// 積分の刻みの下限 [s]（剛い節点があっても刻みが 0 に近づいて終わらなくならないように）。
const MIN_STEP_S: f64 = 1e-3;

/// 積分の刻み [s]。
fn step_s(net: &Compiled, input: &ThermalTransientInput, temps: &[f64]) -> f64 {
    let t_ref = temps
        .iter()
        .fold(STABLE_STEP_MIN_REFERENCE_K, |m, x| m.max(*x));
    let stable = net.stable_step_s(t_ref);
    let limit = input.max_step.map_or(f64::INFINITY, Qty::value);
    let timeline = input.timeline.sampling.step.value();
    stable.min(limit).min(timeline).max(MIN_STEP_S)
}

fn build_output(
    network: &ThermalNetwork,
    t: Vec<f64>,
    history: &[Vec<f64>],
    absorbed_total: Vec<f64>,
    environment: ThermalEnvironmentUsed,
) -> ThermalTransientOutput {
    let mut channels = Vec::with_capacity(network.nodes.len() + 1);
    let mut nodes = Vec::with_capacity(network.nodes.len());
    let mut checks = Vec::new();
    for (i, n) in network.nodes.iter().enumerate() {
        let values: Vec<f64> = history.iter().map(|row| row[i]).collect();
        let (i_min, min) = extreme(&values, |a, b| a < b);
        let (i_max, max) = extreme(&values, |a, b| a > b);
        let mean = time_mean(&t, &values);
        nodes.push(ThermalTransientNodeResult {
            node: n.id.clone(),
            min: Qty::from_raw(min),
            max: Qty::from_raw(max),
            mean: Qty::from_raw(mean),
            time_of_min: Qty::from_raw(t.get(i_min).copied().unwrap_or(0.0)),
            time_of_max: Qty::from_raw(t.get(i_max).copied().unwrap_or(0.0)),
            operating: n.operating(),
            verdict: range_verdict(min, max, n.operating()),
        });
        checks.extend(range_check(&n.id, min, max, n.operating()));
        channels.push(
            Channel::of::<Temperature>(&n.id, ChannelKind::NodeTemperature, values).about(&n.id),
        );
    }
    channels.push(Channel::of::<Power>(
        "absorbed",
        ChannelKind::AbsorbedHeat,
        absorbed_total,
    ));
    ThermalTransientOutput {
        series: Series { t, channels },
        environment,
        nodes,
        range_warning_band: Qty::from_raw(RANGE_WARNING_BAND_K),
        checks,
    }
}

/// `better(a, b)` が真になる方を選んだ (添字, 値)。
fn extreme(values: &[f64], better: impl Fn(f64, f64) -> bool) -> (usize, f64) {
    let mut best = (0, values.first().copied().unwrap_or(f64::NAN));
    for (i, &v) in values.iter().enumerate() {
        if better(v, best.1) {
            best = (i, v);
        }
    }
    best
}

#[cfg(test)]
mod tests;
