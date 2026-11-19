//! パリティテスト（CI 必須）: 熱の明示の結合（`[conductive.*]`・`[radiative.*]`）と外殻の放射率（`[enclosure]`）の編集。
//! 設定のパスは `analysis.thermal.conductive.<節点>.<節点>` の形で、汎用の設定の Op（`SetSetting`）がそのまま編集する。
//! CLI（`fune setting set`・`fune setting unset`）と HTTP（`POST /ops/from_text`・`/ops/unset_from_text`）で、
//! 書かれるファイルがバイト単位で一致することと、解析が使う結合の一覧に反映されることを確かめる。

mod common;

use common::{assert_same_files, demo_copy, post_json, run_cli};
use serde_json::{Value, json};
use std::path::Path;

fn http_text(rt: &tokio::runtime::Runtime, root: &Path, uri: &str, body: Value) {
    let op = post_json(rt, root, uri, body);
    post_json(rt, root, "/api/v1/ops", json!({ "ops": [op] }));
}

#[test]
fn thermal_couplings_match_http() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli) = demo_copy();
    let (_t2, http) = demo_copy();
    let cases = [
        ("analysis.thermal.enclosure.emissivity", "0.85"),
        ("analysis.thermal.conductive.BAT.PANEL_MZ", "0.8 W/K"),
        ("analysis.thermal.conductive.CAM.PANEL_PZ", "0 W/K"),
        ("analysis.thermal.radiative.PANEL_PX.PANEL_MX", "0.02 m^2"),
    ];
    for (path, value) in cases {
        run_cli(&cli, &["setting", "set", path, value]);
        http_text(
            &rt,
            &http,
            "/api/v1/ops/from_text",
            json!({ "path": path, "value": value }),
        );
        assert_same_files(&cli, &http, path);
    }
    let text = std::fs::read_to_string(http.join("analysis/thermal.toml")).unwrap();
    for expected in [
        "[enclosure]\nemissivity = 0.85",
        "[conductive.BAT]\nPANEL_MZ = { value = 0.8, unit = \"W/K\" }",
        "[radiative.PANEL_PX]\nPANEL_MX = { value = 0.02, unit = \"m^2\" }",
    ] {
        assert!(text.contains(expected), "{expected}\n{text}");
    }
    // 解析が使う結合の一覧に出どころとともに載る（取付の値は明示が置き換える）
    let result = post_json(&rt, &http, "/api/v1/analyses/thermal.steady", json!({}));
    let couplings = &result["output"]["couplings"];
    let find = |a: &str, b: &str| {
        couplings["conductive"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["a"] == a && l["b"] == b)
            .cloned()
            .unwrap()
    };
    assert_eq!(find("BAT", "PANEL_MZ")["source"], "explicit");
    assert_eq!(find("CAM", "PANEL_PZ")["conductance"], 0.0);
    assert_eq!(couplings["enclosure"]["applied"], true);
    // 消す
    let path = "analysis.thermal.conductive.BAT.PANEL_MZ";
    run_cli(&cli, &["setting", "unset", path]);
    http_text(
        &rt,
        &http,
        "/api/v1/ops/unset_from_text",
        json!({ "path": path }),
    );
    assert_same_files(&cli, &http, "unset conductive");
}

/// 検査: 同じ対を逆向きに 2 度書くと誤りで、書き込まれない（診断は `thermal.coupling_duplicate`）。
#[test]
fn duplicate_pair_is_refused() {
    let (_t, root) = demo_copy();
    run_cli(
        &root,
        &[
            "setting",
            "set",
            "analysis.thermal.conductive.BAT.PANEL_MZ",
            "0.8 W/K",
        ],
    );
    let before = std::fs::read_to_string(root.join("analysis/thermal.toml")).unwrap();
    let out = common::fune()
        .arg("-C")
        .arg(&root)
        .args([
            "setting",
            "set",
            "analysis.thermal.conductive.PANEL_MZ.BAT",
            "0.5 W/K",
        ])
        .output()
        .unwrap();
    let text =
        String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
    assert!(text.contains("thermal.coupling_duplicate"), "{text}");
    assert_ne!(out.status.code(), Some(0));
    let after = std::fs::read_to_string(root.join("analysis/thermal.toml")).unwrap();
    assert_eq!(before, after);
}
