//! 外部データの API（`/data/status`・`/data/{id}/fetch`・`/data/space_weather`）。取得はネットワークが要るので試さない。

mod common;

use axum::http::StatusCode;
use common::{call, demo_router};
use serde_json::json;

#[tokio::test]
async fn data_status() {
    let (_tmp, r) = demo_router();
    let (s, d) = call(&r, "GET", "/api/v1/data/status", None).await;
    assert_eq!(s, StatusCode::OK, "{d}");
    let ids: Vec<&str> = d["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids[0], "de440");
    // 放射線帯のデータ（地図・古い地磁気モデル・IGRF）は表の行で、宇宙天気はその後ろ
    for id in [
        "ap8min",
        "ap8max",
        "ae8min",
        "ae8max",
        "jensen_cain_1960",
        "gsfc_1966",
        "igrf14",
    ] {
        assert!(ids.contains(&id), "{id} is missing from {ids:?}");
    }
    assert_eq!(*ids.last().unwrap(), "space_weather");
    let ap8 = d["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == "ap8min")
        .unwrap();
    assert!(ap8["license"].as_str().unwrap().contains("PT-11B"));
    let (s, e) = call(&r, "POST", "/api/v1/data/nope/fetch", None).await;
    assert_eq!(s, StatusCode::NOT_FOUND, "{e}");
}

#[tokio::test]
async fn space_weather_lookup() {
    let (_tmp, r) = demo_router();
    // 表（1957 年から）の前の日付は、キャッシュの有無によらずプリセット
    let (s, w) = call(
        &r,
        "GET",
        "/api/v1/data/space_weather?date=1950-01-01&activity=high",
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{w}");
    assert_eq!(
        (&w["weather"]["f107_source"], &w["weather"]["f107"]),
        (&json!("preset"), &json!(250.0))
    );
    let (s, e) = call(
        &r,
        "GET",
        "/api/v1/data/space_weather?date=2003-10-29&activity=x",
        None,
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "{e}");
}

/// 問い合わせの形が違えば、axum の素の文ではなく `{error}` の JSON で 400 を返す（`date` が無い）。
#[tokio::test]
async fn space_weather_bad_query_is_error_envelope() {
    let (_tmp, r) = demo_router();
    let (s, e) = call(&r, "GET", "/api/v1/data/space_weather?activity=high", None).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "{e}");
    assert!(
        e["error"]["code"].is_string() && e["error"]["message"].is_string(),
        "{e}"
    );
}
