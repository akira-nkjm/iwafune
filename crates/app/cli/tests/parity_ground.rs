//! パリティテスト（CI 必須）: 地上局の追加・名前の変更・削除と、属性・種別の名前の変更。CLI（`fune ground add|rename|rm`・
//! `fune attr rename`・`fune kind rename`）と HTTP（足すのは GUI の欄に打った文字列から `POST /ops/from_command`、
//! ほかは `POST /ops`）で、書かれるファイルがバイト単位で一致する。拒まれる編集は CLI で終了コード 2、HTTP で 4xx になり、
//! ファイルは変わらない。

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

fn http_ops(rt: &tokio::runtime::Runtime, root: &Path, ops: Value) {
    post_json(rt, root, "/api/v1/ops", json!({ "ops": ops }));
}

/// 足す（GUI の欄の前後の空白・空の欄は service が読む）→ 名前を変える → 消す。消したあとは元の設計に戻る。
#[test]
fn ground_station_add_rename_remove_match_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let original = snapshot_files(&cli_root);

    run_cli(
        &cli_root,
        &[
            "ground",
            "add",
            "SVALBARD_GS",
            "--label",
            "Svalbard (example)",
            "--latitude",
            "78.2 deg",
            "--longitude",
            "15.4 deg",
            "--altitude",
            "450 m",
            "--min-elevation",
            "5 deg",
        ],
    );
    http_command(
        &rt,
        &http_root,
        json!({ "command": "ground_add", "id": " SVALBARD_GS ", "label": "Svalbard (example)",
            "latitude": "78.2 deg", "longitude": " 15.4 deg", "altitude": "450 m",
            "min_elevation": "5 deg" }),
    );
    assert_same_files(&cli_root, &http_root, "ground add");
    run_cli(&cli_root, &["fmt", "--check"]);

    run_cli(&cli_root, &["ground", "rename", "SVALBARD_GS", "KSAT_GS"]);
    http_ops(
        &rt,
        &http_root,
        json!([{ "op": "rename_ground_station", "id": "SVALBARD_GS", "to": "KSAT_GS" }]),
    );
    assert_same_files(&cli_root, &http_root, "ground rename");

    // 解析が挙げている局の名前の変更は `stations` も書き換える
    run_cli(&cli_root, &["ground", "rename", "TOKYO_GS", "TOKYO_2"]);
    http_ops(
        &rt,
        &http_root,
        json!([{ "op": "rename_ground_station", "id": "TOKYO_GS", "to": "TOKYO_2" }]),
    );
    assert_same_files(&cli_root, &http_root, "ground rename (listed)");
    run_cli(&cli_root, &["ground", "rename", "TOKYO_2", "TOKYO_GS"]);
    http_ops(
        &rt,
        &http_root,
        json!([{ "op": "rename_ground_station", "id": "TOKYO_2", "to": "TOKYO_GS" }]),
    );

    run_cli(&cli_root, &["ground", "rm", "KSAT_GS"]);
    http_ops(
        &rt,
        &http_root,
        json!([{ "op": "remove_ground_station", "id": "KSAT_GS" }]),
    );
    assert_same_files(&cli_root, &http_root, "ground rm");
    assert_eq!(
        snapshot_files(&cli_root),
        original,
        "add then remove restores the design"
    );
    run_cli(&cli_root, &["fmt", "--check"]);
    run_cli(&cli_root, &["check"]);
}

/// 属性と種別の名前の変更（部品・カタログ品・種別の参照も書き換わる）。
#[test]
fn definition_rename_matches_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let original = snapshot_files(&cli_root);

    run_cli(
        &cli_root,
        &["attr", "rename", "linear_density", "mass_per_length"],
    );
    run_cli(&cli_root, &["kind", "rename", "wire", "cable"]);
    http_ops(
        &rt,
        &http_root,
        json!([
            { "op": "rename_attribute", "id": "linear_density", "to": "mass_per_length" },
            { "op": "rename_kind", "id": "wire", "to": "cable" },
        ]),
    );
    assert_same_files(&cli_root, &http_root, "attr and kind rename");
    run_cli(&cli_root, &["fmt", "--check"]);
    run_cli(&cli_root, &["check"]);

    // 戻すと元のバイト列になる
    run_cli(
        &cli_root,
        &["attr", "rename", "mass_per_length", "linear_density"],
    );
    run_cli(&cli_root, &["kind", "rename", "cable", "wire"]);
    assert_eq!(snapshot_files(&cli_root), original);
}

/// 拒まれる編集（挙げられている局の削除・組み込みの定義の名前の変更）。CLI は終了コード 2、HTTP は 4xx で、ファイルは変わらない。
#[test]
fn refused_edits_change_nothing() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let before = snapshot_files(&cli_root);
    let cases = [
        (
            vec!["ground", "rm", "TOKYO_GS"],
            json!({ "op": "remove_ground_station", "id": "TOKYO_GS" }),
        ),
        (
            vec!["attr", "rename", "mass", "weight"],
            json!({ "op": "rename_attribute", "id": "mass", "to": "weight" }),
        ),
        (
            vec!["kind", "rename", "component", "part"],
            json!({ "op": "rename_kind", "id": "component", "to": "part" }),
        ),
    ];
    for (args, op) in cases {
        let out = fune()
            .arg("-C")
            .arg(&cli_root)
            .args(&args)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        let ws = fune_service::Workspace::open(&http_root).unwrap();
        let router = fune_server::router(std::sync::Arc::new(ws));
        let body = json!({ "ops": [op] }).to_string();
        let req = Request::post("/api/v1/ops")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let status = rt.block_on(async { router.oneshot(req).await.unwrap().status() });
        assert!(status.is_client_error(), "{args:?}: {status}");
    }
    assert_eq!(snapshot_files(&cli_root), before);
    assert_eq!(snapshot_files(&http_root), before);
}
