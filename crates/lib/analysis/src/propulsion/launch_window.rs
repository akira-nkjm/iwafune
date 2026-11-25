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
mod chain;
#[cfg(test)]
mod flyby_tests;
mod grid;
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
    DEFAULT_FLYBY_STEPS, DEFAULT_STEPS, DEFAULT_TIME_OF_FLIGHT_FACTORS, LaunchWindowSettings,
    LaunchWindowSpan, MAX_CELLS, MAX_FLYBY_CELLS,
};
use cell::{CellValues, Evaluator};
use chain::Counts;
use grid::Grid;
pub use output::{
    LaunchWindowFlybySearch, LaunchWindowGrid, LaunchWindowLayer, LaunchWindowNote,
    LaunchWindowOutput, LaunchWindowPoint, LaunchWindowPointKind, LaunchWindowQuantity,
    LaunchWindowResult, LaunchWindowWritten,
};
use sat_astro::ephem::EphemerisSource;
use sat_astro::time::Epoch;
use sat_units::typed::Qty;
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
        flyby_dates: v.flyby_dates.clone(),
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

/// 格子の点 `(di, aj)` の値。直接の遷移は 1 回解く（`Evaluator::at`）。通過があれば、通過の日を最適にした値
/// （[`chain::best_for_cell`]。実現できる日付が無ければ `Ok(None)`）。
fn cell(
    ev: &mut Evaluator,
    counts: &mut Counts,
    (departure, arrival): (Epoch, Epoch),
) -> Result<Option<CellValues>, AnalysisError> {
    if ev.flyby_count() == 0 {
        counts.chain_solves += 1;
        ev.at(departure, arrival).map(Some)
    } else {
        chain::best_for_cell(ev, counts, (departure, arrival))
    }
}

/// 軸の全点（飛行時間が範囲に入る点）を解いて格子に入れる。解けた点がひとつも無ければ最初のエラー。
fn scan(
    ev: &mut Evaluator,
    axes: &Axes,
    counts: &mut Counts,
    maneuver: &str,
) -> Result<Grid, AnalysisError> {
    let mut grid = Grid::new(axes.departures.len(), axes.arrivals.len());
    let mut first_error = None;
    for di in 0..axes.departures.len() {
        for aj in 0..axes.arrivals.len() {
            if !axes.in_range(di, aj) {
                continue;
            }
            grid.evaluated += 1;
            match cell(ev, counts, (axes.departures[di], axes.arrivals[aj])) {
                Ok(Some(v)) => grid.record((di, aj), v),
                Ok(None) => {}
                Err(e) => {
                    first_error.get_or_insert(e);
                }
            }
        }
    }
    if grid.evaluated == 0 {
        return Err(AnalysisError::InvalidInput(format!(
            "Maneuver {maneuver}: no point of the launch window grid has a time of flight in range"
        )));
    }
    if grid.best_dv.is_none() {
        return Err(first_error.unwrap_or_else(|| {
            AnalysisError::InvalidInput(if counts.infeasible_cells > 0 {
                format!(
                    "Maneuver {maneuver}: no launch window grid point has flybys that can be flown at the minimum altitude"
                )
            } else {
                format!("Maneuver {maneuver}: no launch window grid point could be solved")
            })
        }));
    }
    Ok(grid)
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
    if ev.flyby_count() > chain::MAX_FLYBYS {
        return Err(AnalysisError::InvalidInput(format!(
            "Maneuver {}: the launch window search handles at most {} flybys ({} written)",
            leg.maneuver.id,
            chain::MAX_FLYBYS,
            ev.flyby_count()
        )));
    }
    let mut counts = Counts::default();
    let grid = scan(&mut ev, &axes, &mut counts, &leg.maneuver.id)?;
    let (Some(((dv_di, dv_aj), dv)), Some((c3_at, c3))) =
        (grid.best_dv.clone(), grid.best_c3.clone())
    else {
        return Err(AnalysisError::InvalidInput(format!(
            "Maneuver {}: no launch window grid point could be solved",
            leg.maneuver.id
        )));
    };
    let id = leg.maneuver.id.as_str();
    // 通過があれば、格子の極小の点から全部の日付を詰める（直接の遷移は格子の最小のまま）
    let (dv_dates, dv) = if ev.flyby_count() == 0 {
        ((axes.departures[dv_di], axes.arrivals[dv_aj]), dv)
    } else {
        let starts = grid
            .local_minima(chain::REFINE_STARTS, axes.departures.len())
            .into_iter()
            .map(|((di, aj), via)| (axes.departures[di], axes.arrivals[aj], via))
            .collect();
        match chain::refine_best(&mut ev, &mut counts, &axes, starts) {
            Some((d, a, v)) if v.objective_m_s <= dv.objective_m_s => ((d, a), v),
            _ => ((axes.departures[dv_di], axes.arrivals[dv_aj]), dv),
        }
    };
    let min_dv = point((id, LaunchWindowPointKind::MinDeltaV), dv_dates, &dv);
    let min_c3 = point(
        (id, LaunchWindowPointKind::MinC3),
        (axes.departures[c3_at.0], axes.arrivals[c3_at.1]),
        &c3,
    );
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
    if grid.on_edge((dv_di, dv_aj), axes.departures.len()) {
        notes.push(LaunchWindowNote::MinimumAtGridEdge);
    }
    if dv.not_ballistic {
        notes.push(LaunchWindowNote::UnpoweredFlybyNotBallistic);
    }
    if counts.refine_capped {
        notes.push(LaunchWindowNote::RefinementIncomplete);
    }
    let flyby_search = (ev.flyby_count() > 0).then(|| LaunchWindowFlybySearch {
        bodies: ev.flyby_bodies(),
        chain_solves: counts.chain_solves,
        infeasible_solves: counts.infeasible_solves,
        infeasible_cells: counts.infeasible_cells,
        refine_evaluations: counts.refine_evals,
        unpowered_residual: Qty::from_raw(dv.uncounted_flyby_dv_m_s),
    });
    let written = written.map(|w| written_result(&mut ev, id, w, (&min_dv, &min_c3), &mut notes));
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
        flyby_search,
        minimum_delta_v: Some(min_dv),
        minimum_c3: Some(min_c3),
        written,
        ephemeris,
        notes,
    })
}

/// マヌーバに書かれた日付（通過の日も書かれたもの）での値と、最小との差。
fn written_result(
    ev: &mut Evaluator,
    id: &str,
    w: &DatedTransferInput,
    (min_dv, min_c3): (&LaunchWindowPoint, &LaunchWindowPoint),
    notes: &mut Vec<LaunchWindowNote>,
) -> LaunchWindowWritten {
    let via = ev.written_flyby_dates();
    let solved = ev.at_via(w.departure, &via, w.arrival).ok();
    if solved.is_none() {
        notes.push(LaunchWindowNote::WrittenDatesUnsolved);
    } else if solved.as_ref().is_some_and(|v| !v.feasible) {
        notes.push(LaunchWindowNote::WrittenFlybyInfeasible);
    }
    let p = solved.map(|v| {
        point(
            (id, LaunchWindowPointKind::Written),
            (w.departure, w.arrival),
            &v,
        )
    });
    let above = |f: &dyn Fn(&LaunchWindowPoint) -> f64| p.as_ref().map(f);
    LaunchWindowWritten {
        delta_v_above_minimum: above(&|p| p.delta_v.value() - min_dv.delta_v.value())
            .map(Qty::from_raw),
        c3_above_minimum: above(&|p| p.c3.value() - min_c3.c3.value()).map(Qty::from_raw),
        point: p,
    }
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
