//! データ送信量と蓄積（`comm.data_throughput`）。パスごとの送信可能量、1 日の送信可能量と発生量、未送信データの推移。
//!
//! - 送信機は 1 台とみなし、複数局が同時に見える時間は先に始まったパスに割り当てる（二重に数えない）。
//! - パスのうち使える時間の割合は `usable_pass_fraction`（捕捉・コマンドの時間などを差し引く）。
//! - `link` を与えると、回線余裕を満たす最低仰角（[`crate::comm::link`]）を局の最低仰角に重ねて、その上だけを数える。
//! - データは一様に発生し、パス中は「発生量 + 蓄積」を上限に送る。

use crate::budget::{BudgetItem, limit_check};
use crate::comm::link::{RfLinkParams, required_margin};
use crate::comm::passes::{
    SearchWindow, day_windows, find_all_passes, pass_interval, search_window,
};
use crate::spec::{
    Analysis, AnalysisError, AnalysisKind, Channel, ChannelKind, Check, Series, System, Verdict,
    ensure_fraction, ensure_non_negative, ensure_positive, ratio_margin,
};
use backlog::Backlog;
use sat_astro::constants::SECONDS_PER_DAY;
use sat_astro::ground::GroundStation;
use sat_astro::time::Epoch;
use sat_astro::timeline::OrbitSampling;
use sat_units::typed::{Angle, DataRate, Decibel, Fraction, Information, Qty, Time};
use serde::{Deserialize, Serialize};

/// パスのうちデータ送信に使える時間の割合の既定値（パスの全部を使う）。
pub const DEFAULT_USABLE_PASS_FRACTION: Fraction = Fraction::new(1.0);

/// 回線が閉じる仰角で可視を絞るための回線の条件（データレートは `downlink_rate` を使う）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DataThroughputLink {
    /// ダウンリンクの RF パラメーター（tx = 衛星、rx = 地上局）
    pub rf: RfLinkParams,
    /// 所要 Eb/N0
    pub required_eb_n0_db: Decibel,
}

/// データ送信量と蓄積の入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DataThroughputInput {
    /// 軌道・期間・パス探索の刻み・β（時系列と同じ標本の取り方。開始は軌道の元期）
    pub sampling: OrbitSampling,
    pub stations: Vec<GroundStation>,
    /// ダウンリンクのデータレート
    pub downlink_rate: Qty<DataRate>,
    /// パスのうちデータ送信に使える時間の割合（0〜1、既定 [`DEFAULT_USABLE_PASS_FRACTION`]）
    #[serde(default)]
    pub usable_pass_fraction: Option<Fraction>,
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
    /// 要求する回線余裕（`link` があるときに使う。既定 [`crate::comm::link::DEFAULT_REQUIRED_MARGIN`]）
    #[serde(default)]
    pub required_margin_db: Option<Decibel>,
}

impl DataThroughputInput {
    /// パスのうち送信に使える割合（省略時 [`DEFAULT_USABLE_PASS_FRACTION`]）。
    pub fn usable_pass_fraction(&self) -> Fraction {
        self.usable_pass_fraction
            .unwrap_or(DEFAULT_USABLE_PASS_FRACTION)
    }

    /// 開始時の未送信データ（省略時 0）。
    pub fn initial_backlog(&self) -> Qty<Information> {
        self.initial_backlog.unwrap_or(Qty::ZERO)
    }

    /// 要求する回線余裕（省略時 [`crate::comm::link::DEFAULT_REQUIRED_MARGIN`]）。
    pub fn required_margin_db(&self) -> Decibel {
        required_margin(self.required_margin_db)
    }
}

/// 回線が閉じるか（`link` を与えたときの、回線余裕を満たす最低仰角）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DataThroughputLinkClosure {
    /// 回線の条件を与えていない（局の最低仰角だけで数える）
    NotEvaluated,
    /// この仰角より上で回線が閉じる（局の最低仰角に重ねる）
    ClosesAbove { min_elevation: Qty<Angle> },
    /// 天頂でも要求の回線余裕を満たさない（送れない）
    NeverCloses,
}

/// 1 パスの送信可能量。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DataThroughputPass {
    pub station: String,
    pub aos: Epoch,
    pub los: Epoch,
    /// 送信に割り当てた時間（他局と重なる分を除く。`usable_pass_fraction` は掛けない）
    pub assigned: Qty<Time>,
    /// 送信可能量 = レート × 割り当て時間 × `usable_pass_fraction`
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
    /// 回線が閉じるか（`link` が無ければ `not_evaluated`）
    pub link_closure: DataThroughputLinkClosure,
    /// 使った回線余裕の要求値（`link` があるときだけ）
    pub required_margin_db: Option<Decibel>,
    /// 使った「パスのうち送信に使える割合」（入力が無ければ既定値）
    pub usable_pass_fraction: Fraction,
    pub passes: Vec<DataThroughputPass>,
    pub days: Vec<DataThroughputDay>,
    /// 1 日あたりの平均送信可能量
    pub capacity_per_day: Qty<Information>,
    pub generated_per_day: Qty<Information>,
    /// 送信の余裕 = 1 日の送信可能量 / 発生量 − 1（[`crate::spec::ratio_margin`]。0 以上なら溜まらない。発生量が 0 なら `None`）
    pub downlink_margin: Option<Fraction>,
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
        let w = search_window(&input.sampling)?;
        let closure = link_closure(input, &w)?;
        let passes = match closure {
            DataThroughputLinkClosure::NeverCloses => Vec::new(),
            DataThroughputLinkClosure::NotEvaluated => {
                find_all_passes(&w.orbit, &input.stations, w.start, w.span_s, w.step_s)?
            }
            DataThroughputLinkClosure::ClosesAbove { min_elevation } => {
                let stations = raise_min_elevation(&input.stations, min_elevation);
                find_all_passes(&w.orbit, &stations, w.start, w.span_s, w.step_s)?
            }
        };
        let rate = input.downlink_rate.value() * input.usable_pass_fraction().value();
        let (pass_results, intervals) = assign_passes(&passes, w.start, w.span_s, rate);
        let sim = simulate(input, w.start, w.span_s, rate, &intervals);
        build_output(input, w.span_s, closure, pass_results, sim)
    }
}

fn validate(input: &DataThroughputInput) -> Result<(), AnalysisError> {
    ensure_positive("ダウンリンクのレート", input.downlink_rate.value())?;
    ensure_fraction("パスのうち使える割合", input.usable_pass_fraction().value())?;
    ensure_non_negative("1 日の発生量", input.generated_per_day.value())?;
    ensure_non_negative("開始時の未送信データ", input.initial_backlog().value())?;
    if let Some(link) = &input.link {
        link.rf.validate("downlink")?;
    }
    Ok(())
}

/// 回線が閉じる最低仰角。`link` が無ければ [`DataThroughputLinkClosure::NotEvaluated`]。
fn link_closure(
    input: &DataThroughputInput,
    w: &SearchWindow,
) -> Result<DataThroughputLinkClosure, AnalysisError> {
    let Some(link) = &input.link else {
        return Ok(DataThroughputLinkClosure::NotEvaluated);
    };
    // 高度は平均（長半径 − 中心天体の半径）で代表させる。
    let altitude = w.orbit.altitude_m();
    if altitude <= 0.0 {
        return Err(AnalysisError::InvalidInput("軌道の高度が負です".to_owned()));
    }
    let el = link.rf.min_elevation_for_margin(
        w.orbit.central.radius_m(),
        altitude,
        input.downlink_rate.value(),
        link.required_eb_n0_db,
        input.required_margin_db(),
    );
    Ok(match el {
        Some(e) => DataThroughputLinkClosure::ClosesAbove {
            min_elevation: Qty::from_raw(e),
        },
        None => DataThroughputLinkClosure::NeverCloses,
    })
}

fn raise_min_elevation(stations: &[GroundStation], min_el: Qty<Angle>) -> Vec<GroundStation> {
    stations
        .iter()
        .map(|st| {
            let mut st = st.clone();
            st.min_elevation = Qty::from_raw(st.min_elevation.value().max(min_el.value()));
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
    let initial = input.initial_backlog().value();
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
    closure: DataThroughputLinkClosure,
    passes: Vec<DataThroughputPass>,
    sim: SimResult,
) -> Result<DataThroughputOutput, AnalysisError> {
    let total_capacity: f64 = sim.days.iter().map(|d| d.capacity.value()).sum();
    let capacity_per_day = total_capacity / (span_s / SECONDS_PER_DAY);
    let generated = input.generated_per_day.value();
    let max_backlog = sim.backlog.points.iter().map(|p| p.1).fold(0.0, f64::max);
    let checks = checks(input, closure, capacity_per_day, max_backlog)?;
    let (t, values): (Vec<f64>, Vec<f64>) = sim.backlog.points.iter().copied().unzip();
    Ok(DataThroughputOutput {
        link_closure: closure,
        required_margin_db: input.link.as_ref().map(|_| input.required_margin_db()),
        usable_pass_fraction: input.usable_pass_fraction(),
        passes,
        days: sim.days,
        capacity_per_day: Qty::from_raw(capacity_per_day),
        generated_per_day: input.generated_per_day,
        downlink_margin: ratio_margin(capacity_per_day, generated),
        max_backlog: Qty::from_raw(max_backlog),
        final_backlog: Qty::from_raw(sim.backlog.value),
        series: Series {
            t,
            channels: vec![Channel::of::<Information>(
                "backlog",
                ChannelKind::Backlog,
                values,
            )],
        },
        checks,
    })
}

fn checks(
    input: &DataThroughputInput,
    closure: DataThroughputLinkClosure,
    capacity_per_day: f64,
    max_backlog: f64,
) -> Result<Vec<Check>, AnalysisError> {
    let mut out = Vec::new();
    if closure == DataThroughputLinkClosure::NeverCloses {
        out.push(Check::new(
            "link_closes",
            "回線",
            Verdict::Violation,
            "天頂でも要求の回線余裕を満たしません",
        ));
    }
    // `value ≤ limit` の判定はバジェットの判定（`budget::limit_check`）を 1 行で使う。
    let at_most = |key: &str, label: &str, value: f64, limit: f64| {
        let row = [BudgetItem::counted(key, value, 1)];
        limit_check::<Information>(&row, limit, key, label)
    };
    let generated = input.generated_per_day.value();
    out.extend(at_most(
        "capacity",
        "送信可能量と発生量",
        generated,
        capacity_per_day,
    )?);
    if let Some(cap) = input.storage_capacity {
        out.extend(at_most(
            "storage",
            "未送信データとストレージ",
            max_backlog,
            cap.value(),
        )?);
    }
    Ok(out)
}

#[path = "data_throughput_backlog.rs"]
mod backlog;

#[cfg(test)]
#[path = "data_throughput_tests.rs"]
mod tests;
