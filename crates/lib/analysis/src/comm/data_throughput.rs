//! データ送信量と蓄積（`comm.data_throughput`）。パスごとの送信可能量、1 日の送信可能量と発生量、未送信データの推移。
//!
//! - 送信機は 1 台とみなし、複数局が同時に見える時間は先に始まったパスに割り当てる（二重に数えない）。
//! - パスのうち使える時間の割合は `usable_fraction`（捕捉・コマンドの時間などを差し引く）。
//! - `link` を与えると、回線余裕を満たす最低仰角（[`crate::comm::link`]）を局の最低仰角に重ねて、その上だけを数える。
//! - データは一様に発生し、パス中は「発生量 + 蓄積」を上限に送る。

use crate::budget::{BudgetItem, budget_rollup};
use crate::comm::link::{DEFAULT_REQUIRED_MARGIN_DB, RfLinkParams};
use crate::comm::passes::{day_windows, find_all_passes, pass_interval, search_window};
use crate::spec::{
    Analysis, AnalysisError, AnalysisKind, Channel, Check, Series, System, Verdict,
    ensure_fraction, ensure_non_negative, ensure_positive,
};
use backlog::Backlog;
use sat_astro::constants::SECONDS_PER_DAY;
use sat_astro::ground::GroundStation;
use sat_astro::orbit::OrbitSpec;
use sat_astro::time::Epoch;
use sat_units::typed::{Angle, DataRate, Fraction, Information, Qty, Time};
use serde::{Deserialize, Serialize};

/// 回線が閉じる仰角で可視を絞るための回線の条件（データレートは `downlink_rate` を使う）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DataThroughputLink {
    /// ダウンリンクの RF パラメーター（tx = 衛星、rx = 地上局）
    pub rf: RfLinkParams,
    /// 所要 Eb/N0 [dB]
    pub required_eb_n0_db: f64,
    /// 要求する回線余裕 [dB]（既定 [`DEFAULT_REQUIRED_MARGIN_DB`]）
    #[serde(default)]
    pub required_margin_db: Option<f64>,
}

/// データ送信量と蓄積の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DataThroughputInput {
    pub orbit: OrbitSpec,
    pub stations: Vec<GroundStation>,
    /// 開始（既定は軌道の元期）
    #[serde(default)]
    pub start: Option<Epoch>,
    pub span: Qty<Time>,
    /// パス探索の刻み
    pub step: Qty<Time>,
    /// ダウンリンクのデータレート
    pub downlink_rate: Qty<DataRate>,
    /// パスのうちデータ送信に使える時間の割合（0〜1、既定 1）
    #[serde(default)]
    pub usable_fraction: Option<f64>,
    /// 1 日あたりのデータ発生量（`cdh.data_budget` の `total_per_day` など）
    pub generated_per_day: Qty<Information>,
    /// 開始時の未送信データ（既定 0）
    #[serde(default)]
    pub initial_backlog: Option<Qty<Information>>,
    /// 搭載ストレージの容量（与えると未送信データの最大と比べる）
    #[serde(default)]
    pub storage_capacity: Option<Qty<Information>>,
    /// 回線の条件（与えると回線が閉じる仰角より上だけを数える）
    #[serde(default)]
    pub link: Option<DataThroughputLink>,
}

/// 1 パスの送信可能量。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DataThroughputPass {
    pub station: String,
    pub aos: Epoch,
    pub los: Epoch,
    /// 送信に割り当てた時間（他局と重なる分を除く。`usable_fraction` は掛けない）
    pub assigned: Qty<Time>,
    /// 送信可能量 = レート × 割り当て時間 × `usable_fraction`
    pub capacity: Qty<Information>,
}

/// 1 日の集計。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DataThroughputDay {
    /// 0 始まりの日の番号
    pub day: u32,
    pub start: Epoch,
    pub generated: Qty<Information>,
    /// 送信可能量
    pub capacity: Qty<Information>,
    /// 実際に送った量（蓄積が無ければ可能量より少ない）
    pub downlinked: Qty<Information>,
    /// その日の終わりの未送信データ
    pub backlog_end: Qty<Information>,
}

/// データ送信量と蓄積の出力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DataThroughputOutput {
    /// 回線が閉じる最低仰角（`link` を与えたときだけ。天頂でも閉じなければ `None` で `link_closes = false`）
    pub link_min_elevation: Option<Qty<Angle>>,
    /// 回線が閉じるか（`link` が無ければ `None`）
    pub link_closes: Option<bool>,
    pub passes: Vec<DataThroughputPass>,
    pub days: Vec<DataThroughputDay>,
    /// 1 日あたりの平均送信可能量
    pub capacity_per_day: Qty<Information>,
    pub generated_per_day: Qty<Information>,
    /// 送信可能量 / 発生量（1 以上なら溜まらない）
    pub capacity_ratio: Fraction,
    pub max_backlog: Qty<Information>,
    pub final_backlog: Qty<Information>,
    /// 未送信データの推移（折れ線の頂点。`backlog` チャンネル）
    pub series: Series,
    pub checks: Vec<Check>,
}

/// データ送信量と蓄積。
pub struct DataThroughput;

impl Analysis for DataThroughput {
    const ID: &'static str = "comm.data_throughput";
    const TITLE: &'static str = "データ送信量と蓄積";
    const SYSTEM: System = System::Comm;
    const KIND: AnalysisKind = AnalysisKind::TimeSeries;
    const DESCRIPTION: &'static str = "1 日の送信可能量と発生量、未送信データの蓄積";
    const TITLE_EN: &'static str = "Downlink capacity and backlog";
    const DESCRIPTION_EN: &'static str =
        "Daily downlink capacity versus generated data, and the accumulated backlog";
    type Input = DataThroughputInput;
    type Output = DataThroughputOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        validate(input)?;
        let (start, span_s, step_s) =
            search_window(&input.orbit, input.start, input.span, input.step)?;
        let (min_el, closes) = link_min_elevation(input)?;
        let passes = if closes == Some(false) {
            Vec::new()
        } else {
            let stations = raise_min_elevation(&input.stations, min_el);
            find_all_passes(&input.orbit, &stations, start, span_s, step_s)?
        };
        let rate = input.downlink_rate.value() * usable_fraction(input);
        let (pass_results, intervals) = assign_passes(&passes, start, span_s, rate);
        let sim = simulate(input, start, span_s, rate, &intervals);
        build_output(input, span_s, min_el, closes, pass_results, sim)
    }
}

/// パスのうち送信に使える割合（既定 1）。
fn usable_fraction(input: &DataThroughputInput) -> f64 {
    input.usable_fraction.unwrap_or(1.0)
}

fn validate(input: &DataThroughputInput) -> Result<(), AnalysisError> {
    ensure_positive("ダウンリンクのレート", input.downlink_rate.value())?;
    ensure_fraction("パスのうち使える割合", usable_fraction(input))?;
    ensure_non_negative("1 日の発生量", input.generated_per_day.value())?;
    if let Some(b) = input.initial_backlog {
        ensure_non_negative("開始時の未送信データ", b.value())?;
    }
    if let Some(link) = &input.link {
        link.rf.validate("downlink")?;
    }
    Ok(())
}

/// 回線が閉じる最低仰角 [rad] と、閉じるかどうか。`link` が無ければ `(None, None)`。
fn link_min_elevation(
    input: &DataThroughputInput,
) -> Result<(Option<f64>, Option<bool>), AnalysisError> {
    let Some(link) = &input.link else {
        return Ok((None, None));
    };
    // 高度は平均（長半径 − 中心天体の半径）で代表させる。
    let altitude = input.orbit.altitude_m();
    if altitude <= 0.0 {
        return Err(AnalysisError::InvalidInput("軌道の高度が負です".to_owned()));
    }
    let el = link.rf.min_elevation_for_margin(
        altitude,
        input.downlink_rate.value(),
        link.required_eb_n0_db,
        link.required_margin_db
            .unwrap_or(DEFAULT_REQUIRED_MARGIN_DB),
    );
    Ok((el, Some(el.is_some())))
}

fn raise_min_elevation(stations: &[GroundStation], min_el: Option<f64>) -> Vec<GroundStation> {
    stations
        .iter()
        .map(|st| {
            let mut st = st.clone();
            if let Some(el) = min_el {
                st.min_elevation = Qty::from_raw(st.min_elevation.value().max(el));
            }
            st
        })
        .collect()
}

/// パスに送信の時間を割り当てる（先に始まったパスが優先）。区間は開始からの秒。
fn assign_passes(
    passes: &[sat_astro::ground::GroundPass],
    start: Epoch,
    span_s: f64,
    rate_bps: f64,
) -> (Vec<DataThroughputPass>, Vec<(f64, f64)>) {
    let mut busy_until: f64 = 0.0;
    let mut results = Vec::with_capacity(passes.len());
    let mut intervals = Vec::with_capacity(passes.len());
    for pass in passes {
        let (s, e) = pass_interval(pass, start, span_s);
        let s = s.max(busy_until);
        let assigned = (e - s).max(0.0);
        if assigned > 0.0 {
            intervals.push((s, e));
            busy_until = e;
        }
        results.push(DataThroughputPass {
            station: pass.station.clone(),
            aos: pass.aos,
            los: pass.los,
            assigned: Qty::from_raw(assigned),
            capacity: Qty::from_raw(rate_bps * assigned),
        });
    }
    (results, intervals)
}

struct SimResult {
    days: Vec<DataThroughputDay>,
    backlog: Backlog,
}

fn simulate(
    input: &DataThroughputInput,
    start: Epoch,
    span_s: f64,
    rate_bps: f64,
    intervals: &[(f64, f64)],
) -> SimResult {
    let initial = input.initial_backlog.map_or(0.0, Qty::value);
    let mut b = Backlog {
        t: 0.0,
        value: initial,
        gen_bps: input.generated_per_day.value() / SECONDS_PER_DAY,
        points: vec![(0.0, initial)],
    };
    let mut days = Vec::new();
    for (i, (ds, de)) in day_windows(span_s).into_iter().enumerate() {
        let (mut capacity, mut sent) = (0.0, 0.0);
        for &(s, e) in intervals {
            let (s, e) = (s.max(ds), e.min(de));
            if e > s {
                b.idle(s);
                capacity += rate_bps * (e - s);
                sent += b.downlink(e, rate_bps);
            }
        }
        b.idle(de);
        days.push(DataThroughputDay {
            day: i as u32,
            start: start.plus_seconds(ds),
            generated: Qty::from_raw(b.gen_bps * (de - ds)),
            capacity: Qty::from_raw(capacity),
            downlinked: Qty::from_raw(sent),
            backlog_end: Qty::from_raw(b.value),
        });
    }
    SimResult { days, backlog: b }
}

fn build_output(
    input: &DataThroughputInput,
    span_s: f64,
    min_el: Option<f64>,
    closes: Option<bool>,
    passes: Vec<DataThroughputPass>,
    sim: SimResult,
) -> Result<DataThroughputOutput, AnalysisError> {
    let total_capacity: f64 = sim.days.iter().map(|d| d.capacity.value()).sum();
    let capacity_per_day = total_capacity / (span_s / SECONDS_PER_DAY);
    let generated = input.generated_per_day.value();
    let ratio = if generated > 0.0 {
        capacity_per_day / generated
    } else {
        f64::INFINITY
    };
    let max_backlog = sim.backlog.points.iter().map(|p| p.1).fold(0.0, f64::max);
    let checks = checks(input, closes, capacity_per_day, max_backlog)?;
    let (t, values): (Vec<f64>, Vec<f64>) = sim.backlog.points.iter().copied().unzip();
    Ok(DataThroughputOutput {
        link_min_elevation: min_el.map(Qty::from_raw),
        link_closes: closes,
        passes,
        days: sim.days,
        capacity_per_day: Qty::from_raw(capacity_per_day),
        generated_per_day: input.generated_per_day,
        capacity_ratio: Fraction::new(ratio),
        max_backlog: Qty::from_raw(max_backlog),
        final_backlog: Qty::from_raw(sim.backlog.value),
        series: Series {
            t,
            channels: vec![Channel::new(
                "backlog",
                "未送信データ",
                "Information",
                values,
            )],
        },
        checks,
    })
}

fn checks(
    input: &DataThroughputInput,
    closes: Option<bool>,
    capacity_per_day: f64,
    max_backlog: f64,
) -> Result<Vec<Check>, AnalysisError> {
    let mut out = Vec::new();
    if closes == Some(false) {
        out.push(Check::new(
            "link_closes",
            "回線",
            Verdict::Violation,
            "天頂でも要求の回線余裕を満たしません",
        ));
    }
    let generated = input.generated_per_day.value();
    out.extend(limit_check(
        "capacity",
        "送信可能量と発生量",
        generated,
        capacity_per_day,
    )?);
    if let Some(cap) = input.storage_capacity {
        out.extend(limit_check(
            "storage",
            "未送信データとストレージ",
            max_backlog,
            cap.value(),
        )?);
    }
    Ok(out)
}

/// `value ≤ limit` の判定（バジェットの判定 `budget::budget_rollup` を 1 行で使う）。
fn limit_check(
    key: &str,
    label: &str,
    value: f64,
    limit: f64,
) -> Result<Option<Check>, AnalysisError> {
    let item = BudgetItem {
        id: key.to_owned(),
        group: String::new(),
        value,
        count: 1,
        growth: 0.0,
    };
    Ok(budget_rollup(&[item], Some(limit), 0.0)?.check(key, label, "Information"))
}

#[path = "data_throughput_backlog.rs"]
mod backlog;

#[cfg(test)]
#[path = "data_throughput_tests.rs"]
mod tests;
