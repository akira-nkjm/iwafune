//! パリティテスト（CI 必須）: 打ち上げ機・ディスペンサーの選択。`fune set conditions.launch.launcher …` と、GUI が使う
//! `POST /ops/from_text`（消すときは `/ops/unset_from_text`）が、バイト単位で同じファイルを書く。選んだあとの解析（`structure.launch_fit`）は
//! HTTP でも結果を返す。

mod common;

use common::{assert_same_files, demo_copy, post_json, run_cli};
use serde_json::{Value, json};
use std::path::Path;

/// 打った文字列 → Op（HTTP）→ 適用（HTTP）。
fn http_text(rt: &tokio::runtime::Runtime, root: &Path, uri: &str, body: Value) -> Value {
    let op = post_json(rt, root, uri, body);
    post_json(rt, root, "/api/v1/ops", json!({ "ops": [op.clone()] }));
    op
}

#[test]
fn launch_selection_matches_between_cli_and_http() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_a, cli) = demo_copy();
    let (_b, http) = demo_copy();
    // 先に外してから選び直す（欄が無い状態からの書き込みも通す）
    for key in ["launcher", "dispenser"] {
        let path = format!("conditions.launch.{key}");
        run_cli(&cli, &["unset", &path]);
        let op = http_text(
            &rt,
            &http,
            "/api/v1/ops/unset_from_text",
            json!({ "path": path }),
        );
        assert_eq!(op["op"], "set_setting", "{op}");
        assert_same_files(&cli, &http, &format!("unset {key}"));
    }
    for (key, value) in [
        ("launcher", "local/lv_alpha"),
        ("dispenser", "local/disp_6u_rs"),
    ] {
        let path = format!("conditions.launch.{key}");
        run_cli(&cli, &["set", &path, value]);
        http_text(
            &rt,
            &http,
            "/api/v1/ops/from_text",
            json!({ "path": path, "value": value }),
        );
        assert_same_files(&cli, &http, &format!("set {key}"));
    }
    let launch = std::fs::read_to_string(http.join("conditions/launch.toml")).unwrap();
    assert!(
        launch.contains("launcher               = \"local/lv_alpha\"\n"),
        "{launch}"
    );
    let fit = post_json(
        &rt,
        &http,
        "/api/v1/analyses/structure.launch_fit",
        json!({}),
    );
    assert_eq!(fit["status"], "ok", "{fit}");
    let keys: Vec<&str> = fit["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["key"].as_str().unwrap())
        .collect();
    assert!(
        keys.contains(&"mass") && keys.contains(&"envelope:z"),
        "{keys:?}"
    );
}
