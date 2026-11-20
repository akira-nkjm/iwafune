//! 編集の履歴の API: `GET /history`・`POST /undo`・`POST /redo`。

use super::{AppState, blocking};
use crate::error::ApiError;
use axum::Json;
use axum::extract::State;
use fune_service::dto::{HistoryStepResult, HistoryView};

/// POST `/undo`（本文なし）。直前の編集を取り消す（`fune undo` と同じ）。
pub async fn undo(State(s): State<AppState>) -> Result<Json<HistoryStepResult>, ApiError> {
    blocking(&s, fune_service::history::undo).await.map(Json)
}

/// POST `/redo`（本文なし）。取り消した編集をやり直す（`fune redo` と同じ）。
pub async fn redo(State(s): State<AppState>) -> Result<Json<HistoryStepResult>, ApiError> {
    blocking(&s, fune_service::history::redo).await.map(Json)
}

/// GET `/history`。編集の履歴（`fune history` と同じ）。
pub async fn view(State(s): State<AppState>) -> Result<Json<HistoryView>, ApiError> {
    blocking(&s, fune_service::history::view).await.map(Json)
}
