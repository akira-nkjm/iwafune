//! 季節解析の代表標本からケースを作る共通入口。永続化は既存の SetCase / apply。
use crate::Workspace;
use crate::dto::CaseFromEclipseRequest;
use crate::error::{Result, ServiceError};
use fune_model::design::{Case, Design};
use fune_model::value::Quantity;
use fune_ops::Op;
use fune_projection::Context;
use sat_analysis::orbit::EclipseSeason;
use sat_analysis::spec::Analysis;

/// 最新の条件から解析し、新しいケースの `SetCase` Op を返す。既存ケースは上書きしない。
pub fn case_from_eclipse_op(ws: &Workspace, req: &CaseFromEclipseRequest) -> Result<Op> {
    let snap = ws.snapshot()?;
    validate_new_case(&snap.design, &req.id, req.mode.as_deref())?;
    let ctx = Context::new(&snap.design, &snap.resolved, None)
        .map_err(|e| ServiceError::Analysis(e.to_string()))?;
    let input = fune_projection::orbit::eclipse_season(&ctx)
        .map_err(|e| ServiceError::Analysis(e.to_string()))?;
    let output = sat_analysis::catalog::run_typed::<EclipseSeason>(&input)
        .map_err(|e| ServiceError::Analysis(e.to_string()))?;
    let point = match req.at {
        crate::dto::EclipseCaseAt::Longest => &output.longest,
        crate::dto::EclipseCaseAt::BetaMin => &output.beta_min,
    };
    // 人が読む解析ケースには度で書く（換算は sat-units の単位の正本で）
    let deg = sat_units::parse_unit(BETA_UNIT)
        .map_err(|e| ServiceError::Internal(e.to_string()))?
        .from_si(point.beta.value());
    Ok(case_op(req, point, deg))
}

/// 新しい解析ケースの ID（形式・重複）とモードの存在を調べる（既存ケースは上書きしない）。ケースを作る入口で共有する。
pub(super) fn validate_new_case(design: &Design, id: &str, mode: Option<&str>) -> Result<()> {
    fune_model::id::validate_entity_id(id)
        .map_err(|e| ServiceError::InvalidInput(e.to_string()))?;
    if design
        .cases
        .keys()
        .any(|existing| fune_model::id::same_id(existing.as_str(), id))
    {
        return Err(ServiceError::AlreadyExists {
            what: id.to_owned(),
        });
    }
    if let Some(mode) = mode
        && !design.modes.contains_key(mode)
    {
        return Err(ServiceError::not_found("mode", mode));
    }
    Ok(())
}

/// 書き出すケースの β の単位。
const BETA_UNIT: &str = "deg";

fn case_op(
    req: &CaseFromEclipseRequest,
    point: &sat_analysis::orbit::EclipseSeasonPoint,
    beta_deg: f64,
) -> Op {
    Op::SetCase {
        case: Case {
            id: req.id.as_str().into(),
            label: None,
            description: Some(format!(
                "{} / {} / {}",
                EclipseSeason::ID,
                match req.at {
                    crate::dto::EclipseCaseAt::Longest => "longest",
                    crate::dto::EclipseCaseAt::BetaMin => "beta_min",
                },
                point.epoch.to_iso()
            )),
            mode: req.mode.as_deref().map(Into::into),
            phase: None,
            side: None,
            thermal_case: None,
            beta: Some(Quantity::new(beta_deg, BETA_UNIT)),
            orbits: None,
            sun_distance: None,
        },
    }
}
