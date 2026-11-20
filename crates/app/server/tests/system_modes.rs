//! 電源のモード別の電力（`POST /system_modes/power` の `analyzed`）と汎用の解析 API の境界を確認する。
mod common;
use axum::http::StatusCode;
use common::{call, demo_router};
use serde_json::json;

const URI: &str = "/api/v1/system_modes/power";

#[tokio::test]
async fn analyzed_view_has_case_resolved_budgets_and_charts() {
    let (_tmp, router) = demo_router();
    let (status, plain) = call(&router, "POST", URI, Some(json!({}))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        plain["modes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["power"].is_null())
    );
    for body in [
        json!({"analyzed": true}),
        json!({"analyzed": true, "case": "hot_imaging"}),
    ] {
        let (status, view) = call(&router, "POST", URI, Some(body)).await;
        assert_eq!(status, StatusCode::OK, "{view}");
        let modes = view["modes"].as_array().unwrap();
        assert!(modes.iter().any(|m| !m["power"]["sunlit"].is_null()));
        assert!(
            modes
                .iter()
                .all(|m| !m["power"]["charts"].as_array().unwrap().is_empty())
        );
    }
    // 電源の解析 4 つは汎用の解析 API で、概要の行に棒グラフを持つ
    for id in [
        "power.mode_load",
        "power.orbit_generation",
        "power.energy_balance",
        "power.sizing",
    ] {
        let (status, result) = call(
            &router,
            "POST",
            &format!("/api/v1/analyses/{id}"),
            Some(json!({})),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(!result["overview"]["chart"].is_null(), "{id}");
    }
}

#[tokio::test]
async fn totals_scope_returns_only_the_total_row() {
    let (_tmp, router) = demo_router();
    let (_, all) = call(&router, "POST", URI, Some(json!({}))).await;
    assert_eq!(all["summary"].as_array().unwrap().len(), 3);
    let (_, totals) = call(&router, "POST", URI, Some(json!({"summary": "totals"}))).await;
    assert_eq!(totals["summary"].as_array().unwrap().len(), 1);
    assert_eq!(totals["summary"][0]["verdict"], false);
}

#[tokio::test]
async fn post_takes_a_case_and_draft_ops_like_budgets() {
    let (_tmp, router) = demo_router();
    let worst_ok = |view: &serde_json::Value| {
        view["modes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["power"]["verdict"] != "violation")
    };
    let body = json!({
        "analyzed": true,
        "ops": [{"op": "set_setting", "path": "analysis.power.dod_max", "value": {"type": "number", "value": 0.001}}],
    });
    let (status, draft) = call(&router, "POST", URI, Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{draft}");
    assert!(!worst_ok(&draft));
    // 書き込まない
    let (_, saved) = call(&router, "POST", URI, Some(json!({"analyzed": true}))).await;
    assert!(worst_ok(&saved));
}
