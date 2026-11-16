//! カタログの API: `GET /catalog/{id}`（`fune catalog show`）と、共有カタログの `POST /catalog/fetch`・`POST /catalog/update`（`fune catalog fetch|update` と同じ service の関数）。

use super::{ApiJson, AppState, blocking};
use crate::error::ApiError;
use axum::Json;
use axum::extract::{Path, State};
use fune_service::dto::{
    CatalogFetchRequest, CatalogItemView, CatalogSyncResult, CatalogUpdateRequest,
};
use fune_service::{query, shared_catalog};

/// GET `/catalog/{id}`（`id` は `local/rw_10mNms` のように `/` を含むので残りのパス全体）
pub async fn item(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<CatalogItemView>, ApiError> {
    blocking(&s, move |ws| query::catalog_item(ws, &id))
        .await
        .map(Json)
}

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
