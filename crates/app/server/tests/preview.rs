//! 保存していない編集を当てた解析とバジェット（解析の依頼の `ops`、`POST /budgets/{kind}`）。
//! 書き込まないこと、同じ編集を保存してから読んだ結果と一致することを確かめる（GUI の下書きの電力の表示の元）。

mod common;

use axum::http::StatusCode;
use common::{call, demo_router};
use serde_json::{Value, json};

/// 撮像で CAM を on、定常で RW_X を off にする下書き
fn draft() -> Value {
    json!([{ "op": "set_mode_states", "cells": [
        { "mode": "MISSION", "part": "CAM", "state": "on", "duty": {} },
        { "mode": "NOMINAL", "part": "RW_X", "state": "off", "duty": {} },
    ] }])
}

fn mode_total(budget: &Value, mode: &str) -> f64 {
    budget["modes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["node"] == mode)
        .unwrap()["with_growth"]["si"]
        .as_f64()
        .unwrap()
}

#[tokio::test]
async fn draft_budget_and_analysis_match_the_saved_design_and_write_nothing() {
    let (_tmp, r) = demo_router();
    let (_, before) = call(&r, "GET", "/api/v1/project", None).await;
    let (s, saved) = call(&r, "GET", "/api/v1/budgets/power", None).await;
    assert_eq!(s, StatusCode::OK, "{saved}");

    let (s, preview) = call(
        &r,
        "POST",
        "/api/v1/budgets/power",
        Some(json!({ "ops": draft() })),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{preview}");
    let (s, run) = call(
        &r,
        "POST",
        "/api/v1/analyses/power.mode_load",
        Some(json!({ "ops": draft() })),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{run}");
    // 下書きで電力が変わる（CAM を足した撮像は増え、RW_X を切った定常は減る）
    assert!(mode_total(&preview, "MISSION") > mode_total(&saved, "MISSION"));
    assert!(mode_total(&preview, "NOMINAL") < mode_total(&saved, "NOMINAL"));

    // 書き込んでいない
    let (_, after) = call(&r, "GET", "/api/v1/project", None).await;
    assert_eq!(before["revision"], after["revision"]);

    // 同じ編集を保存して読み直すと、プレビューと同じ値
    let (s, ok) = call(&r, "POST", "/api/v1/ops", Some(json!({ "ops": draft() }))).await;
    assert_eq!(s, StatusCode::OK, "{ok}");
    let (_, now) = call(&r, "GET", "/api/v1/budgets/power", None).await;
    assert_eq!(now["modes"], preview["modes"]);
    let (_, rerun) = call(
        &r,
        "POST",
        "/api/v1/analyses/power.mode_load",
        Some(json!({})),
    )
    .await;
    assert_eq!(rerun["output"], run["output"]);
}

#[tokio::test]
async fn an_invalid_draft_is_rejected_like_saving() {
    let (_tmp, r) = demo_router();
    let bad = json!([{ "op": "set_mode_states", "cells": [
        { "mode": "MISSION", "part": "NOPE", "state": "on", "duty": {} },
    ] }]);
    let (s, e) = call(
        &r,
        "POST",
        "/api/v1/budgets/power",
        Some(json!({ "ops": bad })),
    )
    .await;
    assert!(s.is_client_error(), "{s} {e}");
    assert!(e["error"]["code"].is_string());
}
