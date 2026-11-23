//! 日付の探索の格子の軸（出発の日・到着の日）と、その既定値の決め方。
//!
//! 既定値は天体の組から導く（惑星ごとの定数は持たない）:
//! - 出発の窓の長さ: 2 天体の会合周期（[`CentralBody::synodic_period_s`]）。
//! - 出発の窓の始め: 元期（最初の軌道の元期）。マヌーバに日付が書かれていれば、書かれた出発の日を窓の真ん中に置く。
//! - 飛行時間: ホーマン遷移の時間（`patched_conic` の `transfer_time_s`。日付なしのマヌーバが使う値）の 0.5〜1.5 倍。
//! - 格子の点の数: 出発・到着の軸ごとに [`DEFAULT_STEPS`]（解くのは飛行時間が範囲に入る点だけなので、実行時間は天体の組によらない）。
//!
//! 軸の日付は元期から整数秒の位置に丸める（書き出した日付を読み直すと同じ元期になり、`propulsion.maneuvers` と値が一致する）。

use super::super::input::DatedTransferInput;
use crate::spec::AnalysisError;
use sat_astro::ephem::CentralBody;
use sat_astro::time::Epoch;
use sat_astro::transfer::{Apsides, patched_conic};
use sat_units::typed::{Qty, Time};
use serde::{Deserialize, Serialize};

/// 飛行時間の下限・上限の、ホーマン遷移の時間に対する既定の倍率。
pub const DEFAULT_TIME_OF_FLIGHT_FACTORS: (f64, f64) = (0.5, 1.5);

/// 出発・到着の軸ごとの既定の点の数（8 000 点ほどの格子で、解く点は飛行時間の範囲に入る 2 000 点ほど。実測は `launch_window_tests`）。
pub const DEFAULT_STEPS: u32 = 100;

/// 格子の点の数の下限（軸ごと）。
const MIN_STEPS: u32 = 2;

/// 格子の全体の点の数の上限（これを超える指定は誤り。実行時間と出力の大きさを抑える）。
pub const MAX_CELLS: u64 = 250_000;

/// 探索の設定（`analysis/propulsion.toml` の `[launch_window]`）。どれも任意で、既定値は上の規則で決める。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchWindowSettings {
    /// 出発の窓の始め
    #[serde(default)]
    pub departure_start: Option<Epoch>,
    /// 出発の窓の長さ
    #[serde(default)]
    pub departure_span: Option<Qty<Time>>,
    /// 飛行時間の下限
    #[serde(default)]
    pub time_of_flight_min: Option<Qty<Time>>,
    /// 飛行時間の上限
    #[serde(default)]
    pub time_of_flight_max: Option<Qty<Time>>,
    /// 出発の軸の点の数
    #[serde(default)]
    pub departure_steps: Option<u32>,
    /// 到着の軸の点の数
    #[serde(default)]
    pub arrival_steps: Option<u32>,
}

/// 決まった探索の範囲（結果にも載せる）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchWindowSpan {
    pub departure_start: Epoch,
    pub departure_end: Epoch,
    pub time_of_flight_min: Qty<Time>,
    pub time_of_flight_max: Qty<Time>,
    pub departure_steps: u32,
    pub arrival_steps: u32,
}

/// 格子の 2 つの軸。
#[derive(Debug, Clone)]
pub(super) struct Axes {
    pub span: LaunchWindowSpan,
    pub departures: Vec<Epoch>,
    pub arrivals: Vec<Epoch>,
    /// 2 天体の会合周期・ホーマン遷移の時間（結果の参考値）
    pub synodic_s: f64,
    pub hohmann_s: f64,
}

fn whole_seconds(s: f64) -> f64 {
    s.round()
}

fn positive(name: &str, q: Option<Qty<Time>>) -> Result<Option<f64>, AnalysisError> {
    match q.map(|q| q.value()) {
        Some(v) if !(v.is_finite() && v > 0.0) => Err(AnalysisError::InvalidInput(format!(
            "Launch window {name} must be a positive finite time ({v} s)"
        ))),
        other => Ok(other),
    }
}

fn steps(name: &str, n: Option<u32>) -> Result<u32, AnalysisError> {
    let n = n.unwrap_or(DEFAULT_STEPS);
    if n < MIN_STEPS {
        return Err(AnalysisError::InvalidInput(format!(
            "Launch window {name} must be at least {MIN_STEPS} ({n})"
        )));
    }
    Ok(n)
}

/// 軸の点（始めから `length_s` まで `n` 点、整数秒に丸める）。
fn axis(start: Epoch, length_s: f64, n: u32) -> Vec<Epoch> {
    let last = f64::from(n - 1);
    (0..n)
        .map(|i| start.plus_seconds(whole_seconds(length_s * f64::from(i) / last)))
        .collect()
}

impl Axes {
    /// 天体 `from` の軌道 `parking` から `to` への探索の軸。`written` はマヌーバに書かれた日付、`epoch` は最初の軌道の元期。
    pub fn new(
        settings: &LaunchWindowSettings,
        (from, parking, to): (CentralBody, Apsides, CentralBody),
        written: Option<DatedTransferInput>,
        epoch: Epoch,
    ) -> Result<Self, AnalysisError> {
        let synodic_s = from.synodic_period_s(to).ok_or_else(|| {
            AnalysisError::InvalidInput(format!(
                "Launch window search needs two bodies orbiting the same parent ({from:?}, {to:?})"
            ))
        })?;
        let hohmann_s = patched_conic(from, parking, to, None)?.transfer_time_s;
        let dep_span = whole_seconds(
            positive("departure span", settings.departure_span)?.unwrap_or(synodic_s),
        );
        let (f_min, f_max) = DEFAULT_TIME_OF_FLIGHT_FACTORS;
        let tof_min = whole_seconds(
            positive("minimum time of flight", settings.time_of_flight_min)?
                .unwrap_or(f_min * hohmann_s),
        );
        let tof_max = whole_seconds(
            positive("maximum time of flight", settings.time_of_flight_max)?
                .unwrap_or(f_max * hohmann_s),
        );
        if tof_min >= tof_max {
            return Err(AnalysisError::InvalidInput(format!(
                "Launch window minimum time of flight ({tof_min} s) must be below the maximum ({tof_max} s)"
            )));
        }
        let start = settings
            .departure_start
            .or_else(|| written.map(|w| w.departure.plus_seconds(-0.5 * dep_span)))
            .unwrap_or(epoch);
        let start = Epoch {
            j2000_s: whole_seconds(start.j2000_s),
        };
        let (n_dep, n_arr) = (
            steps("departure steps", settings.departure_steps)?,
            steps("arrival steps", settings.arrival_steps)?,
        );
        if u64::from(n_dep) * u64::from(n_arr) > MAX_CELLS {
            return Err(AnalysisError::InvalidInput(format!(
                "Launch window grid has {n_dep} x {n_arr} points, above the limit of {MAX_CELLS}"
            )));
        }
        let departures = axis(start, dep_span, n_dep);
        let arrivals = axis(
            start.plus_seconds(tof_min),
            dep_span + (tof_max - tof_min),
            n_arr,
        );
        Ok(Self {
            span: LaunchWindowSpan {
                departure_start: start,
                departure_end: start.plus_seconds(dep_span),
                time_of_flight_min: Qty::from_raw(tof_min),
                time_of_flight_max: Qty::from_raw(tof_max),
                departure_steps: n_dep,
                arrival_steps: n_arr,
            },
            departures,
            arrivals,
            synodic_s,
            hohmann_s,
        })
    }

    /// 出発 `i`・到着 `j` の組の飛行時間 [s]。
    pub fn time_of_flight_s(&self, i: usize, j: usize) -> f64 {
        self.arrivals[j].seconds_since(self.departures[i])
    }

    /// 飛行時間が範囲に入るか（入る組だけ解く）。
    pub fn in_range(&self, i: usize, j: usize) -> bool {
        let t = self.time_of_flight_s(i, j);
        t >= self.span.time_of_flight_min.value() && t <= self.span.time_of_flight_max.value()
    }
}
