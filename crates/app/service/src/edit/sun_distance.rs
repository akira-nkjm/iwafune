//! 太陽を回る軌道の近日点・遠日点（ミッション期間の中の太陽距離の最小・最大）から解析ケースを作る共通入口。
//! 永続化は既存の SetCase / apply。ケースは太陽距離（`sun_distance`）を持ち、解析はその距離の円軌道で回す。
use super::eclipse::validate_new_case;
use crate::Workspace;
use crate::dto::{CaseFromSunDistanceRequest, SunDistanceCaseAt};
use crate::error::{Result, ServiceError};
use fune_model::design::Case;
use fune_model::value::Quantity;
use fune_ops::Op;
use fune_projection::Context;
use sat_analysis::orbit::{SunDistancePoint, sun_distance_extremes};

/// ケースに書く太陽距離の単位（天文単位）。
const DISTANCE_UNIT: &str = "AU";

/// 最新の条件から太陽距離の極値を求め、新しいケースの `SetCase` Op を返す。既存ケースは上書きしない。
pub fn case_from_sun_distance_op(ws: &Workspace, req: &CaseFromSunDistanceRequest) -> Result<Op> {
    let snap = ws.snapshot()?;
    validate_new_case(&snap.design, &req.id, req.mode.as_deref())?;
    let ctx = Context::new(&snap.design, &snap.resolved, None)
        .map_err(|e| ServiceError::Analysis(e.to_string()))?;
    let input = fune_projection::orbit::sun_distance(&ctx)
        .map_err(|e| ServiceError::Analysis(e.to_string()))?;
    let extremes =
        sun_distance_extremes(&input).map_err(|e| ServiceError::Analysis(e.to_string()))?;
    let point = match req.at {
        SunDistanceCaseAt::Min => extremes.min,
        SunDistanceCaseAt::Max => extremes.max,
    };
    // 人が読む解析ケースには AU で書く（換算は sat-units の単位の正本で）
    let au = sat_units::parse_unit(DISTANCE_UNIT)
        .map_err(|e| ServiceError::Internal(e.to_string()))?
        .from_si(point.distance.value());
    Ok(case_op(req, &point, au))
}

fn case_op(req: &CaseFromSunDistanceRequest, point: &SunDistancePoint, au: f64) -> Op {
    let at = match req.at {
        SunDistanceCaseAt::Min => "sun_distance_min",
        SunDistanceCaseAt::Max => "sun_distance_max",
    };
    Op::SetCase {
        case: Case {
            id: req.id.as_str().into(),
            label: None,
            description: Some(format!("{at} / {}", point.epoch.to_iso())),
            mode: req.mode.as_deref().map(Into::into),
            phase: None,
            side: None,
            thermal_case: None,
            beta: None,
            orbits: None,
            sun_distance: Some(Quantity::new(au, DISTANCE_UNIT)),
        },
    }
}
