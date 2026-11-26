//! パリティテスト（CI 必須）: GUI から開いて直す・消す編集。通信の回線の欄の書き換え（`fune set analysis.comm.link.<名前>.<欄>` と
//! `POST /ops/from_text`）、解析ケースの削除と、参照されているケースの扱い（`fune case rm|rename` と `POST /ops`）。
//! 書かれるファイルが CLI と HTTP でバイト単位で一致し、拒まれる編集はどちらもファイルを変えない。

mod common;

use axum::body::Body;
use axum::http::Request;
use common::{assert_same_files, demo_copy, fune, post_json, run_cli, snapshot_files};
use serde_json::json;
use tower::ServiceExt;

/// 回線の向き・部品・帯域を 1 欄ずつ書く（ほかの欄は残る）。
#[test]
fn link_field_edits_match_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    for (field, value) in [("direction", "up"), ("part", "UHF_TRX"), ("band", "uhf")] {
        let path = format!("analysis.comm.link.s_down.{field}");
        run_cli(&cli_root, &["set", &path, value]);
        let op = post_json(
            &rt,
            &http_root,
            "/api/v1/ops/from_text",
            json!({ "path": path, "value": value }),
        );
        post_json(&rt, &http_root, "/api/v1/ops", json!({ "ops": [op] }));
        assert_same_files(&cli_root, &http_root, &path);
    }
    let text = std::fs::read_to_string(http_root.join("analysis/comm.toml")).unwrap();
    assert!(text.contains("modulation          = \"QPSK\""), "{text}");
    run_cli(&cli_root, &["fmt", "--check"]);
}

/// 参照されていないケースは消せる。参照されている（要求の `quantify.cases`）ケースは、
/// CLI も HTTP も 参照元を挙げて拒み、ファイルは変わらない。
#[test]
fn case_removal_matches_cli_and_refuses_referenced_cases() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    run_cli(&cli_root, &["case", "rm", "cold_safe"]);
    post_json(
        &rt,
        &http_root,
        "/api/v1/ops",
        json!({ "ops": [{ "op": "remove_case", "id": "cold_safe" }] }),
    );
    assert_same_files(&cli_root, &http_root, "case rm");
    assert!(!http_root.join("cases/cold_safe.toml").exists());
    run_cli(&cli_root, &["check"]);

    let before = snapshot_files(&cli_root);
    let out = fune()
        .arg("-C")
        .arg(&cli_root)
        .args(["case", "rm", "hot_imaging"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("default case") && stderr.contains("R_STORAGE"),
        "{stderr}"
    );
    let ws = fune_service::Workspace::open(&http_root).unwrap();
    let router = fune_server::router(std::sync::Arc::new(ws));
    let body = json!({ "ops": [{ "op": "remove_case", "id": "hot_imaging" }] }).to_string();
    let req = Request::post("/api/v1/ops")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    let status = rt.block_on(async { router.oneshot(req).await.unwrap().status() });
    assert!(status.is_client_error(), "{status}");
    assert_eq!(snapshot_files(&cli_root), before);
    assert_eq!(snapshot_files(&http_root), before);
}

/// 参照されているケースの名前を変えると、要求の `quantify.cases` も書き換わる（CLI と HTTP で同じ）。
#[test]
fn case_rename_rewrites_referencing_requirements() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    run_cli(&cli_root, &["case", "rename", "hot_imaging", "hot_img"]);
    post_json(
        &rt,
        &http_root,
        "/api/v1/ops",
        json!({ "ops": [{ "op": "rename_case", "id": "hot_imaging", "to": "hot_img" }] }),
    );
    assert_same_files(&cli_root, &http_root, "case rename");
    let text = std::fs::read_to_string(http_root.join("requirements/R_STORAGE.toml")).unwrap();
    assert!(
        text.contains("hot_img") && !text.contains("hot_imaging"),
        "{text}"
    );
    run_cli(&cli_root, &["check"]);
}
