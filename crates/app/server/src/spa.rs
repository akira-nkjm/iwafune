//! ビルド済みの SPA（web/dist）を同梱して配信する。クライアント側のルート（`/parts/RW_X` など）には index.html を返す。

use crate::error::ApiError;
use axum::http::{HeaderValue, StatusCode, Uri, header};
use axum::response::{Html, IntoResponse, Response};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../../web/dist"]
#[allow_missing = true]
struct Assets;

const MISSING_BUILD: &str = "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>fune</title></head>\
<body style=\"font-family:sans-serif;padding:2rem\"><h1>fune</h1>\
<p>The GUI build (web/dist) is missing. Run <code>pnpm -C web install &amp;&amp; pnpm -C web build</code>, then rebuild fune.</p>\
<p>The API is available at <a href=\"/api/v1/project\">/api/v1</a>.</p></body></html>";

/// `/api` 以外のすべての GET を受ける。
pub async fn fallback(uri: Uri) -> Response {
    let path = uri.path();
    if path == "/api" || path.starts_with("/api/") {
        return ApiError::route_not_found(path).into_response();
    }
    let rel = path.trim_start_matches('/');
    if !rel.is_empty()
        && let Some(file) = Assets::get(rel)
    {
        return asset_response(rel, file.data.into_owned());
    }
    if looks_like_file(rel) {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    }
    index()
}

fn index() -> Response {
    match Assets::get("index.html") {
        Some(file) => {
            let mut res = Html(file.data.into_owned()).into_response();
            res.headers_mut()
                .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
            res
        }
        None => Html(MISSING_BUILD).into_response(),
    }
}

fn asset_response(rel: &str, data: Vec<u8>) -> Response {
    let mime = mime_guess::from_path(rel).first_or_octet_stream();
    let cache = if rel.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    let mut res = data.into_response();
    let headers = res.headers_mut();
    if let Ok(v) = HeaderValue::from_str(mime.as_ref()) {
        headers.insert(header::CONTENT_TYPE, v);
    }
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));
    res
}

/// 最後の区切りに拡張子があるか（`/favicon.ico` は 404、`/parts/RW_X` は index.html）。
fn looks_like_file(rel: &str) -> bool {
    rel.rsplit('/')
        .next()
        .is_some_and(|last| last.contains('.') && !last.ends_with('.'))
}
