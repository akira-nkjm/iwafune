//! 電源ページ全体の API。解析収集と表示定義は service が持つ。
use super::{ApiQuery, AppState, blocking};
use crate::error::ApiError;
use axum::{Json, extract::State};
use fune_service::dto::{PowerPageQuery, PowerPageView};

/// GET `/power_page?case=…`
pub async fn get(
    State(s): State<AppState>,
    ApiQuery(req): ApiQuery<PowerPageQuery>,
) -> Result<Json<PowerPageView>, ApiError> {
    blocking(&s, move |ws| {
        fune_service::power_page::power_page(ws, req.case.as_deref())
    })
    .await
    .map(Json)
}
