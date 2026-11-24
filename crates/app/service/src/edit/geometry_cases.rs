//! 最悪の日からケースを作る共通入口（`fune case from-geometry`・`POST /ops/cases_from_geometry`）。
//! フェーズの軌道のミッション幾何（`orbit.geometry`。出どころ・周期の有無を問わない）が出す、太陽距離の最大・最小と地球距離の最大の日付を、
//! 解析ケースの `epoch` にする。距離と日付の計算はミッション幾何の 1 か所（`sat_astro::timeline::geometry`）で、ここは区間の選択と
//! ケースの名前・書き方だけ。永続化は既存の `SetCase` Op（作る・更新するの両方）で、名前は決まっているので何度実行しても同じファイルになる。
use crate::Workspace;
use crate::dto::CaseFromGeometryRequest;
use crate::error::{Result, ServiceError};
use fune_model::design::{Case, Design};
use fune_ops::Op;
use fune_projection::Context;
use sat_analysis::orbit::{Geometry, GeometryLeg};
use sat_analysis::spec::Analysis;
use sat_astro::time::Epoch;

/// 作るケース: (ID の後ろ、説明に書く量)。順序は固定（出力の順序）。
const WORST_DAYS: [(&str, &str); 3] = [
    ("sun_far", "sun distance max"),
    ("sun_near", "sun distance min"),
    ("earth_far", "earth distance max"),
];

/// 最新の条件から幾何を解析し、最悪の日の `SetCase` Op を返す（太陽距離の最大・最小・地球距離の最大の 3 つ）。
/// 同じ ID のケースが既にあれば、`epoch`・`phase`・説明（と、要求にあればモード）だけを書き換え、ほかの欄は残す。
pub fn cases_from_geometry_ops(ws: &Workspace, req: &CaseFromGeometryRequest) -> Result<Vec<Op>> {
    let snap = ws.snapshot()?;
    let design = &snap.design;
    check_request(design, req)?;
    let ctx = Context::new(design, &snap.resolved, None)
        .map_err(|e| ServiceError::Analysis(e.to_string()))?;
    let input = fune_projection::orbit::geometry(&ctx)
        .map_err(|e| ServiceError::Analysis(e.to_string()))?;
    let output = Geometry::run(&input).map_err(|e| ServiceError::Analysis(e.to_string()))?;
    let leg = pick_leg(design, &output.legs, req.phase.as_deref())?;
    let days = [
        leg.sun_distance.max.epoch,
        leg.sun_distance.min.epoch,
        leg.earth_distance.max.epoch,
    ];
    WORST_DAYS
        .iter()
        .zip(days)
        .map(|(&(suffix, what), epoch)| {
            let id = case_id(req, suffix);
            case_op(design, req, &id, what, epoch)
        })
        .collect()
}

/// 要求の形（前置き・フェーズ・モード）を調べる。
fn check_request(design: &Design, req: &CaseFromGeometryRequest) -> Result<()> {
    if let Some(phase) = req.phase.as_deref()
        && !design.phases.contains_key(phase)
    {
        return Err(ServiceError::not_found("phase", phase));
    }
    if let Some(mode) = req.mode.as_deref()
        && !design.modes.contains_key(mode)
    {
        return Err(ServiceError::not_found("mode", mode));
    }
    for (suffix, _) in WORST_DAYS {
        fune_model::id::validate_entity_id(&case_id(req, suffix))
            .map_err(|e| ServiceError::InvalidInput(e.to_string()))?;
    }
    Ok(())
}

/// ケースの ID: `<前置き>_<後ろ>`（前置きは要求、無ければフェーズの ID の小文字、それも無ければ後ろだけ）。
fn case_id(req: &CaseFromGeometryRequest, suffix: &str) -> String {
    let prefix = req
        .prefix
        .clone()
        .or_else(|| req.phase.as_ref().map(|p| p.to_lowercase()));
    match prefix {
        Some(prefix) => format!("{prefix}_{suffix}"),
        None => suffix.to_owned(),
    }
}

/// 読む区間: 要求のフェーズの区間（フェーズが無い設計は設計の軌道の 1 区間）。
fn pick_leg<'a>(
    design: &Design,
    legs: &'a [GeometryLeg],
    phase: Option<&str>,
) -> Result<&'a GeometryLeg> {
    legs.iter()
        .find(|l| l.phase.as_deref() == phase)
        .ok_or_else(|| {
            let known: Vec<&str> = design.phases.keys().map(|p| p.as_str()).collect();
            ServiceError::InvalidInput(format!(
                "Choose the phase to read (--phase): the design has phases ({})",
                known.join(", ")
            ))
        })
}

/// 1 つのケースの Op。ケースが既にあれば、その `epoch`・`phase`・説明を書き換える（大文字小文字だけ違う ID は重複として拒否する）。
fn case_op(
    design: &Design,
    req: &CaseFromGeometryRequest,
    id: &str,
    what: &str,
    epoch: Epoch,
) -> Result<Op> {
    let existing = design.cases.get(id);
    let clash = design
        .cases
        .keys()
        .any(|k| existing.is_none() && fune_model::id::same_id(k.as_str(), id));
    if clash {
        return Err(ServiceError::AlreadyExists {
            what: id.to_owned(),
        });
    }
    let mut case = existing.map_or_else(
        || Case {
            id: id.into(),
            label: None,
            description: None,
            mode: None,
            phase: None,
            side: None,
            thermal_case: None,
            beta: None,
            orbits: None,
            sun_distance: None,
            epoch: None,
        },
        Case::clone,
    );
    case.description = Some(format!("orbit.geometry / {what} / {}", epoch.to_iso()));
    case.epoch = Some(epoch.to_iso());
    case.phase = req.phase.as_deref().map(Into::into);
    if let Some(mode) = req.mode.as_deref() {
        case.mode = Some(mode.into());
    }
    Ok(Op::SetCase { case })
}
