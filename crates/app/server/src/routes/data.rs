//! 外部データの API: `GET /data/status`・`POST /data/{id}/fetch`・`GET /data/space_weather`。

use super::ApiQuery;
use crate::error::ApiError;
use axum::Json;
use axum::extract::Path;
use fune_service::data;
use fune_service::dto::{DataStatus, SpaceWeatherLookup, SpaceWeatherQuery};

/// GET `/data/status`
pub async fn status() -> Result<Json<DataStatus>, ApiError> {
    run(data::data_status).await
}

/// POST `/data/{id}/fetch`（例: `/data/de440/fetch`）
pub async fn fetch(Path(id): Path<String>) -> Result<Json<DataStatus>, ApiError> {
    run(move || data::fetch(&id)).await
}

/// GET `/data/space_weather?date=2003-10-29&activity=moderate`
pub async fn space_weather(
    ApiQuery(q): ApiQuery<SpaceWeatherQuery>,
) -> Result<Json<SpaceWeatherLookup>, ApiError> {
    run(move || data::space_weather::lookup(&q.date, q.activity.as_deref())).await
}

async fn run<T, F>(f: F) -> Result<Json<T>, ApiError>
where
    T: Send + 'static,
    F: FnOnce() -> fune_service::Result<T> + Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .map(Json)
        .map_err(ApiError::from)
}
