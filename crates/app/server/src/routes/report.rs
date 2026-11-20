//! 設計書の表と図の書き出しの API: `/report/export`。ファイルは書かず、内容を返す（書き出すのは呼び出し側）。

use super::{ApiJson, AppState, blocking};
use crate::error::ApiError;
use axum::Json;
use axum::extract::State;
use fune_service::dto::{ReportExport, ReportRequest};
use fune_service::report;

/// POST `/report/export` `{ items?, cases?, lang? }`: `data.json`・Typst のモジュール・SVG の内容（`files`）と数えられなかった項目
pub async fn export(
    State(s): State<AppState>,
    ApiJson(req): ApiJson<ReportRequest>,
) -> Result<Json<ReportExport>, ApiError> {
    blocking(&s, move |ws| report::export(ws, &req))
        .await
        .map(Json)
}
