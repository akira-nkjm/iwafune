//! `/units`・`/units/parse`。

use super::{ApiJson, ApiQuery};
use crate::error::ApiError;
use axum::Json;
use fune_service::dto::{
    FormatRequest, FormatResponse, ParseQuantityRequest, ParsedQuantity, UnitsCatalog, UnitsQuery,
};

/// GET `/units?dimension=Power`
pub async fn catalog(ApiQuery(q): ApiQuery<UnitsQuery>) -> Result<Json<UnitsCatalog>, ApiError> {
    Ok(Json(fune_service::units::units_catalog(&q)?))
}

/// POST `/units/parse` `{ input, dimension? }`
pub async fn parse(
    ApiJson(req): ApiJson<ParseQuantityRequest>,
) -> Result<Json<ParsedQuantity>, ApiError> {
    Ok(Json(fune_service::units::parse_quantity(&req)?))
}

/// POST `/units/format` `{ items: [{ si, dimension, unit? }] }`
pub async fn format(
    ApiJson(req): ApiJson<FormatRequest>,
) -> Result<Json<FormatResponse>, ApiError> {
    Ok(Json(fune_service::units::format_quantities(&req)?))
}
