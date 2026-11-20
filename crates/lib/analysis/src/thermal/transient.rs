//! 熱（非定常）（`thermal.transient`）。軌道上の入熱と内部発熱の区間（モードなど）で節点温度を時間積分する。
//!
//! - 入熱は `sat_astro::timeline` の各サンプルで求め、次のサンプルまで一定とする（sample-and-hold）。
//! - 内部発熱は `phases` を順に繰り返す（1 区間だけなら一定）。区間はサンプルの時刻で切り替える。
//! - 積分は `sat_astro::ode::integrate`（固定刻み RK4）。刻みは安定条件の目安と `max_step` の小さい方。

use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Channel, ChannelKind, Check, Series,
    System, Verdict,
};
use crate::thermal::environment::ResolvedEnvironment;
use crate::thermal::heater::{ThermalHeaterRun, run_checks};
use crate::thermal::network::HeaterSpec;
use crate::thermal::network::{
    Compiled, INITIAL_GUESS_K, ThermalCouplings, ThermalHeater, ThermalLoad, ThermalModeLoads,
    ThermalNetwork,
};
use crate::thermal::orbit_heat::{
    Geometry, ThermalEnvironment, ThermalEnvironmentUsed, absorbed, mean_absorbed,
    timeline_geometry,
};
use crate::thermal::range::{
    ThermalMarginPolicy, ThermalMarginUsed, ThermalNodeRange, ThermalRangeMargin, range_checks,
    range_margins, worst_verdict,
};
use sat_astro::ode::integrate;
use sat_astro::timeline::TimelineSpec;
use sat_astro::timeline::time_mean;
use sat_units::typed::{Power, Qty, Temperature, Time};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod heater_control;
mod phases;
use heater_control::HeaterControl;
use phases::Phases;

/// 長さが書かれていない発熱の区間の長さ [s]。1 日。区間が 1 つだけなら周期で平均した発熱はその区間の発熱で、
/// この長さは結果に効かない（複数の区間があるときは各区間の長さを書く）。
pub const DEFAULT_PHASE_DURATION_S: f64 = sat_astro::constants::SECONDS_PER_DAY;

/// 内部発熱の 1 区間（例: モード 1 つ）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ThermalHeatPhase {
    /// 例: モードの ID
    pub label: String,
    /// 区間の長さ。省略時は [`DEFAULT_PHASE_DURATION_S`]（区間が 1 つなら発熱は一定で、長さは結果に効かない）
    #[serde(default)]
    pub duration: Option<Qty<Time>>,
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
    /// サーモスタット付きのヒータ（走行の間ずっと状態を持つ。サーモスタットが働くかは区間のモードごとに決まり、
    /// モードの無い区間では働く）。オンとオフは設定温度の帯で切り替える（`network::heater`）。
    /// `phases` の `loads` や `mode_loads` に同じ部品を入れない
    #[serde(default)]
    pub heaters: Vec<ThermalHeater>,
    pub initial: ThermalInitial,
    /// 積分の刻みの上限（既定は安定条件の目安だけで決める）
    #[serde(default)]
    pub max_step: Option<Qty<Time>>,
    /// 解析マージンの方針（書かれていない値は解析の既定値）
    #[serde(default)]
    pub margin: ThermalMarginPolicy,
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
    /// 温度範囲（保管・動作・性能の順。区間のモードがあれば部品の電源の状態で選ぶ）と最低・最高の比較
    pub ranges: Vec<ThermalRangeMargin>,
    /// 範囲との比較のうち最も悪い判定（範囲が無ければ Ok）
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
    /// 使った解析マージン（不確かさと警告の幅）
    pub margin: ThermalMarginUsed,
    /// 使った節点間の結合（伝導・放射と出どころ。外殻の放射を求めたときは形態係数）
    #[serde(default)]
    pub couplings: ThermalCouplings,
    /// ヒータごとの結果（入力の順。ヒータが無ければ空）。発熱の時系列は `series` の `heater:<部品>`（W）
    #[serde(default)]
    pub heaters: Vec<ThermalHeaterRun>,
    /// 温度範囲を持つ節点の種類ごとの判定（key は `range_high|range_low:<node>:<storage|operating|performance>`）と、
    /// ヒータごとの判定（key は `heater:<部品>`）
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
        let margin = input.margin.used();
        margin.validate()?;
        let specs = net.heaters(&input.heaters)?;
        let phases = Phases::new(&net, &input.phases, &input.heaters)?;
        let timeline = &input.timeline;
        let central = timeline.orbit().central;
        let env = input.environment.resolve(central)?;
        let (t, geo) = timeline_geometry(timeline)?;
        let space_k = input.environment.space_temperature().value();
        let temps = initial(&net, input, timeline, &phases, &specs)?;
        let t0 = t.first().copied().unwrap_or(0.0);
        let t_end = t.last().copied().unwrap_or(0.0);
        let window_start = t0.max(t_end - timeline.orbit().period_s());
        let control = HeaterControl::new(&specs, &temps, phases.enabled_at(t0), t0, window_start);
        let sim = Simulation {
            net: &net,
            env: &env,
            phases: &phases,
            control: &control,
            space_k,
            input,
        };
        let (history, absorbed_total) = sim.integrate(&t, &geo, temps);
        let (runs, heater_power) = control.finish(&input.heaters, t0, t_end);
        let ranges = node_ranges(input)?;
        let mut output = build_output(
            &input.network,
            &ranges,
            t,
            &history,
            absorbed_total,
            input.environment.used(central)?,
            margin,
        );
        output.couplings = net.couplings;
        output
            .checks
            .extend(run_checks(&runs, margin.heater_duty_warning.value()));
        output
            .series
            .channels
            .extend(runs.iter().zip(heater_power).map(|(r, p)| {
                Channel::of::<Power>(format!("heater:{}", r.id), ChannelKind::HeaterPower, p)
                    .about(&r.id)
            }));
        output.heaters = runs;
        output.checks.extend(phases.checks);
        Ok(output)
    }
}

/// 節点ごとの温度範囲（区間のモードがあれば部品の電源の状態で選ぶ）。
fn node_ranges(input: &ThermalTransientInput) -> Result<Vec<Vec<ThermalNodeRange>>, AnalysisError> {
    let modes: Vec<_> = input
        .phases
        .iter()
        .map(|p| p.mode_loads.as_ref().map(|l| &l.mode))
        .collect();
    input
        .network
        .nodes
        .iter()
        .map(|n| n.ranges_in(&modes))
        .collect()
}

/// 時間積分の道具（入熱・発熱の区間・ヒータの制御器と刻み）。
struct Simulation<'a> {
    net: &'a Compiled,
    env: &'a ResolvedEnvironment,
    phases: &'a Phases,
    control: &'a HeaterControl<'a>,
    space_k: f64,
    /// 積分の刻みを決めるのに使う入力（`step_s`）
    input: &'a ThermalTransientInput,
}

impl Simulation<'_> {
    /// 各サンプルの時刻 `t` の温度（初期値 `temps` から）と外部入熱の合計を求める。入熱・発熱は次のサンプルまで一定、
    /// ヒータは積分の刻みごとに状態を決める（`HeaterControl`）。
    fn integrate(
        &self,
        t: &[f64],
        geo: &[Geometry],
        mut temps: Vec<f64>,
    ) -> (Vec<Vec<f64>>, Vec<f64>) {
        let mut history: Vec<Vec<f64>> = Vec::with_capacity(t.len());
        let mut absorbed_total = Vec::with_capacity(t.len());
        for i in 0..t.len() {
            history.push(temps.clone());
            let q_ext = absorbed(self.net, self.env, &geo[i]);
            absorbed_total.push(q_ext.iter().sum());
            self.control
                .begin_sample(t[i], &temps, self.phases.enabled_at(t[i]));
            let Some(&t_next) = t.get(i + 1) else { break };
            let q: Vec<f64> = q_ext
                .iter()
                .zip(self.phases.at(t[i]))
                .map(|(a, b)| a + b)
                .collect();
            // 刻みは時刻ごとに今の温度で決め直す（放射の剛性は温度の 3 乗で増えるので、初期温度だけで決めると、
            // 高温まで上がる物体で刻みが粗すぎて発散する）
            let dt = step_s(self.net, self.input, &temps);
            let f = |_: f64, x: &Vec<f64>| {
                let mut with_heaters = q.clone();
                self.control.add_heat(&mut with_heaters);
                self.net.derivative(x, &with_heaters, self.space_k)
            };
            temps = integrate(f, t[i], temps, t_next, dt, |tn, x| {
                self.control.observe(tn, x);
            });
        }
        (history, absorbed_total)
    }
}

fn initial(
    net: &Compiled,
    input: &ThermalTransientInput,
    timeline: &TimelineSpec,
    phases: &Phases,
    heaters: &[HeaterSpec],
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
            // ヒータは最初の区間のサーモスタットで、下側の設定温度に保つ電力を出した定常解から始める
            let enabled = phases.enabled_at(0.0);
            net.solve_steady_heated(&q, space_k, INITIAL_GUESS_K, heaters, enabled)?
                .solution
                .temps
        }
    };
    if t.iter().any(|x| x.is_nan() || *x <= 0.0) {
        return Err(AnalysisError::InvalidInput(
            "Initial temperature must be positive".to_owned(),
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

/// 積分の刻み [s]。時刻ごとに、今の温度（と下限の 400 K）で決め直す。
fn step_s(net: &Compiled, input: &ThermalTransientInput, temps: &[f64]) -> f64 {
    let t_ref = temps
        .iter()
        .fold(STABLE_STEP_MIN_REFERENCE_K, |m, x| m.max(*x));
    let stable = net.stable_step_s(t_ref);
    let limit = input.max_step.map_or(f64::INFINITY, Qty::value);
    let timeline = input.timeline.sampling.step.value();
    stable.min(limit).min(timeline).max(MIN_STEP_S)
}

/// 出力を組み立てる。`ranges` は節点ごとの温度範囲（`ThermalNode::ranges_in`。入力の節点の順）。
fn build_output(
    network: &ThermalNetwork,
    ranges: &[Vec<ThermalNodeRange>],
    t: Vec<f64>,
    history: &[Vec<f64>],
    absorbed_total: Vec<f64>,
    environment: ThermalEnvironmentUsed,
    margin: ThermalMarginUsed,
) -> ThermalTransientOutput {
    let mut channels = Vec::with_capacity(network.nodes.len() + 1);
    let mut nodes = Vec::with_capacity(network.nodes.len());
    let mut checks = Vec::new();
    for (i, n) in network.nodes.iter().enumerate() {
        let values: Vec<f64> = history.iter().map(|row| row[i]).collect();
        let (i_min, min) = extreme(&values, |a, b| a < b);
        let (i_max, max) = extreme(&values, |a, b| a > b);
        let mean = time_mean(&t, &values);
        let ranges = range_margins(min, max, &ranges[i], &margin);
        checks.extend(range_checks(&n.id, &ranges, &margin));
        nodes.push(ThermalTransientNodeResult {
            node: n.id.clone(),
            min: Qty::from_raw(min),
            max: Qty::from_raw(max),
            mean: Qty::from_raw(mean),
            time_of_min: Qty::from_raw(t.get(i_min).copied().unwrap_or(0.0)),
            time_of_max: Qty::from_raw(t.get(i_max).copied().unwrap_or(0.0)),
            verdict: worst_verdict(&ranges),
            ranges,
        });
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
        margin,
        couplings: ThermalCouplings::default(),
        heaters: Vec::new(),
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
mod heater_tests;
#[cfg(test)]
mod tests;
