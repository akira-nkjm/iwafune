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
            // 結果は汎用の解析 API と同じ。概要の行だけは電源の棒グラフと隠す表が足される（主な結果は同じ）
            let mut page_result = analysis["result"].clone();
            let mut direct = direct;
            let page_overview = page_result
                .as_object_mut()
                .unwrap()
                .remove("overview")
                .unwrap();
            let direct_overview = direct.as_object_mut().unwrap().remove("overview").unwrap();
            assert_eq!(page_result, direct);
            assert_eq!(page_overview["main"], direct_overview["main"]);
            assert!(direct_overview["chart"].is_null());
            assert!(!page_overview["chart"].is_null());
        }
        assert!(!page["modes"].as_array().unwrap().is_empty());
        assert!(page.get("parts").is_none());
    }
}

#[tokio::test]
async fn post_takes_a_case_and_draft_ops_like_budgets() {
    let (_tmp, router) = demo_router();
    let (_, saved) = call(&router, "GET", "/api/v1/power_page", None).await;
    let dod = |page: &serde_json::Value| {
        page["analyses"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["result"]["id"] == "power.energy_balance")
            .unwrap()["result"]["output"]["dod_limit"]
            .as_f64()
            .unwrap()
    };
    assert!((dod(&saved) - 0.3).abs() < 1e-12);
    let body = json!({
        "case": "hot_imaging",
        "ops": [{"op": "set_setting", "path": "analysis.power.dod_max", "value": {"type": "number", "value": 0.2}}],
    });
    let (status, draft) = call(&router, "POST", "/api/v1/power_page", Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{draft}");
    assert!((dod(&draft) - 0.2).abs() < 1e-12);
    // 書き込まない
    let (_, again) = call(&router, "GET", "/api/v1/power_page", None).await;
    assert!((dod(&again) - 0.3).abs() < 1e-12);
}
