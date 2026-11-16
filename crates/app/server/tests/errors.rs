//! エラーの分類（HTTP の状態）と、エラーにしない返し方。同じ状況は入口が違っても同じ状態で返す。
//! 下書きが検証で拒まれる 422 は `preview.rs`。

mod common;

use axum::http::StatusCode;
use common::{call, demo_router};
use serde_json::json;

/// 同じ ID がもうあるときは、入力の誤り（400）ではなく今の状態との競合（409）。
#[tokio::test]
async fn existing_id_is_409() {
    let (_tmp, r) = demo_router();
    let (_, before) = call(&r, "GET", "/api/v1/project", None).await;
    let op =
        json!([{ "op": "create_part", "id": "RW_X", "group": "aocs", "kind": "reaction_wheel" }]);
    let (s, e) = call(&r, "POST", "/api/v1/ops", Some(json!({ "ops": op }))).await;
    assert_eq!(s, StatusCode::CONFLICT, "{e}");
    assert_eq!(e["error"]["code"], "already_exists", "{e}");
    let (_, after) = call(&r, "GET", "/api/v1/project", None).await;
    assert_eq!(before["revision"], after["revision"]);
}

/// バジェットの入力不足は、解析と同じく 200 の `missing` で返す（500 にしない）。
#[tokio::test]
async fn budget_missing_inputs_are_listed_not_500() {
    let (_tmp, r) = demo_router();
    // モードの割合を消した下書き: データ量は数えられず、足りない設定を名指しする
    let ops = json!([{ "op": "set_setting", "path": "analysis.power.mode_share", "value": null }]);
    let (s, b) = call(
        &r,
        "POST",
        "/api/v1/budgets/data",
        Some(json!({ "ops": ops })),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["missing"][0]["path"], "analysis.power.mode_share", "{b}");
    assert_eq!(b["missing"][0]["target"]["target"], "setting", "{b}");
    assert!(b["total"].is_null() && b["verdict"].is_null(), "{b}");
    assert_eq!(b["rows"].as_array().map(Vec::len), Some(0), "{b}");
    // 上限とシステムマージンは配分から決まるので、足りなくても出す
    assert_eq!(b["system_margin"]["display"], "20.0 %", "{b}");
    // 無いケースも、解析と同じく不足として返す
    let (s, b) = call(
        &r,
        "POST",
        "/api/v1/budgets/power",
        Some(json!({ "case": "nope" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["missing"][0]["target"]["kind"], "case", "{b}");
}

/// バジェットはケースを指定できる（解析と同じ `case`）。電力はそのケースのモードで数える。
#[tokio::test]
async fn budget_follows_the_case_mode() {
    let (_tmp, r) = demo_router();
    // cases/cold_safe.toml のモードは SAFE
    let (s, b) = call(
        &r,
        "POST",
        "/api/v1/budgets/power",
        Some(json!({ "case": "cold_safe" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["case"], "cold_safe");
    assert_eq!(b["mode"], "SAFE");
    let safe = b["modes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["node"] == "SAFE")
        .unwrap();
    assert_eq!(b["total"]["si"], safe["with_growth"]["si"]);
    let (_, default) = call(&r, "GET", "/api/v1/budgets/power", None).await;
    assert!(default["case"].is_null());
    assert_ne!(default["mode"], "SAFE");
    assert!(default["missing"].as_array().unwrap().is_empty());
}

/// ファイルに当たらない設定のパスは、入力の誤り（400）で返す（500 にしない）。
/// 区切りが空のパス（`analysis.` など）も同じ。
#[tokio::test]
async fn bad_setting_paths_are_400() {
    let (_tmp, r) = demo_router();
    let (_, before) = call(&r, "GET", "/api/v1/project", None).await;
    for path in ["analysis.nope.x", "analysis.", "budget.", "conditions.", ""] {
        let ops = json!([{ "op": "set_setting", "path": path, "value": null }]);
        let (s, e) = call(&r, "POST", "/api/v1/ops", Some(json!({ "ops": ops }))).await;
        assert_eq!(s, StatusCode::BAD_REQUEST, "`{path}`: {e}");
        assert_eq!(e["error"]["code"], "invalid_input", "`{path}`: {e}");
    }
    let (_, after) = call(&r, "GET", "/api/v1/project", None).await;
    assert_eq!(before["revision"], after["revision"]);
}
