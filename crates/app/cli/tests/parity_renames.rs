//! パリティテスト（CI 必須）: ネット・ハーネス・解析ケース・出典・マヌーバの名前の変更と、通信の回線の追加・名前の変更・削除。
//! CLI（`fune net rename` など）と HTTP（`POST /ops`。回線を足すのは GUI の欄に打った文字列から `POST /ops/from_command`）で、
//! 書かれるファイルがバイト単位で一致し、`fune fmt --check`・`fune check` に通る。拒まれる編集は CLI で終了コード 2、HTTP で 4xx になり、
//! ファイルは変わらない。

mod common;

use axum::body::Body;
use axum::http::Request;
use common::{assert_same_files, demo_copy, fune, post_json, run_cli, snapshot_files};
use serde_json::{Value, json};
use std::path::Path;
use tower::ServiceExt;

fn http_ops(rt: &tokio::runtime::Runtime, root: &Path, ops: Value) {
    post_json(rt, root, "/api/v1/ops", json!({ "ops": ops }));
}

fn http_command(rt: &tokio::runtime::Runtime, root: &Path, body: Value) {
    let ops = post_json(rt, root, "/api/v1/ops/from_command", body);
    post_json(rt, root, "/api/v1/ops", json!({ "ops": ops }));
}

/// 名前を変える → 元に戻す（元のバイト列に戻る）。どの段階でも CLI と HTTP が同じファイルになる。
#[test]
fn renames_match_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let original = snapshot_files(&cli_root);
    for (noun, op, id, to) in [
        ("net", "rename_net", "P5V", "P5V_MAIN"),
        ("harness", "rename_harness", "H_OBC_CAM", "H_OBC_CAMERA"),
        ("case", "rename_case", "hot_imaging", "hot_img"),
        ("source", "rename_source", "sys_judgement", "sys_judge"),
        ("maneuver", "rename_maneuver", "RAISE", "M_RAISE"),
    ] {
        run_cli(&cli_root, &[noun, "rename", id, to]);
        http_ops(&rt, &http_root, json!([{ "op": op, "id": id, "to": to }]));
        assert_same_files(&cli_root, &http_root, &format!("{noun} rename"));
        run_cli(&cli_root, &["fmt", "--check"]);
        run_cli(&cli_root, &["check"]);
        assert_ne!(snapshot_files(&cli_root), original, "{noun}");
        run_cli(&cli_root, &[noun, "rename", to, id]);
        http_ops(&rt, &http_root, json!([{ "op": op, "id": to, "to": id }]));
        assert_same_files(&cli_root, &http_root, &format!("{noun} rename back"));
        assert_eq!(
            snapshot_files(&cli_root),
            original,
            "{noun}: rename and back"
        );
    }
}

/// 拒まれる名前の変更（既にある ID・不正な ID・無い ID）は何も書かない。
#[test]
fn refused_renames_change_nothing() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let before = snapshot_files(&cli_root);
    for (noun, op, id, to) in [
        ("net", "rename_net", "P5V", "P12V"),
        ("harness", "rename_harness", "H_OBC_CAM", "bad id"),
        ("case", "rename_case", "no_such_case", "fresh"),
        ("source", "rename_source", "sys_judgement", "ds_vendor"),
        ("maneuver", "rename_maneuver", "RAISE", "DEORBIT"),
    ] {
        let out = fune()
            .arg("-C")
            .arg(&cli_root)
            .args([noun, "rename", id, to])
            .output()
            .unwrap();
        assert!(!out.status.success(), "{noun}");
        // 終了コード: 2 は不正な入力・無い対象、3 は重なる ID
        assert!(
            matches!(out.status.code(), Some(2..=3)),
            "{noun}: {:?}",
            out.status
        );
        let ws = fune_service::Workspace::open(&http_root).unwrap();
        let router = fune_server::router(std::sync::Arc::new(ws));
        let body = json!({ "ops": [{ "op": op, "id": id, "to": to }] }).to_string();
        let req = Request::post("/api/v1/ops")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let status = rt.block_on(async { router.oneshot(req).await.unwrap().status() });
        assert!(status.is_client_error(), "{noun}: {status}");
        assert_eq!(snapshot_files(&cli_root), before, "{noun}");
        assert_eq!(snapshot_files(&http_root), before, "{noun}");
    }
}

/// 回線を足す（GUI の欄の前後の空白は service が除く）→ 名前を変える（要求の判定のキーも書き換わる）→ 消す。
#[test]
fn comm_link_add_rename_remove_match_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let original = snapshot_files(&cli_root);

    run_cli(
        &cli_root,
        &[
            "link",
            "add",
            "x_up",
            "--direction",
            "up",
            "--part",
            "UHF_TRX",
            "--band",
            "uhf",
        ],
    );
    http_command(
        &rt,
        &http_root,
        json!({ "command": "link_add", "id": " x_up ", "direction": "up", "part": "UHF_TRX ", "band": " uhf" }),
    );
    assert_same_files(&cli_root, &http_root, "link add");
    run_cli(&cli_root, &["fmt", "--check"]);
    run_cli(&cli_root, &["check"]);

    run_cli(&cli_root, &["link", "rename", "x_up", "x_ul"]);
    http_ops(
        &rt,
        &http_root,
        json!([{ "op": "rename_comm_link", "id": "x_up", "to": "x_ul" }]),
    );
    assert_same_files(&cli_root, &http_root, "link rename");

    // 要求の判定のキーが名指す回線の名前の変更は、そのキーも書き換える
    run_cli(&cli_root, &["link", "rename", "s_down", "s_dl"]);
    http_ops(
        &rt,
        &http_root,
        json!([{ "op": "rename_comm_link", "id": "s_down", "to": "s_dl" }]),
    );
    assert_same_files(&cli_root, &http_root, "link rename (checked)");
    let requirement = std::fs::read_to_string(cli_root.join("requirements/R_LINK_S.toml")).unwrap();
    assert!(
        requirement.contains("margin:s_dl@TOKYO_GS"),
        "{requirement}"
    );
    run_cli(&cli_root, &["fmt", "--check"]);
    run_cli(&cli_root, &["check"]);
    run_cli(&cli_root, &["link", "rename", "s_dl", "s_down"]);
    http_ops(
        &rt,
        &http_root,
        json!([{ "op": "rename_comm_link", "id": "s_dl", "to": "s_down" }]),
    );

    // 名指す要求があるあいだ消せない
    let before = snapshot_files(&cli_root);
    let out = fune()
        .arg("-C")
        .arg(&cli_root)
        .args(["link", "rm", "s_down"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("R_LINK_S"));
    assert_eq!(snapshot_files(&cli_root), before);

    run_cli(&cli_root, &["link", "rm", "x_ul"]);
    http_ops(
        &rt,
        &http_root,
        json!([{ "op": "remove_comm_link", "id": "x_ul" }]),
    );
    assert_same_files(&cli_root, &http_root, "link rm");
    assert_eq!(
        snapshot_files(&cli_root),
        original,
        "add then remove restores the design"
    );
}
