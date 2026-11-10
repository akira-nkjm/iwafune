//! 各エンドポイントのテスト（tower の oneshot でルーターを直接呼ぶ）。

mod common;

use axum::http::StatusCode;
use common::{call, demo_router};
use serde_json::json;

#[tokio::test]
async fn units_catalog_and_filter() {
    let (_tmp, r) = demo_router();
    let (s, v) = call(&r, "GET", "/api/v1/units?dimension=Power", None).await;
    assert_eq!(s, StatusCode::OK);
    let units = v["units"].as_array().unwrap();
    assert!(units.iter().any(|u| u["symbol"] == "W"));
    assert!(units.iter().all(|u| u["dimension"] == "Power"));
}

#[tokio::test]
async fn units_parse_ok_and_error_envelope() {
    let (_tmp, r) = demo_router();
    let (s, v) = call(
        &r,
        "POST",
        "/api/v1/units/parse",
        Some(json!({ "input": "620 g" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["quantity"]["si"], 0.62);
    assert_eq!(v["quantity"]["dimension"], "Mass");

    let (s, v) = call(
        &r,
        "POST",
        "/api/v1/units/parse",
        Some(json!({ "input": "1 kg", "dimension": "Power" })),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert_eq!(v["error"]["code"], "unit_error");
    assert!(v["error"]["message"].as_str().is_some());
}

#[tokio::test]
async fn bad_json_is_an_error_envelope() {
    let (_tmp, r) = demo_router();
    let (s, v) = call(
        &r,
        "POST",
        "/api/v1/units/parse",
        Some(json!({ "nope": 1 })),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert_eq!(v["error"]["code"], "invalid_json");
}

#[tokio::test]
async fn unknown_api_path_is_json_404() {
    let (_tmp, r) = demo_router();
    let (s, v) = call(&r, "GET", "/api/v1/nope", None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert_eq!(v["error"]["code"], "route_not_found");
    let (s, v) = call(&r, "GET", "/api/v2/project", None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert_eq!(v["error"]["code"], "route_not_found");
}

#[tokio::test]
async fn client_routes_get_the_spa() {
    let (_tmp, r) = demo_router();
    let (s, v) = call(&r, "GET", "/parts/RW_X", None).await;
    assert_eq!(s, StatusCode::OK);
    assert!(v.as_str().unwrap().contains("<html"));
    let (s, _) = call(&r, "GET", "/missing-file.js", None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn analyses_catalog_lists_all() {
    let (_tmp, r) = demo_router();
    let (s, v) = call(&r, "GET", "/api/v1/analyses", None).await;
    assert_eq!(s, StatusCode::OK);
    let ids: Vec<&str> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["info"]["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"power.energy_balance"));
    assert!(ids.contains(&"structure.mass_properties"));
}

#[tokio::test]
async fn budget_kind_is_validated() {
    let (_tmp, r) = demo_router();
    let (s, v) = call(&r, "GET", "/api/v1/budgets/volume", None).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert_eq!(v["error"]["code"], "invalid_input");
}

#[tokio::test]
async fn project_and_design() {
    let (_tmp, r) = demo_router();
    let (s, p) = call(&r, "GET", "/api/v1/project", None).await;
    assert_eq!(s, StatusCode::OK, "{p}");
    assert!(p["revision"].as_str().unwrap().starts_with("b3:"));
    assert!(p["counts"]["parts"].as_u64().unwrap() > 0);
    // 書き込めるカタログの出所（Web が置き先の名前を書かずに選ぶ）
    assert_eq!(
        p["catalogs"],
        json!([{ "name": "local", "editable": true }])
    );
    let (s, d) = call(&r, "GET", "/api/v1/design", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(d["project"]["revision"], p["revision"]);
    for key in [
        "tree",
        "parts",
        "nets",
        "harnesses",
        "settings",
        "cases",
        "diagnostics",
    ] {
        assert!(d[key].is_array(), "{key}");
    }
    assert!(d["modes"]["modes"].is_array() && d["modes"]["rows"].is_array());
    assert!(
        d["defs"]["attributes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["id"] == "mass")
    );
}

#[tokio::test]
async fn parts_list_filter_and_detail() {
    let (_tmp, r) = demo_router();
    let (s, all) = call(&r, "GET", "/api/v1/parts", None).await;
    assert_eq!(s, StatusCode::OK);
    let (s, rw) = call(&r, "GET", "/api/v1/parts?kind=reaction_wheel", None).await;
    assert_eq!(s, StatusCode::OK);
    assert!(rw.as_array().unwrap().len() < all.as_array().unwrap().len());
    assert!(
        rw.as_array()
            .unwrap()
            .iter()
            .all(|p| p["kind"] == "reaction_wheel")
    );
    let (s, part) = call(&r, "GET", "/api/v1/parts/RW_X", None).await;
    assert_eq!(s, StatusCode::OK, "{part}");
    assert_eq!(part["part"]["id"], "RW_X");
    assert!(part["sections"].is_array());
    let (s, e) = call(&r, "GET", "/api/v1/parts/NOPE", None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert_eq!(e["error"]["code"], "not_found");
}

#[tokio::test]
async fn value_defs_and_diagnostics() {
    let (_tmp, r) = demo_router();
    let (s, v) = call(&r, "GET", "/api/v1/values/RW_X.mass", None).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["value"]["value"]["nominal"]["dimension"], "Mass");
    let (s, _) = call(&r, "GET", "/api/v1/defs", None).await;
    assert_eq!(s, StatusCode::OK);
    let (s, d) = call(&r, "GET", "/api/v1/diagnostics", None).await;
    assert_eq!(s, StatusCode::OK);
    assert!(d["counts"]["errors"].is_u64());
}

#[tokio::test]
async fn ops_dry_run_conflict_and_apply() {
    let (_tmp, r) = demo_router();
    let op = json!([{ "op": "set_value", "part": "RW_X", "attr": "mass",
        "value": { "type": "scalar", "data": { "value": 700.0, "unit": "g" } } }]);
    let (s, dry) = call(
        &r,
        "POST",
        "/api/v1/ops",
        Some(json!({ "ops": op, "dry_run": true })),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{dry}");
    assert_eq!(dry["dry_run"], true);
    assert!(dry["changed"][0]["diff"].as_str().unwrap().contains("+"));
    assert_eq!(dry["cli"][0], "fune set RW_X.mass \"700 g\"");
    let (s, c) = call(
        &r,
        "POST",
        "/api/v1/ops",
        Some(json!({ "ops": op, "base_revision": "b3:0" })),
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT, "{c}");
    assert_eq!(c["error"]["code"], "revision_conflict");
    let (s, ok) = call(&r, "POST", "/api/v1/ops", Some(json!({ "ops": op }))).await;
    assert_eq!(s, StatusCode::OK, "{ok}");
    assert_ne!(ok["revision"], ok["previous_revision"]);
    let (_, v) = call(&r, "GET", "/api/v1/values/RW_X.mass", None).await;
    assert!((v["value"]["value"]["nominal"]["si"].as_f64().unwrap() - 0.7).abs() < 1e-12);
}

#[tokio::test]
async fn invalid_op_is_400() {
    let (_tmp, r) = demo_router();
    let op = json!([{ "op": "remove_part", "id": "NOPE" }]);
    let (s, e) = call(&r, "POST", "/api/v1/ops", Some(json!({ "ops": op }))).await;
    assert!(
        s == StatusCode::NOT_FOUND || s == StatusCode::BAD_REQUEST,
        "{s} {e}"
    );
    assert!(e["error"]["code"].is_string());
}

/// 名詞のコマンドの引数（`POST /ops/from_command`）: 読めれば Op の列、読めない文字列は入力の誤りで、書き込まない。
#[tokio::test]
async fn command_text_becomes_ops_or_400() {
    let (_tmp, r) = demo_router();
    let body = json!({ "command": "net_set", "net": "P5V", "rail": "5.1 V" });
    let (s, ops) = call(&r, "POST", "/api/v1/ops/from_command", Some(body)).await;
    assert_eq!(s, StatusCode::OK, "{ops}");
    assert_eq!(ops[0]["op"], "set_net_field", "{ops}");
    assert_eq!(ops[0]["field"]["value"]["unit"], "V", "{ops}");
    let bad = json!({ "command": "mode_set", "cells": [
        { "mode": "NOMINAL", "part": "UHF_TRX", "state": "rx", "duty": { "tx": "ten" } }] });
    let (s, e) = call(&r, "POST", "/api/v1/ops/from_command", Some(bad)).await;
    assert!(s.is_client_error(), "{s} {e}");
    assert_eq!(e["error"]["code"], "invalid_input", "{e}");
}

#[tokio::test]
async fn analyses_run_and_budgets() {
    let (_tmp, r) = demo_router();
    let (s, out) = call(
        &r,
        "POST",
        "/api/v1/analyses/power.mode_load",
        Some(json!({})),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{out}");
    assert!(["ok", "missing_inputs", "failed"].contains(&out["status"].as_str().unwrap()));
    let (s, _) = call(&r, "POST", "/api/v1/analyses/nope.nope", Some(json!({}))).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (s, b) = call(&r, "GET", "/api/v1/budgets/mass", None).await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["kind"], "mass");
    assert_eq!(b["total"]["dimension"], "Mass");
}

#[tokio::test]
async fn data_status() {
    let (_tmp, r) = demo_router();
    let (s, d) = call(&r, "GET", "/api/v1/data/status", None).await;
    assert_eq!(s, StatusCode::OK, "{d}");
    assert_eq!(d["items"][0]["id"], "de440");
    let (s, e) = call(&r, "POST", "/api/v1/data/nope/fetch", None).await;
    assert_eq!(s, StatusCode::NOT_FOUND, "{e}");
}

#[tokio::test]
async fn units_format_and_views() {
    let (_tmp, r) = demo_router();
    let body = json!({ "items": [
        { "si": 0.0123, "dimension": "Power" },
        { "si": 2.0e6, "dimension": "Information", "unit": "Mbit" } ] });
    let (s, v) = call(&r, "POST", "/api/v1/units/format", Some(body)).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    // ほかの API と同じ量の表示（`QuantityView`）
    assert_eq!(v["items"][0]["display"], "12.3 mW");
    assert_eq!(v["items"][0]["si"], 0.0123);
    assert_eq!(v["items"][1]["display"], "2.00 Mbit");
    assert_eq!(v["items"][1]["unit"], "Mbit");
    let op = json!([{ "op": "set_view_layout", "view": "electrical",
        "nodes": [{ "id": "RW_X", "x": 10, "y": 20 }] }]);
    let (s, _) = call(&r, "POST", "/api/v1/ops", Some(json!({ "ops": op }))).await;
    assert_eq!(s, StatusCode::OK);
    let (_, d) = call(&r, "GET", "/api/v1/design", None).await;
    let view = d["views"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == "electrical")
        .unwrap();
    assert_eq!(view["nodes"]["RW_X"]["x"], 10);
}

async fn status_with_host(r: &axum::Router, host: &str) -> (StatusCode, serde_json::Value) {
    use axum::body::{Body, to_bytes};
    use tower::ServiceExt;
    let req = axum::http::Request::get("/api/v1/units?dimension=Mass")
        .header("host", host)
        .body(Body::empty())
        .unwrap();
    let res = r.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or_default())
}

#[tokio::test]
async fn host_allowlist_blocks_dns_rebinding() {
    let (_tmp, r) = demo_router();
    for ok in [
        "localhost:8787",
        "127.0.0.1:8787",
        "[::1]:8787",
        "localhost",
    ] {
        let (s, _) = status_with_host(&r, ok).await;
        assert_eq!(s, StatusCode::OK, "{ok}");
    }
    for bad in ["evil.example", "evil.example:8787", "100.64.0.1:8787"] {
        let (s, v) = status_with_host(&r, bad).await;
        assert_eq!(s, StatusCode::FORBIDDEN, "{bad}");
        assert_eq!(v["error"]["code"], "host_not_allowed");
    }
}

#[tokio::test]
async fn allow_host_extends_the_list() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("demo-6u");
    common::copy_dir(&common::fixture(), &root);
    let ws = std::sync::Arc::new(fune_service::Workspace::open(&root).unwrap());
    let hosts =
        fune_server::HostPolicy::for_bind("0.0.0.0".parse().unwrap(), &["100.64.0.1".into()]);
    let r = fune_server::router_with(ws, hosts);
    let (s, _) = status_with_host(&r, "100.64.0.1:8787").await;
    assert_eq!(s, StatusCode::OK);
    let (s, _) = status_with_host(&r, "evil.example").await;
    assert_eq!(s, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn place_body_converts_to_mount_local() {
    let (_tmp, r) = demo_router();
    let (_, before) = call(&r, "GET", "/api/v1/parts/RW_X", None).await;
    let on = before["part"]["mount"]["on"].clone();
    let target = json!({ "x": 10.0, "y": -20.0, "z": 30.0, "unit": "mm" });
    let body = json!({ "position": target });
    let (s, res) = call(&r, "POST", "/api/v1/parts/RW_X/place_body", Some(body)).await;
    assert_eq!(s, StatusCode::OK, "{res}");
    assert!(
        res["cli"][0]
            .as_str()
            .unwrap()
            .starts_with("fune part place RW_X --on")
    );
    let (_, after) = call(&r, "GET", "/api/v1/parts/RW_X", None).await;
    assert_eq!(after["part"]["mount"]["on"], on, "取付先は変えない");
    let p = after["part"]["mount"]["body_position"].as_array().unwrap();
    for (got, want) in p.iter().zip([0.010, -0.020, 0.030]) {
        assert!((got.as_f64().unwrap() - want).abs() < 1e-6, "{p:?}");
    }
    let (s, e) = call(
        &r,
        "POST",
        "/api/v1/parts/NOPE/place_body",
        Some(json!({ "position": target })),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND, "{e}");
}

#[tokio::test]
async fn format_check_is_read_only_and_apply_is_idempotent() {
    let (tmp, router) = demo_router();
    let path = tmp.path().join("demo-6u/parts/AOCS/RW_X.toml");
    let original = std::fs::read_to_string(&path).unwrap();
    let messy = format!("\n\n{original}\n\n");
    std::fs::write(&path, &messy).unwrap();
    let (s, v) = call(&router, "POST", "/api/v1/fmt", Some(json!({"check": true}))).await;
    assert_eq!(s, StatusCode::OK);
    assert!(
        v["changed"]
            .as_array()
            .unwrap()
            .contains(&json!("parts/AOCS/RW_X.toml"))
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), messy);
    let (s, applied) = call(&router, "POST", "/api/v1/fmt", Some(json!({}))).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(applied["changed"], v["changed"]);
    assert_ne!(std::fs::read_to_string(&path).unwrap(), messy);
    let (_, clean) = call(&router, "POST", "/api/v1/fmt", Some(json!({"check": true}))).await;
    assert_eq!(clean["changed"], json!([]));
}
