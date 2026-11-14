//! 電源専用 API と既存の汎用解析 API の境界を確認する。
mod common;
use axum::http::StatusCode;
use common::{call, demo_router};
use serde_json::json;

#[tokio::test]
async fn whole_page_has_case_resolved_analyses_and_power_data() {
    let (_tmp, router) = demo_router();
    for uri in ["/api/v1/power_page", "/api/v1/power_page?case=hot_imaging"] {
        let (status, page) = call(&router, "GET", uri, None).await;
        assert_eq!(status, StatusCode::OK);
        let analyses = page["analyses"].as_array().unwrap();
        assert_eq!(analyses.len(), 4);
        for analysis in analyses {
            let id = analysis["result"]["id"].as_str().unwrap();
            let req = if uri.contains('?') {
                json!({"case":"hot_imaging"})
            } else {
                json!({})
            };
            let (status, direct) = call(
                &router,
                "POST",
                &format!("/api/v1/analyses/{id}"),
                Some(req),
            )
            .await;
            assert_eq!(status, StatusCode::OK);
            assert!(direct.get("power_page").is_none());
            assert_eq!(analysis["result"], direct);
        }
        assert!(!page["modes"].as_array().unwrap().is_empty());
        assert!(page.get("parts").is_none());
    }
}
