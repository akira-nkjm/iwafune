//! 系のフロー図の API。カード・行・つながりは service（`fune_service::flow`）が作る。
use super::{ApiQuery, AppState, blocking};
use crate::error::ApiError;
use axum::{
    Json,
    extract::{Path, State},
};
use fune_service::dto::{FlowQuery, FlowView};

/// GET `/flow/{system}?case=…`
pub async fn get(
    State(s): State<AppState>,
    Path(system): Path<String>,
    ApiQuery(req): ApiQuery<FlowQuery>,
) -> Result<Json<FlowView>, ApiError> {
    blocking(&s, move |ws| {
        let system = fune_service::system_modes::parse_system(&system)?;
        fune_service::flow::system_flow(ws, system, req.case.as_deref())
    })
    .await
    .map(Json)
}
