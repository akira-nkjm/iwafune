//! 日付の探索（`propulsion.launch_window`）。天体間の遷移を持つマヌーバごとに、出発の日 × 到着の日の格子で
//! C3・両端の超過速度・出発の赤緯・マヌーバの ΔV（出発の脱出 + 到着の捕獲）を求め、最小の点と書かれた日付との差を出す
//! （ポークチョッププロットの元の数値）。
//!
//! - **格子の点は `propulsion.maneuvers` と同じ経路で解く**（[`cell`]: マヌーバの `transfer` に日付を入れて `plan` を呼ぶ。
//!   Lambert の設定も ΔV の式も持たない）。そのためその日付をマヌーバに書けば `propulsion.maneuvers` の値と一致する。
//!   ΔV は損失を除いた値（`ManeuverResult.ideal_delta_v`）。
//! - 駐機軌道は、マヌーバの前の軌道（前のマヌーバの結果。日付を外した公称の列から取る。捕獲の軌道は日付によらない）。
//! - 軸と既定値は [`axes`]（出発の窓 = 会合周期、飛行時間 = ホーマンの 0.5〜1.5 倍）。飛行時間が範囲に入る点だけを解く。
//! - 解けない点（暦の外・出発と到着がほぼ正反対で面が決まらない・双曲線が成り立たないなど）は `None`（作った数値は出さない）。
//!   解いた点がひとつも無ければ最初のエラーを返す（`propulsion.maneuvers` と同じ誤りの扱い）。
//! - 天体間の遷移を持つマヌーバが無ければ `windows` は空（`propulsion.transfer_check` と同じ）。
//! - 日付つきの遷移を適用する先は `fune maneuver set-window`（service の `edit::window`）。

mod axes;
mod cell;
mod output;
#[cfg(test)]
mod tests;

use super::input::{DatedTransferInput, PropulsionInput};
use super::maneuvers::nominal;
use super::plan::ManeuverNote;
use super::sequence::Leg;
use crate::spec::{Analysis, AnalysisArea, AnalysisError, AnalysisKind, System};
use axes::Axes;
pub use axes::{
    DEFAULT_STEPS, DEFAULT_TIME_OF_FLIGHT_FACTORS, LaunchWindowSettings, LaunchWindowSpan,
    MAX_CELLS,
};
use cell::{CellValues, Evaluator};
pub use output::{
    LaunchWindowGrid, LaunchWindowLayer, LaunchWindowNote, LaunchWindowOutput, LaunchWindowPoint,
    LaunchWindowPointKind, LaunchWindowQuantity, LaunchWindowResult, LaunchWindowWritten,
};
use sat_astro::ephem::EphemerisSource;
use sat_astro::time::Epoch;
use sat_units::typed::{Angle, NamedKind, Qty, SpecificEnergy, Velocity};
use serde::{Deserialize, Serialize};

/// `propulsion.launch_window` の入力: 推進系の入力（`propulsion.maneuvers` と同じ）と探索の設定。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LaunchWindowInput {
    pub propulsion: PropulsionInput,
    #[serde(default)]
    pub settings: LaunchWindowSettings,
}

fn point(
    (maneuver, kind): (&str, LaunchWindowPointKind),
    (departure, arrival): (Epoch, Epoch),
    v: &CellValues,
) -> LaunchWindowPoint {
    LaunchWindowPoint {
        maneuver: maneuver.to_owned(),
        kind,
        departure,
        arrival,
        time_of_flight: Qty::from_raw(v.time_of_flight_s),
        c3: Qty::from_raw(v.c3_m2_s2),
        v_inf_departure: Qty::from_raw(v.v_inf_departure_m_s),
        v_inf_arrival: Qty::from_raw(v.v_inf_arrival_m_s),
        departure_declination: Qty::from_raw(v.declination_rad),
        delta_v: Qty::from_raw(v.delta_v_m_s),
    }
}

/// 格子の値（出発 × 到着の順）と、解いた点の集計。
struct Grid {
    n_arr: usize,
    c3: Vec<Option<f64>>,
    v_dep: Vec<Option<f64>>,
    v_arr: Vec<Option<f64>>,
    declination: Vec<Option<f64>>,
    delta_v: Vec<Option<f64>>,
    evaluated: u32,
    solved: u32,
    all_de440: bool,
    /// ΔV 最小・C3 最小の点（添字と値。同じ値なら先に解いた点）
    best_dv: Option<((usize, usize), CellValues)>,
    best_c3: Option<((usize, usize), CellValues)>,
}

impl Grid {
    fn new(n_dep: usize, n_arr: usize) -> Self {
        let blank = || vec![None; n_dep * n_arr];
        Self {
            n_arr,
            c3: blank(),
            v_dep: blank(),
            v_arr: blank(),
            declination: blank(),
            delta_v: blank(),
            evaluated: 0,
            solved: 0,
            all_de440: true,
            best_dv: None,
            best_c3: None,
        }
    }

    fn record(&mut self, (i, j): (usize, usize), v: CellValues) {
        let k = i * self.n_arr + j;
        self.c3[k] = Some(v.c3_m2_s2);
        self.v_dep[k] = Some(v.v_inf_departure_m_s);
        self.v_arr[k] = Some(v.v_inf_arrival_m_s);
        self.declination[k] = Some(v.declination_rad);
        self.delta_v[k] = Some(v.delta_v_m_s);
        self.solved += 1;
        self.all_de440 &= v.ephemeris == EphemerisSource::De440;
        if self
            .best_dv
            .is_none_or(|(_, b)| v.delta_v_m_s < b.delta_v_m_s)
        {
            self.best_dv = Some(((i, j), v));
        }
        if self.best_c3.is_none_or(|(_, b)| v.c3_m2_s2 < b.c3_m2_s2) {
            self.best_c3 = Some(((i, j), v));
        }
    }

    /// 出発 `i`・到着 `j` の 4 近傍のどれかが格子の外か、解けていない点か。
    fn on_edge(&self, (i, j): (usize, usize), n_dep: usize) -> bool {
        let solved = |i: Option<usize>, j: Option<usize>| match (i, j) {
            (Some(i), Some(j)) if i < n_dep && j < self.n_arr => {
                self.delta_v[i * self.n_arr + j].is_some()
            }
            _ => false,
        };
        !(solved(i.checked_sub(1), Some(j))
            && solved(Some(i + 1), Some(j))
            && solved(Some(i), j.checked_sub(1))
            && solved(Some(i), Some(j + 1)))
    }

    fn layers(&self) -> Vec<LaunchWindowLayer> {
        use LaunchWindowQuantity as Q;
        let layer = |kind, dimension: &str, values: &Vec<Option<f64>>| LaunchWindowLayer {
            kind,
            dimension: dimension.to_owned(),
            values: values.clone(),
        };
        vec![
            layer(Q::DeltaV, Velocity::name(), &self.delta_v),
            layer(Q::C3, SpecificEnergy::name(), &self.c3),
            layer(Q::VInfDeparture, Velocity::name(), &self.v_dep),
            layer(Q::VInfArrival, Velocity::name(), &self.v_arr),
            layer(Q::DepartureDeclination, Angle::name(), &self.declination),
        ]
    }
}

/// 天体間（兄弟の天体）の遷移を持つマヌーバか。日付の有無は問わない（日付を外した公称の列の区間を渡す）。
fn is_interplanetary(leg: &Leg<'_>) -> bool {
    let (from, to) = (leg.planned.from.central, leg.planned.to.central);
    leg.planned.notes.contains(&ManeuverNote::PatchedConic)
        && from != to
        && from.parent().is_some()
        && from.parent() == to.parent()
}

/// 区間（日付を外した公称の列）の探索。`written` は入力のマヌーバに書かれた日付。
fn search(
    leg: &Leg<'_>,
    input: &LaunchWindowInput,
    written: Option<&DatedTransferInput>,
    i: usize,
) -> Result<LaunchWindowResult, AnalysisError> {
    let (from, to) = (leg.planned.from.central, leg.planned.to.central);
    let axes = Axes::new(
        &input.settings,
        (from, leg.planned.from.apsides, to),
        written,
        input.propulsion.initial_orbit.epoch,
    )?;
    let mut ev = Evaluator::new(leg.planned.from, &input.propulsion.maneuvers[i]);
    let (n_dep, n_arr) = (axes.departures.len(), axes.arrivals.len());
    let mut grid = Grid::new(n_dep, n_arr);
    let mut first_error = None;
    for di in 0..n_dep {
        for aj in 0..n_arr {
            if !axes.in_range(di, aj) {
                continue;
            }
            grid.evaluated += 1;
            match ev.at(axes.departures[di], axes.arrivals[aj]) {
                Ok(v) => grid.record((di, aj), v),
                Err(e) => {
                    first_error.get_or_insert(e);
                }
            }
        }
    }
    if grid.evaluated == 0 {
        return Err(AnalysisError::InvalidInput(format!(
            "Maneuver {}: no point of the launch window grid has a time of flight in range",
            leg.maneuver.id
        )));
    }
    let (Some((dv_at, dv)), Some((c3_at, c3))) = (grid.best_dv, grid.best_c3) else {
        return Err(first_error.unwrap_or_else(|| {
            AnalysisError::InvalidInput(format!(
                "Maneuver {}: no launch window grid point could be solved",
                leg.maneuver.id
            ))
        }));
    };
    let id = leg.maneuver.id.as_str();
    let at = |kind, (di, aj): (usize, usize), v: &CellValues| {
        point((id, kind), (axes.departures[di], axes.arrivals[aj]), v)
    };
    let min_dv = at(LaunchWindowPointKind::MinDeltaV, dv_at, &dv);
    let min_c3 = at(LaunchWindowPointKind::MinC3, c3_at, &c3);
    let mut notes = Vec::new();
    let ephemeris = if grid.all_de440 {
        EphemerisSource::De440
    } else {
        notes.push(LaunchWindowNote::AnalyticEphemeris);
        EphemerisSource::Analytic
    };
    if dv.declination_rad.abs() > leg.planned.from.inclination_rad {
        notes.push(LaunchWindowNote::DeclinationAboveInclination);
    }
    if grid.on_edge(dv_at, n_dep) {
        notes.push(LaunchWindowNote::MinimumAtGridEdge);
    }
    let written = written.map(|w| {
        let p = ev.at(w.departure, w.arrival).ok().map(|v| {
            point(
                (id, LaunchWindowPointKind::Written),
                (w.departure, w.arrival),
                &v,
            )
        });
        if p.is_none() {
            notes.push(LaunchWindowNote::WrittenDatesUnsolved);
        }
        let above = |f: &dyn Fn(&LaunchWindowPoint) -> f64| p.as_ref().map(f);
        LaunchWindowWritten {
            delta_v_above_minimum: above(&|p| p.delta_v.value() - min_dv.delta_v.value())
                .map(Qty::from_raw),
            c3_above_minimum: above(&|p| p.c3.value() - min_c3.c3.value()).map(Qty::from_raw),
            point: p,
        }
    });
    Ok(LaunchWindowResult {
        maneuver: leg.maneuver.id.clone(),
        from,
        to,
        span: axes.span,
        synodic_period: Qty::from_raw(axes.synodic_s),
        hohmann_time: Qty::from_raw(axes.hohmann_s),
        grid: LaunchWindowGrid {
            layers: grid.layers(),
            departures: axes.departures,
            arrivals: axes.arrivals,
        },
        evaluated_cells: grid.evaluated,
        solved_cells: grid.solved,
        minimum_delta_v: Some(min_dv),
        minimum_c3: Some(min_c3),
        written,
        ephemeris,
        notes,
    })
}

/// 日付の探索（出発の日 × 到着の日）。
pub struct LaunchWindow;

impl Analysis for LaunchWindow {
    const ID: &'static str = "propulsion.launch_window";
    const TITLE: &'static str = "打ち上げの窓（日付の探索）";
    const TITLE_EN: &'static str = "Launch window (date search)";
    const AREA: AnalysisArea = AnalysisArea::System(System::Propulsion);
    const KIND: AnalysisKind = AnalysisKind::Steady;
    const DESCRIPTION: &'static str = "惑星間の遷移を持つマヌーバごとに、出発の日 × 到着の日の格子で C3・超過速度・赤緯・ΔV を求め、最小の点と書かれた日付との差を出す（ポークチョップの元の数値）";
    const DESCRIPTION_EN: &'static str = "For each maneuver with an interplanetary transfer, solves a departure date x arrival date grid for C3, excess speeds, declination and delta-V, and reports the minimum points and the gap of the written dates (the data of a porkchop plot)";

    type Input = LaunchWindowInput;
    type Output = LaunchWindowOutput;

    fn run(input: &Self::Input) -> Result<Self::Output, AnalysisError> {
        // 日付を探索するのは出発と到着の 2 つ。途中の通過（スイングバイ）の日は探索しないので、断る
        if let Some(m) = input
            .propulsion
            .maneuvers
            .iter()
            .find(|m| m.transfer.as_ref().is_some_and(|t| !t.flybys.is_empty()))
        {
            return Err(AnalysisError::InvalidInput(format!(
                "Maneuver {}: the launch window search covers the two dates of a transfer without flybys; it does not search the dates of a swing-by transfer (remove the flybys to search, or set the dates by hand)",
                m.id
            )));
        }
        // マヌーバの前の軌道は日付によらないので、日付を外した公称の列から取る（日付が不正でも探索はできる）
        let mut undated = input.propulsion.clone();
        undated.maneuvers.iter_mut().for_each(|m| m.transfer = None);
        let seq = nominal(&undated)?;
        let windows = seq
            .legs
            .iter()
            .enumerate()
            .filter(|(_, leg)| is_interplanetary(leg))
            .map(|(i, leg)| {
                search(
                    leg,
                    input,
                    input.propulsion.maneuvers[i].transfer.as_ref(),
                    i,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let points = windows.iter().flat_map(|w| w.points().cloned()).collect();
        Ok(LaunchWindowOutput { windows, points })
    }
}
