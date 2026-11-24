//! パリティテスト（CI 必須）: 形態の編集と読み取り。GUI は編集の欄に打った文字列を `fune configuration add|set` と
//! `fune part place --configuration` と同じ形（`CommandText::ConfigurationAdd`・`ConfigurationSet`・`PartPlace`）で
//! `POST /ops/from_command` に送り、`POST /ops` で保存する。機体座標の配置は `POST /parts/{id}/place_body`（`configuration` つき）。
//! 読み方は CLI と同じ service の関数なので、書かれるファイルはバイト単位で一致する。
//! 読み取り（`configuration ls`・`part ls|show --configuration`）は `GET /configurations`・`/parts?configuration=`・`/parts/{id}?configuration=` と同じ。

mod common;

use common::{assert_same_files, demo_copy, fune, get_json, post_json, run_cli};
use serde_json::{Value, json};
use std::path::Path;

fn http_command(rt: &tokio::runtime::Runtime, root: &Path, body: Value) -> Vec<Value> {
    let ops = post_json(rt, root, "/api/v1/ops/from_command", body);
    post_json(rt, root, "/api/v1/ops", json!({ "ops": ops.clone() }));
    ops.as_array().cloned().unwrap_or_default()
}

fn cli_json(root: &Path, args: &[&str]) -> Value {
    let out = fune()
        .arg("-C")
        .arg(root)
        .arg("--json")
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn configuration_edits_match_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    // (CLI の引数, HTTP の /ops/from_command の本文, 期待する Op)
    let edits: Vec<(Vec<&str>, Value, &str)> = vec![
        (
            vec![
                "configuration",
                "add",
                "STOWED",
                "--label",
                "Stowed",
                "--description",
                "Paddle folded onto the body",
            ],
            json!({ "command": "configuration_add", "id": "STOWED", "label": "Stowed",
                "description": "Paddle folded onto the body" }),
            "create_configuration",
        ),
        // 取付先を書かなくてよい。位置は取付点に対する相対、姿勢は角度（四元数にして書く）
        (
            vec![
                "part",
                "place",
                "SAP_PY",
                "--configuration",
                "STOWED",
                "--position",
                "0 5 1.5 mm",
                "--rotation",
                "0 0 -90 deg",
            ],
            json!({ "command": "part_place", "part": "SAP_PY", "configuration": "STOWED",
                "position": "0 5 1.5 mm", "rotation": "0 0 -90 deg" }),
            "place",
        ),
        // 書くなら部品の取付と同じ取付先・取付点
        (
            vec![
                "part",
                "place",
                "SAP_PY",
                "--configuration",
                "STOWED",
                "--on",
                "PANEL_PY",
                "--point",
                "HINGE",
                "--position",
                "0 6 1.5 mm",
            ],
            json!({ "command": "part_place", "part": "SAP_PY", "configuration": "STOWED",
                "on": "PANEL_PY", "point": "HINGE", "position": "0 6 1.5 mm" }),
            "place",
        ),
        (
            vec![
                "configuration",
                "set",
                "STOWED",
                "--label",
                "Stowed (folded)",
            ],
            json!({ "command": "configuration_set", "id": "STOWED", "label": "Stowed (folded)" }),
            "set_configuration",
        ),
    ];
    for (cli, body, op) in edits {
        run_cli(&cli_root, &cli);
        let ops = http_command(&rt, &http_root, body);
        assert!(ops.iter().all(|o| o["op"] == op), "{ops:?}");
        assert_same_files(&cli_root, &http_root, &cli.join(" "));
    }
    let text = std::fs::read_to_string(http_root.join("configurations/STOWED.toml")).unwrap();
    assert!(
        text.contains("Stowed (folded)") && text.contains("[pose.SAP_PY]"),
        "{text}"
    );
    // 機体座標での配置: CLI は `--body-pos`、HTTP は `/parts/{id}/place_body`（どちらも形態の中の機体座標を読み、形態の `[pose]` に書く）
    run_cli(
        &cli_root,
        &[
            "part",
            "place",
            "SAP_PY",
            "--configuration",
            "STOWED",
            "--body-pos",
            "30 60 40 mm",
        ],
    );
    post_json(
        &rt,
        &http_root,
        "/api/v1/parts/SAP_PY/place_body",
        json!({ "position": { "x": 30.0, "y": 60.0, "z": 40.0, "unit": "mm" },
            "configuration": "STOWED" }),
    );
    assert_same_files(
        &cli_root,
        &http_root,
        "part place --configuration --body-pos",
    );
    // 部品自身のファイルは、形態の中の配置では変わらない
    let base = demo_copy();
    assert_eq!(
        std::fs::read(base.1.join("parts/power/SAP_PY.toml")).unwrap(),
        std::fs::read(http_root.join("parts/power/SAP_PY.toml")).unwrap()
    );
    // 上書きを消す・名前の変更・削除は Op をそのまま送る
    run_cli(
        &cli_root,
        &["configuration", "unset-pose", "STOWED", "SAP_PY"],
    );
    post_json(
        &rt,
        &http_root,
        "/api/v1/ops",
        json!({ "ops": [{ "op": "unset_configuration_pose", "configuration": "STOWED",
            "part": "SAP_PY" }] }),
    );
    assert_same_files(&cli_root, &http_root, "configuration unset-pose");
    let text = std::fs::read_to_string(http_root.join("configurations/STOWED.toml")).unwrap();
    assert!(!text.contains("pose"), "{text}");
    run_cli(&cli_root, &["configuration", "rename", "STOWED", "FOLDED"]);
    post_json(
        &rt,
        &http_root,
        "/api/v1/ops",
        json!({ "ops": [{ "op": "rename_configuration", "id": "STOWED", "to": "FOLDED" }] }),
    );
    assert_same_files(&cli_root, &http_root, "configuration rename");
    assert!(http_root.join("configurations/FOLDED.toml").exists());
    run_cli(&cli_root, &["configuration", "rm", "FOLDED"]);
    post_json(
        &rt,
        &http_root,
        "/api/v1/ops",
        json!({ "ops": [{ "op": "remove_configuration", "id": "FOLDED" }] }),
    );
    assert_same_files(&cli_root, &http_root, "configuration rm");
    assert!(!http_root.join("configurations/FOLDED.toml").exists());
}

/// 一覧・部品の表示（形態の中の取付と機体座標）は CLI の JSON と HTTP で同じ。基準と形態で姿勢が違い、上書きの印が付く。
#[test]
fn configuration_reads_match_http() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t, root) = demo_copy();
    run_cli(
        &root,
        &["configuration", "add", "STOWED", "--label", "Stowed"],
    );
    run_cli(
        &root,
        &[
            "part",
            "place",
            "SAP_PY",
            "--configuration",
            "STOWED",
            "--position",
            "0 5 1.5 mm",
        ],
    );
    let http = get_json(&rt, &root, "/api/v1/configurations");
    assert_eq!(cli_json(&root, &["configuration", "ls"]), http);
    assert_eq!(http[0]["id"], "STOWED");
    assert_eq!(http[0]["overridden"], json!(["SAP_PY"]));
    assert_eq!(http[0]["text"]["label"], "Stowed");
    assert_eq!(
        cli_json(&root, &["configuration", "show", "STOWED"]),
        http[0]
    );
    // `GET /design` にも載る
    let design = get_json(&rt, &root, "/api/v1/design");
    assert_eq!(design["configurations"], http);

    let parts = get_json(&rt, &root, "/api/v1/parts?configuration=STOWED");
    assert_eq!(
        cli_json(&root, &["part", "ls", "--configuration", "STOWED"]),
        parts
    );
    let row = |rows: &Value| -> Value {
        rows.as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == "SAP_PY")
            .cloned()
            .unwrap()
    };
    let stowed = row(&parts);
    let base = row(&get_json(&rt, &root, "/api/v1/parts"));
    assert_eq!(stowed["mount"]["overridden"], true);
    assert_eq!(base["mount"]["overridden"], false);
    assert_ne!(stowed["mount"]["position"], base["mount"]["position"]);
    assert_ne!(stowed["body"]["position"], base["body"]["position"]);
    // 取付先・取付点・部品の他の欄は同じ
    assert_eq!(stowed["mount"]["on"], base["mount"]["on"]);
    assert_eq!(stowed["file"], base["file"]);
    let one = get_json(&rt, &root, "/api/v1/parts/SAP_PY?configuration=STOWED");
    assert_eq!(
        cli_json(
            &root,
            &["part", "show", "SAP_PY", "--configuration", "STOWED"]
        ),
        one
    );
    assert_eq!(one["part"], stowed);
    // 形態を指さない読み取りは今まで通り基準
    assert_eq!(get_json(&rt, &root, "/api/v1/parts/SAP_PY")["part"], base);
}
