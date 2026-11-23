//! パリティテスト（CI 必須）: `fune.toml` の `[project]` の書き換え（`fune project set|unset`）と、このプロジェクトの中のカタログの
//! 追加・削除（`fune catalog source add-local|rm`）。CLI と HTTP（打った文字列から `POST /ops/from_command`、外すのは `POST /ops`）で、
//! 書かれるファイルがバイト単位で一致し、`fune fmt --check`・`fune check` に通る。拒まれる編集は CLI で終了コード 2、HTTP で 4xx に
//! なり、ファイルは変わらない。

mod common;

use axum::body::Body;
use axum::http::Request;
use common::{assert_same_files, demo_copy, fune, post_json, run_cli, snapshot_files};
use serde_json::{Value, json};
use std::path::Path;
use tower::ServiceExt;

fn http_command(rt: &tokio::runtime::Runtime, root: &Path, body: Value) {
    let ops = post_json(rt, root, "/api/v1/ops/from_command", body);
    post_json(rt, root, "/api/v1/ops", json!({ "ops": ops }));
}

fn http_status(rt: &tokio::runtime::Runtime, root: &Path, uri: &str, body: Value) -> u16 {
    let ws = fune_service::Workspace::open(root).unwrap();
    let router = fune_server::router(std::sync::Arc::new(ws));
    let req = Request::post(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    rt.block_on(async { router.oneshot(req).await.unwrap().status().as_u16() })
}

#[test]
fn project_set_and_unset_match_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let original = snapshot_files(&cli_root);

    run_cli(&cli_root, &["project", "set", "name", "renamed-demo"]);
    http_command(
        &rt,
        &http_root,
        json!({ "command": "project_set", "field": " name", "value": " renamed-demo " }),
    );
    assert_same_files(&cli_root, &http_root, "project set name");

    run_cli(&cli_root, &["project", "set", "label", "Renamed demo"]);
    http_command(
        &rt,
        &http_root,
        json!({ "command": "project_set", "field": "label", "value": "Renamed demo" }),
    );
    assert_same_files(&cli_root, &http_root, "project set label");
    run_cli(&cli_root, &["fmt", "--check"]);
    run_cli(&cli_root, &["check"]);
    let text = std::fs::read_to_string(cli_root.join("fune.toml")).unwrap();
    assert!(
        text.starts_with("# IWAFUNE 6U (demo)"),
        "the leading comment stays: {text}"
    );

    run_cli(&cli_root, &["project", "unset", "label"]);
    http_command(
        &rt,
        &http_root,
        json!({ "command": "project_set", "field": "label" }),
    );
    assert_same_files(&cli_root, &http_root, "project unset label");
    run_cli(&cli_root, &["fmt", "--check"]);

    // 元の値に戻すと、元のバイト列に戻る
    run_cli(&cli_root, &["project", "set", "name", "iwafune-6u-demo"]);
    run_cli(&cli_root, &["project", "set", "label", "IWAFUNE 6U (demo)"]);
    assert_eq!(snapshot_files(&cli_root), original);

    // 拒まれる編集は何も書かない
    let before = snapshot_files(&cli_root);
    for args in [["project", "unset", "name"], ["project", "set", "schema"]] {
        let mut args = args.to_vec();
        if args[1] == "set" {
            args.push("3");
        }
        let out = fune()
            .arg("-C")
            .arg(&cli_root)
            .args(&args)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2), "{args:?}");
    }
    let status = http_status(
        &rt,
        &http_root,
        "/api/v1/ops/from_command",
        json!({ "command": "project_set", "field": "name" }),
    );
    assert!((400..500).contains(&status), "{status}");
    let status = http_status(
        &rt,
        &http_root,
        "/api/v1/ops",
        json!({ "ops": [{ "op": "set_project_field", "field": { "field": "name", "value": " " } }] }),
    );
    assert!((400..500).contains(&status), "{status}");
    assert_eq!(snapshot_files(&cli_root), before);
}

#[test]
fn local_catalog_add_and_remove_match_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let original = snapshot_files(&cli_root);

    run_cli(
        &cli_root,
        &[
            "catalog",
            "source",
            "add-local",
            "vendor",
            "--path",
            "vendor_parts",
        ],
    );
    http_command(
        &rt,
        &http_root,
        json!({ "command": "catalog_local_add", "name": " vendor ", "path": "vendor_parts " }),
    );
    assert_same_files(&cli_root, &http_root, "catalog source add-local");
    run_cli(&cli_root, &["fmt", "--check"]);
    run_cli(&cli_root, &["check"]);

    // 追加したカタログに品を足せる（ディレクトリは品を足すときに作られる）
    run_cli(
        &cli_root,
        &["catalog", "add", "vendor/x_one", "--kind", "wire"],
    );
    assert!(cli_root.join("vendor_parts/x_one.toml").exists());
    let before = snapshot_files(&cli_root);
    let out = fune()
        .arg("-C")
        .arg(&cli_root)
        .args(["catalog", "source", "rm", "vendor"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("1 item"));
    assert_eq!(snapshot_files(&cli_root), before);
    run_cli(&cli_root, &["catalog", "rm", "vendor/x_one"]);

    run_cli(&cli_root, &["catalog", "source", "rm", "vendor"]);
    let status = http_status(
        &rt,
        &http_root,
        "/api/v1/ops",
        json!({ "ops": [{ "op": "remove_catalog_source", "name": "vendor" }] }),
    );
    assert_eq!(status, 200);
    assert_same_files(&cli_root, &http_root, "catalog source rm (local)");
    assert_eq!(
        snapshot_files(&cli_root),
        original,
        "add then remove restores the design"
    );
}

/// 規約が使うディレクトリ・ほかのカタログと重なるディレクトリは拒まれ、何も書かない。
#[test]
fn refused_local_catalogs_change_nothing() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let before = snapshot_files(&cli_root);
    for path in ["parts", "catalog/sub", "../outside"] {
        let out = fune()
            .arg("-C")
            .arg(&cli_root)
            .args(["catalog", "source", "add-local", "vendor", "--path", path])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2), "{path}");
        let status = http_status(
            &rt,
            &http_root,
            "/api/v1/ops",
            json!({ "ops": [{ "op": "add_local_catalog", "name": "vendor", "path": path }] }),
        );
        assert!((400..500).contains(&status), "{path}: {status}");
    }
    assert_eq!(snapshot_files(&cli_root), before);
    assert_eq!(snapshot_files(&http_root), before);
}
