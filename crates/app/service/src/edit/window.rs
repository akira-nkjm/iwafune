//! 日付の探索（`propulsion.launch_window`）の最小の点から、マヌーバの出発・到着の日付を決める共通入口。
//! 日付を書く Op は既存の `SetManeuver`（`fune maneuver set … --depart … --arrive …` と同じ書き方）で、
//! CLI（`fune maneuver set-window`）と HTTP（`POST /ops/maneuver_window`）はどちらもこの関数を呼ぶ。
use crate::Workspace;
use crate::dto::{ManeuverWindowRequest, WindowObjective};
use crate::error::{Result, ServiceError};
use fune_model::design::ManeuverTransfer;
use fune_ops::Op;
use fune_projection::Context;
use sat_analysis::propulsion::LaunchWindow;
use sat_analysis::spec::Analysis;

/// 最新の設計から日付を探索し、指定のマヌーバの `transfer` を最小の点の日付にした `SetManeuver` Op を返す
/// （マヌーバのほかの欄は今のまま）。マヌーバが無い・天体間の遷移でない・格子に解けた点が無いときはエラー。
pub fn maneuver_window_op(ws: &Workspace, req: &ManeuverWindowRequest) -> Result<Op> {
    let snap = ws.snapshot()?;
    let maneuver = snap
        .design
        .maneuvers
        .get(req.id.as_str())
        .ok_or_else(|| ServiceError::not_found("maneuver", &req.id))?;
    let ctx = Context::new(&snap.design, &snap.resolved, None)
        .map_err(|e| ServiceError::Analysis(e.to_string()))?;
    let input = fune_projection::propulsion::launch_window(&ctx)
        .map_err(|e| ServiceError::Analysis(e.to_string()))?;
    let output = sat_analysis::catalog::run_typed::<LaunchWindow>(&input)
        .map_err(|e| ServiceError::Analysis(e.to_string()))?;
    let window = output
        .windows
        .iter()
        .find(|w| w.maneuver == req.id)
        .ok_or_else(|| {
            ServiceError::InvalidInput(format!(
                "Maneuver {} is not a transfer between bodies ({} has nothing to search)",
                req.id,
                LaunchWindow::ID
            ))
        })?;
    let point = match req.min {
        WindowObjective::Dv => window.minimum_delta_v.as_ref(),
        WindowObjective::C3 => window.minimum_c3.as_ref(),
    }
    .ok_or_else(|| ServiceError::Analysis(format!("No solved point for maneuver {}", req.id)))?;
    let mut maneuver = maneuver.clone();
    maneuver.transfer = Some(ManeuverTransfer {
        departure: Some(point.departure.to_iso()),
        arrival: Some(point.arrival.to_iso()),
    });
    Ok(Op::SetManeuver { maneuver })
}
