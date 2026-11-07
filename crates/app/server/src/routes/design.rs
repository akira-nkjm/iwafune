//! 読み取りの API: `/project`・`/design`・`/parts`・`/parts/{id}`・`/values/{path}`・`/defs`・`/diagnostics`。

use super::{ApiQuery, AppState, blocking};
use crate::error::ApiError;
use axum::Json;
use axum::extract::{Path, State};
use fune_service::dto::{
    CheckReport, DefsView, DesignView, PartRow, PartView, PartsQuery, ProjectInfo, ValueAnswer,
    ValueQuery,
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

/// GET `/parts?group=AOCS&kind=reaction_wheel`
pub async fn parts(
    State(s): State<AppState>,
    ApiQuery(q): ApiQuery<PartsQuery>,
) -> Result<Json<Vec<PartRow>>, ApiError> {
    blocking(&s, move |ws| query::list_parts(ws, &q))
        .await
        .map(Json)
}

/// GET `/parts/{id}`
pub async fn part(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PartView>, ApiError> {
    blocking(&s, move |ws| query::part_view(ws, &id))
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
