//! パリティテスト（CI 必須）: 共有カタログの出所を足す・外す。`fune catalog source add|rm` と HTTP（足すのは GUI の欄に打った
//! 文字列から `POST /ops/from_command` で作った Op、外すのは `POST /ops`）で、書かれる fune.toml がバイト単位で一致する。
//! 拒まれる編集は CLI で終了コード 2、HTTP で 4xx になり、ファイルは変わらない。取得はしない（ネットワークを使わない）。

mod common;

use axum::body::Body;
use axum::http::Request;
use common::{assert_same_files, demo_copy, fune, post_json, run_cli, snapshot_files};
use serde_json::{Value, json};
use std::path::Path;
use tower::ServiceExt;

/// 打った文字列 → Op の列（HTTP）→ 適用（HTTP）。
fn http_command(rt: &tokio::runtime::Runtime, root: &Path, body: Value) {
    let ops = post_json(rt, root, "/api/v1/ops/from_command", body);
    post_json(rt, root, "/api/v1/ops", json!({ "ops": ops }));
}

fn http_ops(rt: &tokio::runtime::Runtime, root: &Path, ops: Value) {
    post_json(rt, root, "/api/v1/ops", json!({ "ops": ops }));
}

#[test]
fn add_and_remove_match_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let original = std::fs::read(cli_root.join("fune.toml")).unwrap();

    run_cli(
        &cli_root,
        &[
            "catalog",
            "source",
            "add",
            "lab",
            "--git",
            "https://example.com/catalog.git",
            "--rev",
            "main",
        ],
    );
    // GUI の欄の前後の空白は service が除く
    http_command(
        &rt,
        &http_root,
        json!({ "command": "catalog_source_add", "name": " lab ",
            "git": "https://example.com/catalog.git ", "rev": "main" }),
    );
    assert_same_files(&cli_root, &http_root, "catalog source add lab");

    // rev を省く（空の欄は省いたのと同じ）
    run_cli(
        &cli_root,
        &[
            "catalog",
            "source",
            "add",
            "team",
            "--git",
            "file:///srv/git/catalog.git",
        ],
    );
    http_command(
        &rt,
        &http_root,
        json!({ "command": "catalog_source_add", "name": "team",
            "git": "file:///srv/git/catalog.git", "rev": "" }),
    );
    assert_same_files(&cli_root, &http_root, "catalog source add team");
    let text = std::fs::read_to_string(cli_root.join("fune.toml")).unwrap();
    assert!(
        text.contains("name = \"team\"\ngit  = \"file:///srv/git/catalog.git\"\n"),
        "{text}"
    );

    for name in ["lab", "team"] {
        run_cli(&cli_root, &["catalog", "source", "rm", name]);
        http_ops(
            &rt,
            &http_root,
            json!([{ "op": "remove_catalog_source", "name": name }]),
        );
        assert_same_files(&cli_root, &http_root, &format!("catalog source rm {name}"));
    }
    assert_eq!(
        std::fs::read(cli_root.join("fune.toml")).unwrap(),
        original,
        "足して外すと元に戻る"
    );
    run_cli(&cli_root, &["fmt", "--check"]);
}

/// 足したあとは取得の案内を出す（足すだけでは取得しない）。
#[test]
fn add_prints_the_fetch_hint() {
    let (_t, root) = demo_copy();
    let out = fune()
        .arg("-C")
        .arg(&root)
        .args([
            "catalog",
            "source",
            "add",
            "lab",
            "--git",
            "https://example.com/catalog.git",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("fune catalog fetch"), "{stdout}");
    assert!(
        !root.join("catalog.lock").exists(),
        "足すだけでは取得しない"
    );
}

/// このプロジェクトのカタログ（path = …）は外せない。CLI は終了コード 2、HTTP は 4xx で、ファイルは変わらない。
#[test]
fn refused_remove_changes_nothing() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let before = snapshot_files(&cli_root);
    let out = fune()
        .arg("-C")
        .arg(&cli_root)
        .args(["catalog", "source", "rm", "local"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("外せません"));
    let ws = fune_service::Workspace::open(&http_root).unwrap();
    let router = fune_server::router(std::sync::Arc::new(ws));
    let body = json!({ "ops": [{ "op": "remove_catalog_source", "name": "local" }] }).to_string();
    let req = Request::post("/api/v1/ops")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    let status = rt.block_on(async { router.oneshot(req).await.unwrap().status() });
    assert!(status.is_client_error(), "{status}");
    assert_eq!(snapshot_files(&cli_root), before);
    assert_eq!(snapshot_files(&http_root), before);
}
