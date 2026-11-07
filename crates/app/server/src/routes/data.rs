//! 外部データの API: `GET /data/status`・`POST /data/{id}/fetch`。

use crate::error::ApiError;
use axum::Json;
use axum::extract::Path;
use fune_service::data;
use fune_service::dto::DataStatus;

/// GET `/data/status`
pub async fn status() -> Result<Json<DataStatus>, ApiError> {
    run(data::data_status).await
}

/// POST `/data/{id}/fetch`（例: `/data/de440/fetch`）
pub async fn fetch(Path(id): Path<String>) -> Result<Json<DataStatus>, ApiError> {
    run(move || data::fetch(&id)).await
}

async fn run<F>(f: F) -> Result<Json<DataStatus>, ApiError>
where
    F: FnOnce() -> fune_service::Result<DataStatus> + Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .map(Json)
        .map_err(ApiError::from)
}
