//! パリティテスト（CI 必須）: モード別の表示。`fune mode view <系>` の `--json` は、
//! HTTP（`POST /system_modes/{system}`）と同じ JSON。対応しない系は終了コード 2（HTTP は 4xx）、
//! 終了コードは `--json` でも表でも同じ。

mod common;

use common::{demo_copy, fune, get_json, post_json};
use serde_json::{Value, json};
use std::path::Path;

fn cli(root: &Path, args: &[&str]) -> (Option<i32>, Vec<u8>) {
    let out = fune().arg("-C").arg(root).args(args).output().unwrap();
    (out.status.code(), out.stdout)
}

fn cli_json(root: &Path, args: &[&str]) -> Value {
    let mut full = vec!["--json"];
    full.extend_from_slice(args);
    serde_json::from_slice(&cli(root, &full).1).unwrap()
}

/// 電源の解析を含む見方（`--analyzed`・`--case`）と合計だけの足もと（`--totals-only`）は HTTP の依頼と同じ JSON。
#[test]
fn analyzed_and_totals_options_equal_http() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t, root) = demo_copy();
    let uri = "/api/v1/system_modes/power";
    assert_eq!(
        cli_json(&root, &["mode", "view", "power", "--analyzed"]),
        post_json(&rt, &root, uri, json!({ "analyzed": true }))
    );
    let case = get_json(&rt, &root, "/api/v1/design")["cases"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        cli_json(
            &root,
            &["mode", "view", "power", "--analyzed", "--case", &case]
        ),
        post_json(&rt, &root, uri, json!({ "analyzed": true, "case": case }))
    );
    assert_eq!(
        cli_json(&root, &["mode", "view", "power", "--totals-only"]),
        post_json(&rt, &root, uri, json!({ "summary": "totals" }))
    );
    assert_eq!(
        cli(&root, &["mode", "view", "power", "--analyzed"]).0,
        Some(0)
    );
}

#[test]
fn mode_view_json_equals_http_for_every_mode_system() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t, root) = demo_copy();
    let defs = get_json(&rt, &root, "/api/v1/defs");
    let systems: Vec<String> = defs["mode_systems"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_owned())
        .collect();
    assert!(systems.contains(&"power".to_owned()) && !systems.contains(&"comm".to_owned()));
    for system in &systems {
        let http = post_json(
            &rt,
            &root,
            &format!("/api/v1/system_modes/{system}"),
            json!({ "ops": [] }),
        );
        assert_eq!(cli_json(&root, &["mode", "view", system]), http, "{system}");
        assert_eq!(cli(&root, &["mode", "view", system]).0, Some(0), "{system}");
    }
}

/// 保存していない編集（`--ops`）は HTTP の `ops` と同じに当たる。
#[test]
fn mode_view_with_unsaved_ops_equals_http() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t, root) = demo_copy();
    let ops = json!([{ "op": "set_setting", "path": "analysis.power.mode_share", "value": null }]);
    let file = root.join("ops.json");
    std::fs::write(&file, ops.to_string()).unwrap();
    let http = post_json(
        &rt,
        &root,
        "/api/v1/system_modes/power",
        json!({ "ops": ops }),
    );
    let file = file.to_str().unwrap();
    assert_eq!(
        cli_json(&root, &["mode", "view", "power", "--ops", file]),
        http
    );
}

#[test]
fn mode_view_rejects_a_system_without_a_mode_view() {
    let (_t, root) = demo_copy();
    for flags in [
        &["mode", "view", "comm"][..],
        &["--json", "mode", "view", "comm"],
    ] {
        assert_eq!(cli(&root, flags).0, Some(2), "{flags:?}");
    }
    let (code, out) = cli(&root, &["--json", "mode", "view", "comm"]);
    assert_eq!(code, Some(2));
    let v: Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["error"]["code"], "invalid_input");
    assert_eq!(cli(&root, &["mode", "view", "nope"]).0, Some(2));
}

/// 電力の上限を下げると違反になり、終了コードは表でも `--json` でも 1。
#[test]
fn violations_exit_with_one_for_table_and_json() {
    let (_t, root) = demo_copy();
    // 上限は要求が持つ（`R_POWER`）
    let file = root.join("requirements/R_POWER.toml");
    let text = std::fs::read_to_string(&file).unwrap();
    let tight = text.replace(
        "value = { value = 24.0, unit = \"W\" }",
        "value = { value = 5.0, unit = \"W\" }",
    );
    assert_ne!(text, tight);
    std::fs::write(&file, tight).unwrap();
    for flags in [
        &["mode", "view", "power"][..],
        &["--json", "mode", "view", "power"],
    ] {
        assert_eq!(cli(&root, flags).0, Some(1), "{flags:?}");
    }
}
