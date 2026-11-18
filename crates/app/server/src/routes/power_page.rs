//! 電源ページ全体の API。解析収集と表示定義は service が持つ。
use super::{ApiJson, ApiQuery, AppState, blocking};
use crate::error::ApiError;
use axum::{Json, extract::State};
use fune_service::dto::{PowerPageQuery, PowerPageView};

/// GET `/power_page?case=…`
pub async fn get(
    State(s): State<AppState>,
    ApiQuery(req): ApiQuery<PowerPageQuery>,
) -> Result<Json<PowerPageView>, ApiError> {
    blocking(&s, move |ws| {
        fune_service::power_page::power_page_with(ws, &req)
    })
    .await
    .map(Json)
}

/// POST `/power_page` `{ case, ops }`: 保存していない編集を当てた設計の電源ページ（書き込まない）
pub async fn post(
    State(s): State<AppState>,
    ApiJson(req): ApiJson<PowerPageQuery>,
) -> Result<Json<PowerPageView>, ApiError> {
    blocking(&s, move |ws| {
        fune_service::power_page::power_page_with(ws, &req)
    })
    .await
    .map(Json)
}
