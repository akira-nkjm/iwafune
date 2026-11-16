//! サービスのエラー → HTTP の状態と JSON（`{ error: { code, message }, diagnostics }`）。

use axum::Json;
use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use fune_service::dto::{ErrorEnvelope, ErrorInfo};
use fune_service::{ErrorKind, ServiceError};

/// HTTP で返すエラー。
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    body: ErrorEnvelope,
}

impl ApiError {
    /// 状態とコードと文から作る。
    pub fn new(status: StatusCode, code: &str, message: impl Into<String>) -> Self {
        Self {
            status,
            body: ErrorEnvelope {
                error: ErrorInfo {
                    code: code.to_owned(),
                    message: message.into(),
                },
                diagnostics: Vec::new(),
            },
        }
    }

    /// `/api` の下の未知のパス。
    pub fn route_not_found(path: &str) -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            "route_not_found",
            format!("No such API path: {path}"),
        )
    }

    /// 内部エラー（タスクの異常終了など）。
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal", message)
    }
}

/// エラーの分類 → HTTP の状態。
pub fn status_of(kind: ErrorKind) -> StatusCode {
    match kind {
        ErrorKind::NotFound => StatusCode::NOT_FOUND,
        ErrorKind::InvalidInput => StatusCode::BAD_REQUEST,
        ErrorKind::Validation => StatusCode::UNPROCESSABLE_ENTITY,
        ErrorKind::Conflict => StatusCode::CONFLICT,
        ErrorKind::Internal => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

impl From<ServiceError> for ApiError {
    fn from(e: ServiceError) -> Self {
        Self {
            status: status_of(e.kind()),
            body: e.envelope(),
        }
    }
}

impl From<JsonRejection> for ApiError {
    fn from(e: JsonRejection) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "invalid_json", e.body_text())
    }
}

impl From<QueryRejection> for ApiError {
    fn from(e: QueryRejection) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "invalid_query", e.body_text())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}
