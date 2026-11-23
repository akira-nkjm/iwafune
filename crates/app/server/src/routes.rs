//! ルーティング。ハンドラーは引数を読み、`fune_service` を呼び、JSON にするだけ。
//!
//! | モジュール | パス |
//! |---|---|
//! | `design` | `/project` `/design` `/system_modes/{system}`（POST） `/maneuvers` `/phases` `/configurations` `/requirements` `/requirements/compliance` `/parts` `/parts/{id}`（POST は下書きを当てた部品） `/values/{path}` `/defs` `/diagnostics` |
//! | `edit` | `/ops` `/ops/from_text` `/ops/unset_from_text` `/ops/from_command` `/ops/case_from_eclipse` `/ops/case_from_sun_distance` `/parts/{id}/place_body` `/fmt` |
//! | `history` | `/history` `/undo` `/redo` |
//! | `units` | `/units` `/units/parse` `/units/format` |
//! | `analyses` | `/analyses` `/analyses/{id}` `/budgets/{kind}`（GET と、下書きを当てる POST） |
//! | `report` | `/report/export` |
//! | `data` | `/data/status` `/data/space_weather` `/data/{id}/fetch` |
//! | `catalog` | `/catalog/{id}`（カタログ品 1 件） `/catalog/fetch` `/catalog/update`（共有カタログ） |

mod analyses;
mod catalog;
mod data;
mod design;
mod edit;
mod history;
mod report;
mod units;

use crate::error::ApiError;
use crate::host::HostPolicy;
use crate::spa;
use axum::Router;
use axum::extract::{FromRequest, FromRequestParts, Request};
use axum::http::request::Parts;
use axum::http::{HeaderValue, Method, Uri};
use axum::routing::{get, post};
use fune_service::Workspace;
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};

/// ハンドラーが共有する状態。
#[derive(Clone)]
pub struct AppState {
    pub workspace: Arc<Workspace>,
}

/// `/api/v1` の API と SPA の配信をまとめたルーター（Host はローカルホストだけ許す）。
pub fn router(workspace: Arc<Workspace>) -> Router {
    router_with(workspace, HostPolicy::local_only())
}

/// Host の許可リストを指定したルーター（`fune gui --host 0.0.0.0 --allow-host …` 用）。
pub fn router_with(workspace: Arc<Workspace>, hosts: HostPolicy) -> Router {
    fune_service::data::install_cached_ephemeris();
    let hosts = Arc::new(hosts);
    let api = Router::new()
        .route("/project", get(design::project))
        .route("/design", get(design::design))
        .route("/system_modes/{system}", post(design::system_modes))
        .route("/maneuvers", get(design::maneuvers))
        .route("/phases", get(design::phases))
        .route("/configurations", get(design::configurations))
        .route("/requirements", get(design::requirements))
        .route("/requirements/compliance", get(design::compliance))
        .route("/parts", get(design::parts))
        .route("/parts/{id}", get(design::part).post(design::part_with))
        .route("/parts/{id}/place_body", post(edit::place_body))
        .route("/values/{path}", get(design::value))
        .route("/defs", get(design::defs))
        .route("/diagnostics", get(design::diagnostics))
        .route("/fmt", post(edit::format))
        .route("/ops", post(edit::apply))
        .route("/ops/from_text", post(edit::op_from_text))
        .route("/ops/unset_from_text", post(edit::op_unset_from_text))
        .route("/ops/from_command", post(edit::ops_from_command))
        .route("/ops/case_from_eclipse", post(edit::case_from_eclipse))
        .route(
            "/ops/case_from_sun_distance",
            post(edit::case_from_sun_distance),
        )
        .route("/history", get(history::view))
        .route("/undo", post(history::undo))
        .route("/redo", post(history::redo))
        .route("/units", get(units::catalog))
        .route("/units/parse", post(units::parse))
        .route("/units/format", post(units::format))
        .route("/analyses", get(analyses::catalog))
        .route("/analyses/{id}", post(analyses::run))
        .route(
            "/budgets/{kind}",
            get(analyses::budget).post(analyses::budget_with),
        )
        .route("/report/export", post(report::export))
        .route("/data/status", get(data::status))
        .route("/data/space_weather", get(data::space_weather))
        .route("/data/{id}/fetch", post(data::fetch))
        .route("/catalog/{*id}", get(catalog::item))
        .route("/catalog/fetch", post(catalog::fetch))
        .route("/catalog/update", post(catalog::update))
        .fallback(api_not_found);
    Router::new()
        .nest("/api/v1", api)
        .fallback(spa::fallback)
        .layer(cors())
        .layer(axum::middleware::from_fn(move |req, next| {
            crate::host::check(hosts.clone(), req, next)
        }))
        .with_state(AppState { workspace })
}

async fn api_not_found(uri: Uri) -> ApiError {
    ApiError::route_not_found(&format!("/api/v1{}", uri.path()))
}

/// 開発中の Vite（別のポート）から呼べるよう、ローカルホストの origin だけを許す。
fn cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin: &HeaderValue, _| {
            origin.to_str().is_ok_and(is_local_origin)
        }))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([axum::http::header::CONTENT_TYPE])
}

fn is_local_origin(origin: &str) -> bool {
    let host = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
        .unwrap_or_default();
    let host = host.rsplit_once(':').map_or(host, |(h, _)| h);
    matches!(host, "localhost" | "127.0.0.1" | "[::1]")
}

/// サービスの関数を別スレッドで呼ぶ（ファイル I/O と解析で非同期の実行器を止めないため）。
pub(crate) async fn blocking<T, F>(state: &AppState, f: F) -> Result<T, ApiError>
where
    T: Send + 'static,
    F: FnOnce(&Workspace) -> fune_service::Result<T> + Send + 'static,
{
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || f(&ws))
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .map_err(ApiError::from)
}

/// 失敗を `{error}` の JSON で返す JSON 本文の取り出し。
pub(crate) struct ApiJson<T>(pub T);

impl<S, T> FromRequest<S> for ApiJson<T>
where
    S: Send + Sync,
    axum::Json<T>: FromRequest<S, Rejection = axum::extract::rejection::JsonRejection>,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let axum::Json(v) = axum::Json::<T>::from_request(req, state).await?;
        Ok(Self(v))
    }
}

/// 失敗を `{error}` の JSON で返すクエリの取り出し。
pub(crate) struct ApiQuery<T>(pub T);

impl<S, T> FromRequestParts<S> for ApiQuery<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let axum::extract::Query(v) =
            axum::extract::Query::<T>::from_request_parts(parts, state).await?;
        Ok(Self(v))
    }
}
