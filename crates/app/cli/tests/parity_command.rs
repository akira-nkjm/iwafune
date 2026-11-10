//! パリティテスト（CI 必須）: 名詞のコマンドに打った引数の編集。GUI は欄に打った文字列を `POST /ops/from_command`
//! で Op にしてから `POST /ops` で送る。読み方（量・ベクトル・クォータニオン・時間割合・一覧・`名前=種類`・表示名の既定）を
//! 決めるのは CLI と同じ service の関数なので、書かれるファイルもバイト単位で一致する。

mod common;

use common::{assert_same_files, demo_copy, get_json, post_json, run_cli};
use serde_json::{Value, json};
use std::path::Path;

/// 打った引数 → Op の列（HTTP）→ 適用（HTTP）。Op の列を返す。
fn http_command(rt: &tokio::runtime::Runtime, root: &Path, body: Value) -> Vec<Value> {
    let ops = post_json(rt, root, "/api/v1/ops/from_command", body);
    post_json(rt, root, "/api/v1/ops", json!({ "ops": ops.clone() }));
    ops.as_array().cloned().unwrap_or_default()
}

/// 系統図のネットの欄（レール電圧など）と `fune net set|unset`。欄の名前は API の `NetView.field_paths`（診断のパスと同じ）。
#[test]
fn gui_net_field_edits_match_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let design = get_json(&rt, &http_root, "/api/v1/design");
    let p12v = design["nets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == "P12V")
        .unwrap();
    assert_eq!(p12v["field_paths"]["rail"], "nets.P12V.rail", "{p12v}");
    let edits: Vec<(Vec<&str>, Value)> = vec![
        // GUI のレールの欄は rail だけを送る
        (
            vec!["net", "set", "P12V", "--rail", "12.2 V"],
            json!({ "command": "net_set", "net": "P12V", "rail": "12.2 V" }),
        ),
        // いくつかの旗（宣言の順に 1 フィールド 1 つの Op）
        (
            vec!["net", "set", "P5V", "--limit", "1.5 A", "--label", "5 V 系"],
            json!({ "command": "net_set", "net": "P5V", "label": "5 V 系", "limit": "1.5 A" }),
        ),
        (
            vec!["net", "unset", "P12V", "limit"],
            json!({ "command": "net_unset", "net": "P12V", "field": "limit" }),
        ),
    ];
    for (cli, body) in edits {
        run_cli(&cli_root, &cli);
        let ops = http_command(&rt, &http_root, body);
        assert!(ops.iter().all(|op| op["op"] == "set_net_field"), "{ops:?}");
        assert_same_files(&cli_root, &http_root, &cli.join(" "));
    }
    let p12v = std::fs::read_to_string(http_root.join("nets/P12V.toml")).unwrap();
    assert!(p12v.contains("12.2") && !p12v.contains("limit"), "{p12v}");
}

/// 部品の配置の欄（取付先・取付点・位置・姿勢）と `fune part place`。位置・姿勢は打った文字列のまま送る。
/// 取付先が空なら機体の根（`--root`）で、取付点の欄は書かない。
#[test]
fn gui_placement_matches_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let edits: Vec<(Vec<&str>, Value)> = vec![
        (
            vec![
                "part",
                "place",
                "CAM",
                "--on",
                "PANEL_PZ",
                "--point",
                "CAM_PORT",
                "--position",
                "0 -55 -90 mm",
                "--orientation",
                "1 0 0 0",
            ],
            json!({ "command": "part_place", "part": "CAM", "on": "PANEL_PZ", "point": "CAM_PORT",
                "position": "0 -55 -90 mm", "orientation": "1 0 0 0" }),
        ),
        (
            vec!["part", "place", "RW_X", "--root", "--position", "10 0 0 mm"],
            json!({ "command": "part_place", "part": "RW_X", "on": "", "point": "RW_SLOT_X",
                "position": "10 0 0 mm", "orientation": " " }),
        ),
    ];
    for (cli, body) in edits {
        run_cli(&cli_root, &cli);
        let ops = http_command(&rt, &http_root, body);
        assert_eq!(ops.len(), 1, "{ops:?}");
        assert_eq!(ops[0]["op"], "place", "{ops:?}");
        assert_same_files(&cli_root, &http_root, &cli.join(" "));
    }
    let cam = std::fs::read_to_string(http_root.join("parts/PAYLOAD/CAM.toml")).unwrap();
    assert!(
        cam.contains("-55.0") && cam.contains("unit = \"mm\""),
        "{cam}"
    );
}

/// モード表の編集（基本の状態・時間割合・未指定に戻す）と `fune mode set|unset`。GUI は変えたセルをまとめて送り、
/// 割合は欄に打った文字列のまま（空の欄は書かない）。1 つの `SetModeStates` になる。
#[test]
fn gui_mode_cells_match_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    for cli in [
        vec![
            "mode", "set", "NOMINAL", "UHF_TRX", "rx", "--duty", "tx=0.15",
        ],
        vec![
            "mode",
            "set",
            "NOMINAL",
            "HTR_BAT",
            "off",
            "--duty",
            "on = 0.35",
        ],
        vec!["mode", "set", "SAFE", "RW_X", "idle"],
        vec!["mode", "unset", "DOWNLINK", "CAM"],
    ] {
        run_cli(&cli_root, &cli);
    }
    let ops = http_command(
        &rt,
        &http_root,
        json!({ "command": "mode_set", "cells": [
            { "mode": "NOMINAL", "part": "UHF_TRX", "state": "rx", "duty": { "tx": "0.15" } },
            { "mode": "NOMINAL", "part": "HTR_BAT", "state": "off", "duty": { "on": " 0.35", "idle": "" } },
            { "mode": "SAFE", "part": "RW_X", "state": "idle", "duty": {} },
            { "mode": "DOWNLINK", "part": "CAM", "state": null, "duty": {} },
        ] }),
    );
    assert_eq!(ops.len(), 1, "{ops:?}");
    assert_eq!(ops[0]["cells"][1]["duty"], json!({ "on": 0.35 }), "{ops:?}");
    assert_same_files(&cli_root, &http_root, "mode set|unset");
    let downlink = std::fs::read_to_string(http_root.join("modes/DOWNLINK.toml")).unwrap();
    assert!(
        !downlink.lines().any(|l| l.starts_with("CAM ")),
        "{downlink}"
    );
}

/// 定義のページの属性・種別の作成と `fune attr add`・`fune kind add`。表示名を空にすると ID（既定は service）。
/// 種別の状態・属性の一覧はカンマ区切りの打った文字列のまま、ポートの雛形は `名前=種類` で送る。
#[test]
fn gui_definitions_match_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let edits: Vec<(Vec<&str>, Value)> = vec![
        (
            vec![
                "attr",
                "add",
                "tank_pressure",
                "--label",
                "タンク圧",
                "--dimension",
                "Pressure",
                "--units",
                "Pa,kPa",
                "--by-state",
            ],
            json!({ "command": "attr_add", "id": "tank_pressure", "label": "タンク圧",
                "dimension": "Pressure", "units": ["Pa", "kPa"], "by_state": true }),
        ),
        (
            vec!["attr", "add", "depth", "--dimension", "Length"],
            json!({ "command": "attr_add", "id": "depth", "label": "", "dimension": "Length",
                "units": [], "by_state": false }),
        ),
        (
            vec![
                "kind",
                "add",
                "propellant_tank",
                "--role",
                "propellant_tank",
                "--states",
                "off, on",
                "--required",
                "mass, tank_pressure",
                "--port",
                "PWR=power",
            ],
            json!({ "command": "kind_add", "id": "propellant_tank", "role": "propellant_tank",
                "states": "off, on", "required": "mass, tank_pressure", "ports": ["PWR=power"] }),
        ),
    ];
    for (cli, body) in edits {
        run_cli(&cli_root, &cli);
        let ops = http_command(&rt, &http_root, body);
        assert_eq!(ops.len(), 1, "{ops:?}");
        assert_same_files(&cli_root, &http_root, &cli.join(" "));
    }
    let depth = std::fs::read_to_string(http_root.join("defs/attribute/depth.toml")).unwrap();
    assert!(
        depth
            .lines()
            .any(|l| l.starts_with("label") && l.ends_with("\"depth\"")),
        "{depth}"
    );
}
