//! 編集の API: `POST /ops`・`POST /ops/from_text`・`POST /ops/unset_from_text`・`POST /parts/{id}/place_body`。

use super::{ApiJson, AppState, blocking};
use crate::error::ApiError;
use axum::Json;
use axum::extract::State;
use fune_service::dto::{ApplyRequest, ApplyResult, Op, SetText, UnsetText};

/// POST `/ops` `{ ops, base_revision?, dry_run? }`
pub async fn apply(
    State(s): State<AppState>,
    ApiJson(req): ApiJson<ApplyRequest>,
) -> Result<Json<ApplyResult>, ApiError> {
    blocking(&s, move |ws| fune_service::edit::apply(ws, &req))
        .await
        .map(Json)
}

/// POST `/parts/{id}/place_body` `{ position: {x,y,z,unit}, orientation?, base_revision?, dry_run? }`
/// 機体座標での配置を、取付点に対する相対位置・姿勢の `Place` に直して適用する。
pub async fn place_body(
    State(s): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    ApiJson(req): ApiJson<fune_service::dto::PlaceBodyRequest>,
) -> Result<Json<ApplyResult>, ApiError> {
    blocking(&s, move |ws| fune_service::edit::place_body(ws, &id, &req))
        .await
        .map(Json)
}

/// POST `/ops/from_text` `{ item?, path, value, source?, note?, min?, max? }`
/// 人が打った値の文字列を Op にする（`fune set <PATH> <VALUE>`、`item` があれば `fune catalog set <品> <PATH> <VALUE>`
/// と同じ解釈）。適用はしない（`POST /ops` で送る）。
pub async fn op_from_text(
    State(s): State<AppState>,
    ApiJson(req): ApiJson<SetText>,
) -> Result<Json<Op>, ApiError> {
    blocking(&s, move |ws| fune_service::edit::set_op(ws, &req))
        .await
        .map(Json)
}

/// POST `/ops/unset_from_text` `{ item?, path }`
/// 消す先のパスを Op にする（`fune unset <PATH>`、`item` があれば `fune catalog unset <品> <PATH>` と同じ解釈）。
/// 適用はしない（`POST /ops` で送る）。
pub async fn op_unset_from_text(
    State(s): State<AppState>,
    ApiJson(req): ApiJson<UnsetText>,
) -> Result<Json<Op>, ApiError> {
    blocking(&s, move |_| fune_service::edit::unset_op(&req))
        .await
        .map(Json)
}

/// POST /fmt: check-only or in-place design formatting.
pub async fn format(
    State(s): State<AppState>,
    ApiJson(req): ApiJson<fune_service::dto::FormatDesignRequest>,
) -> Result<Json<fune_service::dto::FormatDesignResult>, ApiError> {
    blocking(&s, move |ws| fune_service::format::format_design(ws, &req))
        .await
        .map(Json)
}
