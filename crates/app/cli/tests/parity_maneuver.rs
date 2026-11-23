//! パリティテスト（CI 必須）: マヌーバの編集と一覧。GUI は編集の欄に打った文字列を `fune maneuver add|set` と同じ形
//! （`CommandText::ManeuverAdd`・`ManeuverSet`）で `POST /ops/from_command` に送り、`POST /ops` で保存する。
//! 読み方は CLI と同じ service の関数なので、書かれるファイルはバイト単位で一致する。一覧は `GET /maneuvers` と
//! `fune --json maneuver ls` が同じ。

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
fn maneuver_edits_match_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let edits: Vec<(Vec<&str>, Value, &str)> = vec![
        (
            vec![
                "maneuver",
                "add",
                "MV_UP",
                "--category",
                "insertion",
                "--method",
                "low_thrust",
                "--label",
                "Ascent (test)",
                "--thruster",
                "RW_X",
                "--altitude",
                "620 km",
                "--inclination",
                "97.7 deg",
                "--sunlit-only",
                "true",
            ],
            json!({ "command": "maneuver_add", "id": "MV_UP", "category": "insertion",
                "method": "low_thrust", "label": "Ascent (test)", "thruster": "RW_X",
                "altitude": "620 km", "inclination": "97.7 deg", "sunlit_only": "true",
                "after": "", "mode": null }),
            "create_maneuver",
        ),
        (
            vec![
                "maneuver",
                "add",
                "MV_DOWN",
                "--category",
                "deorbit",
                "--method",
                "impulsive",
                "--after",
                "MV_UP",
                "--perigee-altitude",
                "-50 km",
            ],
            json!({ "command": "maneuver_add", "id": "MV_DOWN", "category": "deorbit",
                "method": "impulsive", "after": "MV_UP", "perigee_altitude": "-50 km" }),
            "create_maneuver",
        ),
        (
            vec![
                "maneuver",
                "set",
                "MV_UP",
                "--category",
                "insertion",
                "--method",
                "finite_burn",
                "--thruster",
                "RW_X",
                "--altitude",
                "650 km",
            ],
            json!({ "command": "maneuver_set", "id": "MV_UP", "category": "insertion",
                "method": "finite_burn", "thruster": "RW_X", "altitude": "650 km" }),
            "set_maneuver",
        ),
    ];
    for (cli, body, op) in edits {
        run_cli(&cli_root, &cli);
        let ops = http_command(&rt, &http_root, body);
        assert!(ops.iter().all(|o| o["op"] == op), "{ops:?}");
        assert_same_files(&cli_root, &http_root, &cli.join(" "));
    }
    run_cli(&cli_root, &["maneuver", "rm", "MV_UP"]);
    post_json(
        &rt,
        &http_root,
        "/api/v1/ops",
        json!({ "ops": [{ "op": "remove_maneuver", "id": "MV_UP" }] }),
    );
    assert_same_files(&cli_root, &http_root, "maneuver rm MV_UP");
    let down = std::fs::read_to_string(http_root.join("maneuvers/MV_DOWN.toml")).unwrap();
    assert!(!down.contains("after"), "{down}");
}

/// 一覧（実行の順・目標の要素・編集の欄の初期値）は CLI の JSON と HTTP で同じ。
#[test]
fn maneuver_list_matches_http() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t, root) = demo_copy();
    run_cli(
        &root,
        &[
            "maneuver",
            "add",
            "MV_B",
            "--category",
            "other",
            "--method",
            "impulsive",
            "--after",
            "MV_A",
            "--altitude",
            "500 km",
        ],
    );
    run_cli(
        &root,
        &[
            "maneuver",
            "add",
            "MV_A",
            "--category",
            "insertion",
            "--method",
            "impulsive",
            "--altitude",
            "700 km",
        ],
    );
    let http = get_json(&rt, &root, "/api/v1/maneuvers");
    let out = fune()
        .arg("-C")
        .arg(&root)
        .args(["--json", "maneuver", "ls"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let cli: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(cli, http);
    let a = http
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == "MV_A")
        .unwrap();
    let b = http
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == "MV_B")
        .unwrap();
    assert!(a["order"].as_u64() < b["order"].as_u64(), "{http}");
    assert_eq!(b["text"]["altitude"], "500 km");
    assert_eq!(
        b["field_paths"]["altitude"],
        "maneuvers.MV_B.target.altitude"
    );
}

/// 日付つきの遷移（`--depart`・`--arrive`）も CLI と HTTP で同じファイルになり、両方を空にして書き直すと `[transfer]` が消える。
#[test]
fn dated_transfer_edits_match_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let cli = [
        "maneuver",
        "add",
        "MV_MARS",
        "--category",
        "insertion",
        "--method",
        "impulsive",
        "--body",
        "mars",
        "--perigee-altitude",
        "300 km",
        "--apogee-altitude",
        "33000 km",
        "--depart",
        "2028-12-10T00:00:00Z",
        "--arrive",
        "2029-09-25T00:00:00Z",
    ];
    run_cli(&cli_root, &cli);
    let body = json!({ "command": "maneuver_add", "id": "MV_MARS", "category": "insertion",
        "method": "impulsive", "body": "mars", "perigee_altitude": "300 km",
        "apogee_altitude": "33000 km", "depart": "2028-12-10T00:00:00Z",
        "arrive": "2029-09-25T00:00:00Z" });
    http_command(&rt, &http_root, body);
    assert_same_files(&cli_root, &http_root, &cli.join(" "));
    let text = std::fs::read_to_string(http_root.join("maneuvers/MV_MARS.toml")).unwrap();
    assert!(
        text.contains("[transfer]\ndeparture = \"2028-12-10T00:00:00Z\"\narrival   = \"2029-09-25T00:00:00Z\"\n"),
        "{text}"
    );
    // 日付を 1 つだけ書くのは誤り（どちらも同じ service の関数）
    let fail = fune()
        .arg("-C")
        .arg(&cli_root)
        .args([
            "maneuver",
            "set",
            "MV_MARS",
            "--category",
            "insertion",
            "--method",
            "impulsive",
        ])
        .args([
            "--body",
            "mars",
            "--perigee-altitude",
            "300 km",
            "--depart",
            "2028-12-10T00:00:00Z",
        ])
        .output()
        .unwrap();
    assert!(!fail.status.success());
    // 日付を書かずに書き直すと `[transfer]` が消える
    let reset = [
        "maneuver",
        "set",
        "MV_MARS",
        "--category",
        "insertion",
        "--method",
        "impulsive",
        "--body",
        "mars",
        "--perigee-altitude",
        "300 km",
    ];
    run_cli(&cli_root, &reset);
    http_command(
        &rt,
        &http_root,
        json!({ "command": "maneuver_set", "id": "MV_MARS", "category": "insertion",
            "method": "impulsive", "body": "mars", "perigee_altitude": "300 km",
            "depart": "", "arrive": null }),
    );
    assert_same_files(&cli_root, &http_root, "maneuver set (no dates)");
    let text = std::fs::read_to_string(http_root.join("maneuvers/MV_MARS.toml")).unwrap();
    assert!(!text.contains("transfer"), "{text}");
    // 一覧の編集の欄の初期値は `--depart`・`--arrive` と同じ文字列
    run_cli(&cli_root, &cli_set_with_dates());
    let http = get_json(&rt, &cli_root, "/api/v1/maneuvers");
    let row = http
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == "MV_MARS")
        .unwrap();
    assert_eq!(row["text"]["depart"], "2028-12-10T00:00:00Z");
    assert_eq!(
        row["field_paths"]["arrive"],
        "maneuvers.MV_MARS.transfer.arrival"
    );
}

fn cli_set_with_dates() -> Vec<&'static str> {
    vec![
        "maneuver",
        "set",
        "MV_MARS",
        "--category",
        "insertion",
        "--method",
        "impulsive",
        "--body",
        "mars",
        "--perigee-altitude",
        "300 km",
        "--depart",
        "2028-12-10T00:00:00Z",
        "--arrive",
        "2029-09-25T00:00:00Z",
    ]
}
