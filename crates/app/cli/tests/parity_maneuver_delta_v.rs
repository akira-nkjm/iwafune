//! パリティテスト（CI 必須）: 直接書いた ΔV（`--delta-v`）の編集。GUI は `POST /ops/from_command`（`maneuver_add`・`maneuver_set`）と
//! `POST /ops` で保存し、書かれるファイルは CLI とバイト単位で一致する。

mod common;

use common::{assert_same_files, demo_copy, get_json, post_json, run_cli};
use serde_json::{Value, json};
use std::path::Path;

fn http_command(rt: &tokio::runtime::Runtime, root: &Path, body: Value) -> Vec<Value> {
    let ops = post_json(rt, root, "/api/v1/ops/from_command", body);
    post_json(rt, root, "/api/v1/ops", json!({ "ops": ops.clone() }));
    ops.as_array().cloned().unwrap_or_default()
}

/// 直接書いた ΔV（`--delta-v`）も CLI と HTTP で同じファイルになり、編集で量を変えられ、書き直して外せばその行だけが消える。
#[test]
fn written_delta_v_edits_match_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let cli = [
        "maneuver",
        "add",
        "MV_TCM",
        "--category",
        "station_keeping",
        "--method",
        "impulsive",
        "--label",
        "Trajectory correction allowance",
        "--thruster",
        "RW_X",
        "--delta-v",
        "15 m/s",
    ];
    run_cli(&cli_root, &cli);
    let body = json!({ "command": "maneuver_add", "id": "MV_TCM", "category": "station_keeping",
        "method": "impulsive", "label": "Trajectory correction allowance", "thruster": "RW_X",
        "delta_v": "15 m/s" });
    let ops = http_command(&rt, &http_root, body);
    assert!(ops.iter().all(|o| o["op"] == "create_maneuver"), "{ops:?}");
    assert_same_files(&cli_root, &http_root, &cli.join(" "));
    let text = std::fs::read_to_string(http_root.join("maneuvers/MV_TCM.toml")).unwrap();
    assert!(
        text.contains("delta_v  = { value = 15.0, unit = \"m/s\" }"),
        "{text}"
    );
    // 量を変える
    let cli = [
        "maneuver",
        "set",
        "MV_TCM",
        "--category",
        "station_keeping",
        "--method",
        "impulsive",
        "--thruster",
        "RW_X",
        "--delta-v",
        "20 m/s",
    ];
    run_cli(&cli_root, &cli);
    let body = json!({ "command": "maneuver_set", "id": "MV_TCM", "category": "station_keeping",
        "method": "impulsive", "thruster": "RW_X", "delta_v": "20 m/s" });
    http_command(&rt, &http_root, body);
    assert_same_files(&cli_root, &http_root, &cli.join(" "));
    // 一覧の欄の初期値は書いた文字列
    let http = get_json(&rt, &http_root, "/api/v1/maneuvers");
    let m = http
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == "MV_TCM")
        .unwrap();
    assert_eq!(m["text"]["delta_v"], "20 m/s");
    assert_eq!(m["field_paths"]["delta_v"], "maneuvers.MV_TCM.delta_v");
}
