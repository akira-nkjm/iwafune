//! パリティテスト（CI 必須）: 定義（属性・種別）の作成・置き換え・削除。`fune attr|kind add|replace|rm` と
//! HTTP（`POST /ops`、置き換えは打った文字列から `POST /ops/from_command` で作った Op）で、書かれるファイルが
//! バイト単位で一致する。拒まれる削除は CLI で終了コード 2、HTTP で 4xx になり、ファイルは変わらない。

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

/// 打った文字列 → Op の列（HTTP）→ 適用（HTTP）。
fn http_command(rt: &tokio::runtime::Runtime, root: &Path, body: Value) {
    let ops = post_json(rt, root, "/api/v1/ops/from_command", body);
    post_json(rt, root, "/api/v1/ops", json!({ "ops": ops }));
}

#[test]
fn definition_edits_match_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let steps: Vec<(Vec<&str>, Value)> = vec![
        (
            vec![
                "attr",
                "add",
                "max_dipole2",
                "--label",
                "最大双極子 2",
                "--dimension",
                "MagneticDipoleMoment",
                "--units",
                "A*m^2",
            ],
            json!([{ "op": "create_attribute", "def": { "id": "max_dipole2", "label": "最大双極子 2",
                "type": "scalar", "dimension": "MagneticDipoleMoment", "units": ["A*m^2"] } }]),
        ),
        (
            vec![
                "kind",
                "add",
                "wire2",
                "--label",
                "線材 2",
                "--states",
                "off,on",
                "--required",
                "mass",
            ],
            json!([{ "op": "create_kind", "def": { "id": "wire2", "label": "線材 2",
                "states": ["off", "on"], "required": ["mass"] } }]),
        ),
        (
            vec![
                "attr",
                "replace",
                "max_dipole2",
                "--label",
                "最大双極子 3",
                "--dimension",
                "MagneticDipoleMoment",
                "--units",
                "A*m^2,mA*m^2",
            ],
            json!([{ "op": "update_attribute", "def": { "id": "max_dipole2", "label": "最大双極子 3",
                "type": "scalar", "dimension": "MagneticDipoleMoment", "units": ["A*m^2", "mA*m^2"] } }]),
        ),
        (
            vec![
                "kind",
                "replace",
                "wire2",
                "--label",
                "線材 3",
                "--states",
                "off,on",
                "--required",
                "mass,power",
            ],
            json!([{ "op": "update_kind", "def": { "id": "wire2", "label": "線材 3",
                "states": ["off", "on"], "required": ["mass", "power"] } }]),
        ),
        (
            vec!["kind", "rm", "wire2"],
            json!([{ "op": "delete_kind", "id": "wire2" }]),
        ),
        (
            vec!["attr", "rm", "max_dipole2"],
            json!([{ "op": "delete_attribute", "id": "max_dipole2" }]),
        ),
    ];
    for (cli, ops) in steps {
        run_cli(&cli_root, &cli);
        http_ops(&rt, &http_root, ops);
        assert_same_files(&cli_root, &http_root, &cli.join(" "));
    }
}

/// GUI の欄に打った文字列からの置き換え（`POST /ops/from_command`）も CLI と同じファイルになる。
#[test]
fn replace_from_typed_text_matches_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    run_cli(
        &cli_root,
        &[
            "attr",
            "replace",
            "linear_density",
            "--label",
            "単位長さ質量",
            "--units",
            "g/m,kg/m",
            "--display-unit",
            "g/m",
            "--min",
            "0",
            "--section",
            "mass",
        ],
    );
    http_command(
        &rt,
        &http_root,
        json!({ "command": "attr_replace", "id": "linear_density", "label": "単位長さ質量",
            "units": ["g/m", "kg/m"], "display_unit": "g/m", "min": "0", "section": "mass" }),
    );
    assert_same_files(&cli_root, &http_root, "attr replace");
    run_cli(
        &cli_root,
        &[
            "kind",
            "replace",
            "wire",
            "--label",
            "線材",
            "--required",
            "linear_density,resistance_per_length",
        ],
    );
    http_command(
        &rt,
        &http_root,
        json!({ "command": "kind_replace", "id": "wire", "label": "線材",
            "required": "linear_density,resistance_per_length" }),
    );
    assert_same_files(&cli_root, &http_root, "kind replace");
}

/// 使われている定義の削除は、CLI が終了コード 2、HTTP が 4xx で拒み、ファイルは変わらない。
#[test]
fn refused_delete_changes_nothing() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let before = snapshot_files(&cli_root);
    let out = fune()
        .arg("-C")
        .arg(&cli_root)
        .args(["kind", "rm", "wire"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("消せません"));
    let ws = fune_service::Workspace::open(&http_root).unwrap();
    let router = fune_server::router(std::sync::Arc::new(ws));
    let body = json!({ "ops": [{ "op": "delete_kind", "id": "wire" }] }).to_string();
    let req = Request::post("/api/v1/ops")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    let status = rt.block_on(async { router.oneshot(req).await.unwrap().status() });
    assert!(status.is_client_error(), "{status}");
    assert_eq!(snapshot_files(&cli_root), before);
    assert_eq!(snapshot_files(&http_root), before);
}
