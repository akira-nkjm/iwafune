//! 解析とバジェットの API: `/analyses`・`/analyses/{id}`・`/budgets/{kind}`。

use super::{ApiJson, AppState, blocking};
use crate::error::ApiError;
use axum::Json;
use axum::extract::{Path, State};
use fune_service::dto::{
    AnalysisEntry, AnalysisRunResult, BudgetRequest, BudgetView, RunAnalysisRequest,
};
use fune_service::{analysis, budget};

/// GET `/analyses`
pub async fn catalog() -> Result<Json<Vec<AnalysisEntry>>, ApiError> {
    Ok(Json(analysis::analysis_catalog()?))
}

/// POST `/analyses/{id}` `{ case?, source?, ops? }`（`source` は `{ kind: "design" | "patched" | "given", … }`）
pub async fn run(
    State(s): State<AppState>,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<RunAnalysisRequest>,
) -> Result<Json<AnalysisRunResult>, ApiError> {
    blocking(&s, move |ws| analysis::run_analysis(ws, &id, &req))
        .await
        .map(Json)
}

/// GET `/budgets/{kind}`（mass / power / data）
pub async fn budget(
    State(s): State<AppState>,
    Path(kind): Path<String>,
) -> Result<Json<BudgetView>, ApiError> {
    blocking(&s, move |ws| {
        budget::budget(ws, budget::parse_kind(&kind)?, &BudgetRequest::default())
    })
    .await
    .map(Json)
}

/// POST `/budgets/{kind}` `{ ops }`: 保存していない編集を当てた設計のバジェット（書き込まない）
pub async fn budget_with(
    State(s): State<AppState>,
    Path(kind): Path<String>,
    ApiJson(req): ApiJson<BudgetRequest>,
) -> Result<Json<BudgetView>, ApiError> {
    blocking(&s, move |ws| {
        budget::budget(ws, budget::parse_kind(&kind)?, &req)
    })
    .await
    .map(Json)
}
