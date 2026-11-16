//! データ送信量と蓄積（`comm.data_throughput`）。パスごとの送信可能量、1 日の送信可能量と発生量、未送信データの推移。
//!
//! - 送信機は 1 台とみなし、複数局が同時に見える時間は先に始まったパスに割り当てる（二重に数えない）。
//! - 下り回線は 1 本だけ数える（`downlink_rate` と局ごとの回線の条件 `station_links`）。ほかの下り回線は `other_downlinks` に名前だけ受け取り、数えていないと判定で知らせる。
//! - パスのうち使える時間の割合は `usable_pass_fraction`（捕捉・コマンドの時間などを差し引く）。
//! - `station_links` を与えると、回線余裕を満たす最低仰角（[`crate::comm::link`]）を局ごとに求め、その局の最低仰角に重ねて、その上だけを数える。
//! - データは一様に発生し、パス中は「発生量 + 蓄積」を上限に送る。

use crate::comm::link::{RfLinkParams, required_margin};
use crate::comm::passes::{day_windows, find_all_passes, pass_interval, search_window};
use crate::spec::{
    Analysis, AnalysisArea, AnalysisError, AnalysisKind, Channel, ChannelKind, Check, Series,
    System, ensure_fraction, ensure_non_negative, ensure_positive, ratio_margin,
};
use backlog::Backlog;
use checks::checks;
use closure::{station_closures, summarize, usable_stations};
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

/// 局に専用の回線の条件（局ごとに地上側の利得などが違うとき）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DataThroughputStationLink {
    /// 地上局の ID（`stations` のどれか）
    pub station: String,
    pub link: DataThroughputLink,
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
    /// 局ごとの回線の条件（与えた局は、回線が閉じる仰角より上だけを数える。与えない局は局の最低仰角だけで数える）
    #[serde(default)]
    pub station_links: Vec<DataThroughputStationLink>,
    /// 下り回線の帯域の設定が地上局に無いので数えない局の ID（`stations` には含めない。結果の `station_closures` に残す）
    #[serde(default)]
    pub stations_without_band: Vec<String>,
    /// 送信量に数える下り回線の名前（表示用。`downlink_rate`・`station_links` はこの回線のもの）
    #[serde(default)]
    pub downlink: Option<String>,
    /// 下り回線が複数あるとき、数えていない回線の名前（どの回線でデータを送るかを決める設定が無いため）
    #[serde(default)]
    pub other_downlinks: Vec<String>,
    /// 要求する回線余裕（`station_links` があるときに使う。既定 [`crate::comm::link::DEFAULT_REQUIRED_MARGIN`]）
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

/// 回線が閉じるか（`station_links` で回線の条件を与えた局の、回線余裕を満たす最低仰角）。
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
    /// 下り回線の帯域の設定が地上局に無い（その局は数えない。警告ではなく事実として残す）
    NoBandSettings,
}

/// 局ごとの回線が閉じる仰角。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DataThroughputStationClosure {
    pub station: String,
    pub closure: DataThroughputLinkClosure,
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
    /// 回線が閉じるか（回線の条件が無ければ `not_evaluated`。局が複数のときは、どれかの局で閉じるなら最も低い仰角、
    /// どの局でも閉じなければ `never_closes`）
    pub link_closure: DataThroughputLinkClosure,
    /// 局ごとの回線が閉じる仰角（帯域の設定が無くて数えない局も `no_band_settings` で載せる）
    pub station_closures: Vec<DataThroughputStationClosure>,
    /// 使った回線余裕の要求値（`station_links` で回線を評価したとき、つまり `link_closure` が `not_evaluated` でないときだけ）
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
    const AREA: AnalysisArea = AnalysisArea::System(System::Comm);
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
        let station_closures = station_closures(input, &w)?;
        let closure = summarize(&station_closures);
        let stations = usable_stations(input, &station_closures);
        let passes = if stations.is_empty() {
            Vec::new()
        } else {
            find_all_passes(&w.orbit, &stations, w.start, w.span_s, w.step_s)?
        };
        let rate = input.downlink_rate.value() * input.usable_pass_fraction().value();
        let (pass_results, intervals) = assign_passes(&passes, w.start, w.span_s, rate);
        let sim = simulate(input, w.start, w.span_s, rate, &intervals);
        build_output(
            input,
            w.span_s,
            (closure, station_closures),
            pass_results,
            sim,
        )
    }
}

fn validate(input: &DataThroughputInput) -> Result<(), AnalysisError> {
    ensure_positive("ダウンリンクのレート", input.downlink_rate.value())?;
    ensure_fraction("パスのうち使える割合", input.usable_pass_fraction().value())?;
    ensure_non_negative("1 日の発生量", input.generated_per_day.value())?;
    ensure_non_negative("開始時の未送信データ", input.initial_backlog().value())?;
    for s in &input.station_links {
        s.link.rf.validate("downlink")?;
    }
    Ok(())
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
    (closure, station_closures): (DataThroughputLinkClosure, Vec<DataThroughputStationClosure>),
    passes: Vec<DataThroughputPass>,
    sim: SimResult,
) -> Result<DataThroughputOutput, AnalysisError> {
    let total_capacity: f64 = sim.days.iter().map(|d| d.capacity.value()).sum();
    let capacity_per_day = total_capacity / (span_s / SECONDS_PER_DAY);
    let generated = input.generated_per_day.value();
    let max_backlog = sim.backlog.points.iter().map(|p| p.1).fold(0.0, f64::max);
    let checks = checks(
        input,
        closure,
        &station_closures,
        capacity_per_day,
        max_backlog,
    )?;
    let (t, values): (Vec<f64>, Vec<f64>) = sim.backlog.points.iter().copied().unzip();
    Ok(DataThroughputOutput {
        link_closure: closure,
        station_closures,
        required_margin_db: (closure != DataThroughputLinkClosure::NotEvaluated)
            .then(|| input.required_margin_db()),
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

mod backlog;
mod checks;
mod closure;

#[cfg(test)]
mod tests;
