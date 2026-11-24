//! 読み取りの API: `/project`・`/design`・`/parts`・`/parts/{id}`（POST は下書きを当てる）・`/values/{path}`・`/defs`・
//! `/diagnostics`。

use super::{ApiJson, ApiQuery, AppState, blocking};
use crate::error::ApiError;
use axum::Json;
use axum::extract::{Path, State};
use fune_service::dto::{
    CheckReport, DefsView, DesignView, PartQuery, PartRow, PartView, PartViewRequest, PartsQuery,
    ProjectInfo, SystemModesRequest, ValueAnswer, ValueQuery,
};
use fune_service::query;

/// GET `/project`
pub async fn project(State(s): State<AppState>) -> Result<Json<ProjectInfo>, ApiError> {
    blocking(&s, query::project_info).await.map(Json)
}

/// GET `/design`
pub async fn design(State(s): State<AppState>) -> Result<Json<DesignView>, ApiError> {
    blocking(&s, query::design_view).await.map(Json)
}

/// GET `/maneuvers`（実行の順。`fune maneuver ls`）
pub async fn maneuvers(
    State(s): State<AppState>,
) -> Result<Json<Vec<fune_service::dto::ManeuverView>>, ApiError> {
    blocking(&s, query::maneuvers).await.map(Json)
}

/// GET `/phases`（並びの順。`fune phase ls|show`）
pub async fn phases(
    State(s): State<AppState>,
) -> Result<Json<Vec<fune_service::dto::PhaseView>>, ApiError> {
    blocking(&s, query::phases).await.map(Json)
}

/// GET `/configurations`（ID 順。`fune configuration ls|show`）
pub async fn configurations(
    State(s): State<AppState>,
) -> Result<Json<Vec<fune_service::dto::ConfigurationView>>, ApiError> {
    blocking(&s, query::configurations).await.map(Json)
}

/// GET `/requirements`（ID 順。`fune requirement ls`）
pub async fn requirements(
    State(s): State<AppState>,
) -> Result<Json<Vec<fune_service::dto::RequirementView>>, ApiError> {
    blocking(&s, query::requirements).await.map(Json)
}

/// GET `/requirements/compliance`（要求適合表。`fune requirement compliance`）
pub async fn compliance(
    State(s): State<AppState>,
) -> Result<Json<fune_service::dto::ComplianceReport>, ApiError> {
    blocking(&s, fune_service::requirements::compliance)
        .await
        .map(Json)
}

/// GET `/parts?group=aocs&kind=reaction_wheel&configuration=STOWED`
pub async fn parts(
    State(s): State<AppState>,
    ApiQuery(q): ApiQuery<PartsQuery>,
) -> Result<Json<Vec<PartRow>>, ApiError> {
    blocking(&s, move |ws| query::list_parts(ws, &q))
        .await
        .map(Json)
}

/// GET `/parts/{id}?configuration=STOWED`
pub async fn part(
    State(s): State<AppState>,
    Path(id): Path<String>,
    ApiQuery(q): ApiQuery<PartQuery>,
) -> Result<Json<PartView>, ApiError> {
    blocking(&s, move |ws| {
        let req = PartViewRequest {
            configuration: q.configuration,
            ..PartViewRequest::default()
        };
        query::part_view(ws, &id, &req)
    })
    .await
    .map(Json)
}

/// POST `/parts/{id}` `{ ops }`: 保存していない編集を当てた設計の部品（書き込まない）
pub async fn part_with(
    State(s): State<AppState>,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<PartViewRequest>,
) -> Result<Json<PartView>, ApiError> {
    blocking(&s, move |ws| query::part_view(ws, &id, &req))
        .await
        .map(Json)
}

/// GET `/values/{path}?state=on`
pub async fn value(
    State(s): State<AppState>,
    Path(path): Path<String>,
    ApiQuery(q): ApiQuery<ValueQuery>,
) -> Result<Json<ValueAnswer>, ApiError> {
    blocking(&s, move |ws| query::get_value(ws, &path, &q))
        .await
        .map(Json)
}

/// GET `/defs`
pub async fn defs(State(s): State<AppState>) -> Result<Json<DefsView>, ApiError> {
    blocking(&s, query::defs_view).await.map(Json)
}

/// GET `/diagnostics`
pub async fn diagnostics(State(s): State<AppState>) -> Result<Json<CheckReport>, ApiError> {
    blocking(&s, query::check).await.map(Json)
}

/// POST `/system_modes/{system}`: 保存していない編集も同じ経路で評価する。
pub async fn system_modes(
    State(s): State<AppState>,
    Path(system): Path<String>,
    ApiJson(req): ApiJson<SystemModesRequest>,
) -> Result<Json<fune_service::dto::SystemModesView>, ApiError> {
    blocking(&s, move |ws| {
        let system = fune_service::system_modes::parse_system(&system)?;
        fune_service::system_modes::system_modes(ws, system, &req)
    })
    .await
    .map(Json)
}
