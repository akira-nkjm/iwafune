//! 熱（非定常）（`thermal.transient`）。軌道上の入熱と内部発熱の区間（モードなど）で節点温度を時間積分する。
//!
//! - 積分は `sat_astro::ode::Dop853`（適応刻み・密出力）。許容誤差は `integration`（既定は解析が持つ）。
//! - 外部入熱は積分の右辺の中で時刻ごとに求める（`sat_astro::timeline::TimelineEvaluator`。サンプルの間も連続）。
//!   日陰の境（日照率が折れ曲がる時刻）・発熱の区間の切り替わり・サンプルの時刻で区間を分ける（`simulate`）。
//! - 内部発熱は `phases` を順に繰り返す（1 区間だけなら一定）。区間の切り替わりの時刻で積分を分ける。
//! - ヒータは設定温度に届く時刻を密出力で求めて切り替える（`heater_control`）。出力の時刻はタイムラインのサンプルのまま。

use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Channel, ChannelKind, Check, Series,
    System, Verdict,
};
use crate::thermal::heater::{ThermalHeaterRun, run_checks};
use crate::thermal::network::{
    Compiled, ThermalCouplings, ThermalHeater, ThermalLoad, ThermalModeLoads, ThermalNetwork,
};
use crate::thermal::orbit_heat::{ThermalEnvironment, ThermalEnvironmentUsed};
use crate::thermal::range::{
    ThermalMarginPolicy, ThermalMarginUsed, ThermalNodeRange, ThermalRangeMargin, range_checks,
    range_margins, worst_verdict,
};
use sat_astro::timeline::{
    OrbitPlaneTrack, TimelineEvaluator, TimelineSpec, build, orbit_plane_track, time_mean,
};
use sat_units::typed::{Power, Qty, Temperature, Time};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod heater_control;
mod initial;
mod integration;
mod phases;
mod simulate;
use heater_control::HeaterControl;
use initial::initial;
pub use integration::{DEFAULT_ABS_TOL_K, DEFAULT_REL_TOL, ThermalIntegration};
use phases::Phases;
use simulate::Simulation;

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
    /// 積分の刻みの上限（既定は上限なし。刻みは許容誤差で決まる）
    #[serde(default)]
    pub max_step: Option<Qty<Time>>,
    /// 積分の相対許容誤差（既定は [`DEFAULT_REL_TOL`]）
    #[serde(default)]
    pub rel_tol: Option<f64>,
    /// 積分の絶対許容誤差（既定は [`DEFAULT_ABS_TOL_K`]）
    #[serde(default)]
    pub abs_tol: Option<Qty<Temperature>>,
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
    /// 使った積分の設定と費用（刻みの数・右辺の評価の数・ヒータの切り替えの数）
    #[serde(default)]
    pub integration: ThermalIntegration,
    /// 軌道の図（インセット）と 3D の太陽の矢印に使う軌道面の表示用の座標。標本の時刻は `series.t` と同じ。
    /// 表示枠は最初の標本で決めた固定の軌道面（+x が太陽側）。模式で、実際の日陰は `sunlit`（`sat_astro::timeline`）
    #[serde(default)]
    pub orbit_plane: OrbitPlaneTrack,
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
        let central = timeline.orbit().central();
        let env = input.environment.resolve(central)?;
        let cfg = integration::integrator(input)?;
        let evaluator = TimelineEvaluator::new(timeline)?;
        let t = timeline.sampling.times()?;
        let space_k = input.environment.space_temperature().value();
        let temps = initial(&net, input, timeline, &phases, &specs)?;
        let t0 = t.first().copied().unwrap_or(0.0);
        let t_end = t.last().copied().unwrap_or(0.0);
        let window_start = t0.max(t_end - timeline.orbit().cycle().length_s());
        let mut control =
            HeaterControl::new(&specs, &temps, phases.enabled_at(t0), t0, window_start);
        let sim = Simulation {
            net: &net,
            env: &env,
            phases: &phases,
            timeline: &evaluator,
            space_k,
            cfg,
        };
        let run = sim.run(&t, temps, &mut control)?;
        let integration = ThermalIntegration::new(&sim.cfg, input, run.segments, &run.cost);
        let (runs, heater_power) = control.finish(&input.heaters, t0, t_end);
        let ranges = node_ranges(input)?;
        let mut output = build_output(
            &input.network,
            &ranges,
            t,
            &run.history,
            run.absorbed,
            input.environment.used(central)?,
            margin,
        );
        output.couplings = net.couplings;
        output.integration = integration;
        let shadow_below = crate::spec::SUNLIT_THRESHOLD;
        output.orbit_plane = orbit_plane_track(&build(timeline)?, central, shadow_below);
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
        integration: ThermalIntegration::default(),
        orbit_plane: OrbitPlaneTrack::default(),
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
