//! パリティテスト: 取り消し・やり直しを CLI（`fune undo` / `fune redo`）と HTTP（`POST /undo`・`POST /redo`）で行い、
//! 結果のファイルと返す JSON が一致することを確かめる。ほかで書き換わっていれば、どちらも同じ理由で断る。

mod common;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use common::{assert_same_files, demo_copy, fune, post_json, run_cli, snapshot_files};
use fune_service::Workspace;
use serde_json::{Value, json};
use std::path::Path;
use std::sync::Arc;
use tower::ServiceExt;

fn cli_json(root: &Path, command: &str) -> Value {
    let out = fune()
        .arg("-C")
        .arg(root)
        .arg("--json")
        .arg(command)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "fune {command} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

/// 同じ編集（質量の変更・部品の作成）を CLI と HTTP で行う。
fn edit_both(rt: &tokio::runtime::Runtime, cli: &Path, http: &Path) {
    run_cli(cli, &["set", "RW_X.mass", "650 g"]);
    post_json(
        rt,
        http,
        "/api/v1/ops",
        json!({ "ops": [{ "op": "set_value", "part": "RW_X", "attr": "mass",
            "value": { "type": "scalar", "data": { "value": 650.0, "unit": "g" } } }] }),
    );
    run_cli(
        cli,
        &[
            "part",
            "add",
            "RW_W",
            "--kind",
            "reaction_wheel",
            "--group",
            "aocs",
        ],
    );
    post_json(
        rt,
        http,
        "/api/v1/ops",
        json!({ "ops": [{ "op": "create_part", "id": "RW_W", "group": "aocs", "kind": "reaction_wheel" }] }),
    );
}

#[test]
fn undo_and_redo_match_between_cli_and_http() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_a, cli) = demo_copy();
    let (_b, http) = demo_copy();
    edit_both(&rt, &cli, &http);
    assert_same_files(&cli, &http, "the edits");
    let edited = snapshot_files(&cli);

    for step in ["undo", "undo", "redo", "redo"] {
        let a = cli_json(&cli, step);
        let b = post_json(&rt, &http, &format!("/api/v1/{step}"), Value::Null);
        assert_eq!(a, b, "{step} result differs");
        assert_same_files(&cli, &http, step);
    }
    assert_eq!(snapshot_files(&cli), edited);
}

#[test]
fn undo_goes_back_to_the_original_bytes() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_a, cli) = demo_copy();
    let (_b, http) = demo_copy();
    let original = snapshot_files(&cli);
    edit_both(&rt, &cli, &http);
    for _ in 0..2 {
        run_cli(&cli, &["undo"]);
        post_json(&rt, &http, "/api/v1/undo", Value::Null);
    }
    assert_eq!(snapshot_files(&cli), original);
    assert_eq!(snapshot_files(&http), original);
}

/// ファイルの履歴を通さない書き換えのあとは、どちらも同じ理由で断り、何も書かない。
#[test]
fn undo_is_refused_alike_after_an_outside_edit() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_a, cli) = demo_copy();
    let (_b, http) = demo_copy();
    edit_both(&rt, &cli, &http);
    for root in [&cli, &http] {
        let path = root.join("fune.toml");
        let mut text = std::fs::read_to_string(&path).unwrap();
        text.push_str("\n# edited by hand\n");
        std::fs::write(&path, text).unwrap();
    }
    let before = snapshot_files(&cli);
    let out = fune()
        .arg("-C")
        .arg(&cli)
        .arg("--json")
        .arg("undo")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let a: Value = serde_json::from_slice(&out.stdout).unwrap();

    let ws = Workspace::open(&http).unwrap();
    let router = fune_server::router(Arc::new(ws));
    let (status, body) = rt.block_on(async {
        let res = router
            .oneshot(Request::post("/api/v1/undo").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice::<Value>(&bytes).unwrap())
    });
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(a["error"]["code"], "history_conflict");
    assert_eq!(a["error"]["code"], body["error"]["code"]);
    assert_eq!(snapshot_files(&cli), before);
}

#[test]
fn nothing_to_undo_is_reported_alike() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_a, cli) = demo_copy();
    let out = fune()
        .arg("-C")
        .arg(&cli)
        .arg("--json")
        .arg("redo")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let a: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(a["error"]["code"], "history_empty");
    assert_eq!(a["error"]["message"], "Nothing to redo");
    let ws = Workspace::open(&cli).unwrap();
    let router = fune_server::router(Arc::new(ws));
    let status = rt.block_on(async {
        router
            .oneshot(Request::post("/api/v1/redo").body(Body::empty()).unwrap())
            .await
            .unwrap()
            .status()
    });
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[test]
fn history_listing_matches_between_cli_and_http() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_a, cli) = demo_copy();
    let (_b, http) = demo_copy();
    edit_both(&rt, &cli, &http);
    run_cli(&cli, &["undo"]);
    post_json(&rt, &http, "/api/v1/undo", Value::Null);
    let a = cli_json(&cli, "history");
    let b = common::get_json(&rt, &http, "/api/v1/history");
    // 時刻だけは実行ごとに違う
    let strip = |mut v: Value| {
        for e in v["entries"].as_array_mut().unwrap() {
            e["at"] = Value::Null;
        }
        v
    };
    assert_eq!(strip(a.clone()), strip(b));
    assert_eq!(a["can_undo"], true);
    assert_eq!(a["can_redo"], true);
    assert_eq!(a["entries"][1]["undone"], true);
}
