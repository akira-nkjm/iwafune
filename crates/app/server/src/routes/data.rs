//! 外部データの API: `GET /data/status`・`POST /data/{id}/fetch`・`GET /data/space_weather`。

use crate::error::ApiError;
use axum::Json;
use axum::extract::{Path, Query};
use fune_service::data;
use fune_service::dto::{DataStatus, SpaceWeatherLookup};
use serde::Deserialize;

/// GET `/data/status`
pub async fn status() -> Result<Json<DataStatus>, ApiError> {
    run(data::data_status).await
}

/// POST `/data/{id}/fetch`（例: `/data/de440/fetch`）
pub async fn fetch(Path(id): Path<String>) -> Result<Json<DataStatus>, ApiError> {
    run(move || data::fetch(&id)).await
}

/// `GET /data/space_weather` の問い合わせ。
#[derive(Debug, Deserialize)]
pub struct SpaceWeatherQuery {
    /// ISO 8601 の元期（日付だけでも可）
    pub date: String,
    /// 表の範囲の外で使う太陽活動（`low`・`moderate`・`high`）
    pub activity: Option<String>,
}

/// GET `/data/space_weather?date=2003-10-29&activity=moderate`
pub async fn space_weather(
    Query(q): Query<SpaceWeatherQuery>,
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
