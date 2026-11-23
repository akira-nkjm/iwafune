//! パリティテスト（CI 必須）: フェーズの編集と一覧。GUI は編集の欄に打った文字列を `fune phase add|set|set-field` と同じ形
//! （`CommandText::PhaseAdd`・`PhaseSet`・`PhaseField`・`CasePhase`）で `POST /ops/from_command` に送り、`POST /ops` で保存する。
//! 読み方は CLI と同じ service の関数なので、書かれるファイルはバイト単位で一致する。一覧は `GET /phases` と
//! `fune --json phase ls` が同じ。

mod common;

use common::{assert_same_files, demo_copy, fune, get_json, post_json, run_cli};
use serde_json::{Value, json};
use std::path::Path;

fn http_command(rt: &tokio::runtime::Runtime, root: &Path, body: Value) -> Vec<Value> {
    let ops = post_json(rt, root, "/api/v1/ops/from_command", body);
    post_json(rt, root, "/api/v1/ops", json!({ "ops": ops.clone() }));
    ops.as_array().cloned().unwrap_or_default()
}

#[test]
fn phase_edits_match_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let edits: Vec<(Vec<&str>, Value, &str)> = vec![
        (
            vec![
                "phase",
                "add",
                "PH_TEST",
                "--label",
                "Test phase",
                "--order",
                "7",
                "--start",
                "Start of the test",
                "--duration",
                "10 d",
                "--modes",
                "SAFE,NOMINAL",
            ],
            json!({ "command": "phase_add", "id": "PH_TEST", "label": "Test phase",
                "order": "7", "start": "Start of the test", "end": "", "duration": "10 d",
                "modes": "SAFE,NOMINAL" }),
            "create_phase",
        ),
        (
            vec!["phase", "set-field", "PH_TEST", "orbit.altitude", "420 km"],
            json!({ "command": "phase_field", "phase": "PH_TEST", "path": "orbit.altitude",
                "value": "420 km" }),
            "set_phase_field",
        ),
        (
            vec![
                "phase",
                "set-field",
                "PH_TEST",
                "orbit.epoch",
                "2028-01-01T00:00:00Z",
            ],
            json!({ "command": "phase_field", "phase": "PH_TEST", "path": "orbit.epoch",
                "value": "2028-01-01T00:00:00Z" }),
            "set_phase_field",
        ),
        (
            vec![
                "phase",
                "set-field",
                "PH_TEST",
                "law.NOMINAL.law",
                "nadir_pointing",
            ],
            json!({ "command": "phase_field", "phase": "PH_TEST", "path": "law.NOMINAL.law",
                "value": "nadir_pointing" }),
            "set_phase_field",
        ),
        // `set` は見出しの欄だけを書き換え、軌道と姿勢則は残す
        (
            vec![
                "phase",
                "set",
                "PH_TEST",
                "--label",
                "Renamed label",
                "--order",
                "8",
            ],
            json!({ "command": "phase_set", "id": "PH_TEST", "label": "Renamed label",
                "order": "8" }),
            "set_phase",
        ),
        (
            vec!["case", "set-phase", "hot_imaging", "PH_TEST"],
            json!({ "command": "case_phase", "case": "hot_imaging", "phase": "PH_TEST" }),
            "set_case_phase",
        ),
        (
            vec!["case", "unset-phase", "hot_imaging"],
            json!({ "command": "case_phase", "case": "hot_imaging" }),
            "set_case_phase",
        ),
        (
            vec!["phase", "unset-field", "PH_TEST", "orbit.epoch"],
            json!({ "command": "phase_field", "phase": "PH_TEST", "path": "orbit.epoch" }),
            "set_phase_field",
        ),
    ];
    for (cli, body, op) in edits {
        run_cli(&cli_root, &cli);
        let ops = http_command(&rt, &http_root, body);
        assert!(ops.iter().all(|o| o["op"] == op), "{ops:?}");
        assert_same_files(&cli_root, &http_root, &cli.join(" "));
    }
    let text = std::fs::read_to_string(http_root.join("phases/PH_TEST.toml")).unwrap();
    assert!(
        text.contains("Renamed label")
            && text.contains("[orbit]")
            && text.contains("[law.NOMINAL]"),
        "{text}"
    );
    assert!(!text.contains("epoch"), "{text}");
    // 名前の変更・削除は Op をそのまま送る
    run_cli(&cli_root, &["phase", "rename", "PH_TEST", "PH_NEW"]);
    run_cli(&cli_root, &["phase", "rm", "PH_NEW"]);
    post_json(
        &rt,
        &http_root,
        "/api/v1/ops",
        json!({ "ops": [{ "op": "rename_phase", "id": "PH_TEST", "to": "PH_NEW" },
            { "op": "remove_phase", "id": "PH_NEW" }] }),
    );
    assert_same_files(&cli_root, &http_root, "phase rename + rm");
    assert!(!http_root.join("phases/PH_NEW.toml").exists());
}

/// 一覧（並びの順・軌道と姿勢則の上書き・編集の欄の初期値）は CLI の JSON と HTTP で同じ。
#[test]
fn phase_list_matches_http() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t, root) = demo_copy();
    run_cli(
        &root,
        &[
            "phase",
            "add",
            "PH_FIRST",
            "--order",
            "-5",
            "--duration",
            "2 d",
        ],
    );
    let http = get_json(&rt, &root, "/api/v1/phases");
    let out = fune()
        .arg("-C")
        .arg(&root)
        .args(["--json", "phase", "ls"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let cli: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(cli, http);
    let ids: Vec<&str> = http
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["PH_FIRST", "LEOP", "NOMINAL_OPS", "DISPOSAL"]);
    let first = &http[0];
    assert_eq!(first["position"], 1);
    assert_eq!(first["text"]["order"], "-5");
    assert_eq!(first["text"]["duration"], "2 d");
    assert_eq!(first["field_paths"]["duration"], "phases.PH_FIRST.duration");
    let disposal = &http[3];
    assert_eq!(disposal["orbit_overridden"], true);
    assert!(
        disposal["orbit"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["path"] == "orbit.altitude" && f["display"] == "250 km")
    );
    assert_eq!(http[2]["cases"].as_array().unwrap().len(), 3);
    // `show` は一覧の 1 件と同じ
    let out = fune()
        .arg("-C")
        .arg(&root)
        .args(["--json", "phase", "show", "DISPOSAL"])
        .output()
        .unwrap();
    let shown: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(&shown, disposal);
}

/// 上書きを 1 手で足す・消す操作（`phase copy-orbit|unset-orbit|unset-law`）も CLI と HTTP で同じファイルになる。
/// 姿勢則の上書きは、種類を書く `set-field law.<モード>.law` 1 手で作れる。
#[test]
fn phase_override_edits_match_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let edits: Vec<(Vec<&str>, Value, &str)> = vec![
        (
            vec!["phase", "add", "PH_TEST", "--modes", "SAFE,NOMINAL"],
            json!({ "command": "phase_add", "id": "PH_TEST", "modes": "SAFE,NOMINAL" }),
            "create_phase",
        ),
        (
            vec!["phase", "copy-orbit", "PH_TEST"],
            json!({ "command": "phase_copy_orbit", "phase": "PH_TEST" }),
            "copy_phase_orbit",
        ),
        (
            vec!["phase", "set-field", "PH_TEST", "orbit.altitude", "420 km"],
            json!({ "command": "phase_field", "phase": "PH_TEST", "path": "orbit.altitude",
                "value": "420 km" }),
            "set_phase_field",
        ),
        (
            vec![
                "phase",
                "set-field",
                "PH_TEST",
                "law.SAFE.law",
                "sun_pointing",
            ],
            json!({ "command": "phase_field", "phase": "PH_TEST", "path": "law.SAFE.law",
                "value": "sun_pointing" }),
            "set_phase_field",
        ),
        (
            vec!["phase", "unset-law", "PH_TEST", "SAFE"],
            json!({ "command": "phase_unset_law", "phase": "PH_TEST", "mode": "SAFE" }),
            "unset_phase_law",
        ),
        (
            vec!["phase", "unset-orbit", "PH_TEST"],
            json!({ "command": "phase_unset_orbit", "phase": "PH_TEST" }),
            "unset_phase_orbit",
        ),
    ];
    for (cli, body, op) in edits {
        run_cli(&cli_root, &cli);
        let ops = http_command(&rt, &http_root, body);
        assert!(ops.iter().all(|o| o["op"] == op), "{ops:?}");
        assert_same_files(&cli_root, &http_root, &cli.join(" "));
    }
    let text = std::fs::read_to_string(http_root.join("phases/PH_TEST.toml")).unwrap();
    assert!(
        !text.contains("[orbit]") && !text.contains("[law"),
        "{text}"
    );
    // 写した軌道は設計の軌道と同じ値で、`GET /phases` が欄の値（編集の欄の元）を返す
    run_cli(&cli_root, &["phase", "copy-orbit", "PH_TEST"]);
    http_command(
        &rt,
        &http_root,
        json!({ "command": "phase_copy_orbit", "phase": "PH_TEST" }),
    );
    let phases = get_json(&rt, &http_root, "/api/v1/phases");
    let test = phases
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "PH_TEST")
        .unwrap();
    let altitude = test["orbit"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["path"] == "orbit.altitude")
        .unwrap();
    assert_eq!(altitude["display"], "550 km");
    assert_eq!(altitude["value"]["type"], "scalar");
}
