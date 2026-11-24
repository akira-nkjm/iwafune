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

/// 能力の曲線の書き込み（`fune catalog set local/lv_alpha payload_mass_by_c3 "…"` と `POST /ops/from_text` の `item`）が、
/// バイト単位で同じファイルを書き、曲線の値が `[attr]` に 1 行で入る。
#[test]
fn capability_curve_edits_match_between_cli_and_http() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_a, cli) = demo_copy();
    let (_b, http) = demo_copy();
    let path = "payload_mass_by_c3";
    let value = "0:140 10:120.5 20:90 (km^2/s^2 -> kg)";
    run_cli(&cli, &["catalog", "set", "local/lv_alpha", path, value]);
    let op = http_text(
        &rt,
        &http,
        "/api/v1/ops/from_text",
        json!({ "item": "local/lv_alpha", "path": path, "value": value }),
    );
    assert_eq!(op["value"]["type"], "curve", "{op}");
    assert_same_files(&cli, &http, "catalog set payload_mass_by_c3");
    let item = std::fs::read_to_string(http.join("catalog/lv_alpha.toml")).unwrap();
    assert!(
        item.contains(
            "{ at = [0.0, 10.0, 20.0], at_unit = \"km^2/s^2\", values = [140.0, 120.5, 90.0], unit = \"kg\" }"
        ),
        "{item}"
    );
    // 書いたものを `fmt --check` と `check` が通す
    run_cli(&cli, &["fmt", "--check"]);
    run_cli(&cli, &["check"]);
}
