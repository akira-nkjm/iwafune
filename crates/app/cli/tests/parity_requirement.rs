//! パリティテスト（CI 必須）: 要求の編集・一覧・適合表。GUI は編集の欄に打った文字列を `fune requirement add|set` と同じ形
//! （`CommandText::RequirementAdd`・`RequirementSet`）で `POST /ops/from_command` に送り、`POST /ops` で保存する。
//! 読み方は CLI と同じ service の関数なので、書かれるファイルはバイト単位で一致する。一覧と適合表は
//! `GET /requirements`・`GET /requirements/compliance` と `fune --json requirement ls|compliance` が同じ。

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
fn requirement_edits_match_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let edits: Vec<(Vec<&str>, Value, &str)> = vec![
        (
            vec![
                "requirement",
                "add",
                "R_TEST",
                "--statement",
                "The battery shall stay between 0 and 40 degC.",
                "--statement-en",
                "Battery range",
                "--label",
                "Battery range",
                "--quantity",
                "Temperature",
                "--source",
                "sys_judgement",
                "--kind",
                "range",
                "--min",
                "-5 degC",
                "--max",
                "40 degC",
                "--cases",
                "hot_imaging,cold_safe",
                "--analysis",
                "thermal.steady",
                "--check",
                "heater_duty:BAT",
            ],
            json!({ "command": "requirement_add", "id": "R_TEST",
                "statement": "The battery shall stay between 0 and 40 degC.",
                "statement_en": "Battery range", "label": "Battery range",
                "quantity": "Temperature", "source": "sys_judgement", "kind": "range",
                "min": "-5 degC", "max": "40 degC", "cases": "hot_imaging,cold_safe",
                "analysis": "thermal.steady", "check": "heater_duty:BAT",
                "tolerance": "", "budget": null }),
            "create_requirement",
        ),
        (
            vec![
                "requirement",
                "set",
                "R_TEST",
                "--statement",
                "The mass shall not exceed 11 kg.",
                "--quantity",
                "Mass",
                "--source",
                "lv_icd",
                "--kind",
                "at_most",
                "--value",
                "11 kg",
                "--all-cases",
                "true",
                "--budget",
                "mass",
            ],
            json!({ "command": "requirement_set", "id": "R_TEST",
                "statement": "The mass shall not exceed 11 kg.", "quantity": "Mass",
                "source": "lv_icd", "kind": "at_most", "value": "11 kg",
                "all_cases": "true", "budget": "mass" }),
            "set_requirement",
        ),
        // 解析の入力を与える要求（ミッション期間）。`--input` は `RequirementText.input`
        (
            vec![
                "requirement",
                "set",
                "R_MISSION",
                "--statement",
                "The mission shall last at least 2 years.",
                "--quantity",
                "Time",
                "--source",
                "sys_judgement",
                "--kind",
                "at_least",
                "--value",
                "2 yr",
                "--input",
                "mission_duration",
            ],
            json!({ "command": "requirement_set", "id": "R_MISSION",
                "statement": "The mission shall last at least 2 years.", "quantity": "Time",
                "source": "sys_judgement", "kind": "at_least", "value": "2 yr",
                "input": "mission_duration" }),
            "set_requirement",
        ),
    ];
    for (cli, body, op) in edits {
        run_cli(&cli_root, &cli);
        let ops = http_command(&rt, &http_root, body);
        assert!(ops.iter().all(|o| o["op"] == op), "{ops:?}");
        assert_same_files(&cli_root, &http_root, &cli.join(" "));
    }
    run_cli(&cli_root, &["requirement", "rm", "R_TEST"]);
    post_json(
        &rt,
        &http_root,
        "/api/v1/ops",
        json!({ "ops": [{ "op": "remove_requirement", "id": "R_TEST" }] }),
    );
    assert_same_files(&cli_root, &http_root, "requirement rm R_TEST");
    assert!(!http_root.join("requirements/R_TEST.toml").exists());
}

fn cli_json(root: &Path, args: &[&str]) -> Value {
    let out = fune()
        .arg("-C")
        .arg(root)
        .arg("--json")
        .args(args)
        .output()
        .unwrap();
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!("{e}: {}", String::from_utf8_lossy(&out.stderr));
    })
}

/// 一覧（制約の文・編集の欄の初期値・欄のパス）は CLI の JSON と HTTP で同じ。
#[test]
fn requirement_list_matches_http() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t, root) = demo_copy();
    let http = get_json(&rt, &root, "/api/v1/requirements");
    assert_eq!(cli_json(&root, &["requirement", "ls"]), http);
    let mass = http
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "R_MASS")
        .unwrap();
    assert_eq!(mass["constraint"]["text"], "≤ 12.0 kg");
    assert_eq!(mass["text"]["budget"], "mass");
    assert_eq!(
        mass["field_paths"]["value"],
        "requirements.R_MASS.constraint.value"
    );
    // GUI の一括取得の `requirements` も同じ
    let design = get_json(&rt, &root, "/api/v1/design");
    assert_eq!(design["requirements"], http);
}

/// 適合表は CLI の JSON と HTTP で同じ。違反があれば終了コードは 1（`--deny-unverified` なら確かめられないものも）。
#[test]
fn compliance_matches_http_and_sets_the_exit_code() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t, root) = demo_copy();
    let http = get_json(&rt, &root, "/api/v1/requirements/compliance");
    assert_eq!(cli_json(&root, &["requirement", "compliance"]), http);
    let out = fune()
        .arg("-C")
        .arg(&root)
        .args(["requirement", "compliance"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    // 違反の要求を外すと 0、確かめられない要求が残るので --deny-unverified なら 1
    for id in ["R_BAT_TEMP", "R_STORAGE"] {
        run_cli(&root, &["requirement", "rm", id]);
    }
    let ok = fune()
        .arg("-C")
        .arg(&root)
        .args(["requirement", "compliance"])
        .output()
        .unwrap();
    assert_eq!(ok.status.code(), Some(0));
    let strict = fune()
        .arg("-C")
        .arg(&root)
        .args(["requirement", "compliance", "--deny-unverified"])
        .output()
        .unwrap();
    assert_eq!(strict.status.code(), Some(1));
}
