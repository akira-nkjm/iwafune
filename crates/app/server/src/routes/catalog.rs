//! 共有カタログの API: `POST /catalog/fetch`・`POST /catalog/update`（`fune catalog fetch|update` と同じ service の関数）。

use super::{ApiJson, AppState, blocking};
use crate::error::ApiError;
use axum::Json;
use axum::extract::State;
use fune_service::dto::{CatalogFetchRequest, CatalogSyncResult, CatalogUpdateRequest};
use fune_service::shared_catalog;

/// POST `/catalog/fetch` `{ locked? }`
pub async fn fetch(
    State(s): State<AppState>,
    ApiJson(req): ApiJson<CatalogFetchRequest>,
) -> Result<Json<CatalogSyncResult>, ApiError> {
    blocking(&s, move |ws| shared_catalog::fetch(ws, &req))
        .await
        .map(Json)
}

/// POST `/catalog/update` `{ name }`
pub async fn update(
    State(s): State<AppState>,
    ApiJson(req): ApiJson<CatalogUpdateRequest>,
) -> Result<Json<CatalogSyncResult>, ApiError> {
    blocking(&s, move |ws| shared_catalog::update(ws, &req))
        .await
        .map(Json)
}
