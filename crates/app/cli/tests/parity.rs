//! パリティテスト（CI 必須）: 同じ編集を CLI（`fune` の実行ファイル）と HTTP（`POST /api/v1/ops`）で行い、
//! 結果のファイルがバイト単位で一致することを確かめる（CONTRIBUTING.md の「CLI と GUI の対等性」）。
//! 人が打った文字列の編集（`POST /ops/from_text`・`/ops/unset_from_text`）は `parity_text.rs`、
//! 名詞のコマンドに打った引数（`POST /ops/from_command`）は `parity_command.rs`。

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{assert_same_files, demo_copy, post_json, run_cli, snapshot_files};
use fune_service::Workspace;
use serde_json::{Value, json};
use std::path::Path;
use std::sync::Arc;
use tower::ServiceExt;

/// 1 つの編集: CLI の引数と、GUI が送る Op の JSON。
struct Edit {
    name: &'static str,
    cli: &'static [&'static str],
    ops: Value,
}
fn edits() -> Vec<Edit> {
    let mut all = part_edits();
    all.extend(vec![
        Edit { name: "SetViewEdge", cli: &["view", "edge-set", "electrical", "P12V:PCDU.OUT1->RW_X.PWR", "--points", "-20,0 20,0 20,80", "--source-side", "right"], ops: json!([{ "op": "set_view_edge", "view": "electrical", "key": "P12V:PCDU.OUT1->RW_X.PWR", "edge": { "points": [[-20,0],[20,0],[20,80]], "source_side": "right" } }]) },
        Edit { name: "UnsetViewEdge", cli: &["view", "edge-reset", "electrical", "P12V:PCDU.OUT1->RW_X.PWR"], ops: json!([{ "op": "unset_view_edge", "view": "electrical", "key": "P12V:PCDU.OUT1->RW_X.PWR" }]) },
        Edit { name: "ResetView", cli: &["view", "reset", "electrical"], ops: json!([{ "op": "reset_view", "view": "electrical" }]) },
    ]);
    all.extend(record_edits());
    all.extend(def_edits());
    all.extend(case_edits());
    all.extend(schema_edits());
    all
}
fn part_edits() -> Vec<Edit> {
    vec![
        Edit {
            name: "SetValue",
            cli: &[
                "set",
                "RW_X.mass",
                "650 g",
                "--source",
                "em_test_2026_10",
                "--at",
                "table 4",
                "--note",
                "EM measurement",
            ],
            ops: json!([{ "op": "set_value", "part": "RW_X", "attr": "mass",
                "value": { "type": "scalar", "data": { "value": 650.0, "unit": "g" } },
                "from": { "source": "em_test_2026_10", "at": "table 4", "note": "EM measurement" } }]),
        },
        Edit {
            name: "CreatePart",
            cli: &[
                "part",
                "add",
                "RW_W",
                "--kind",
                "reaction_wheel",
                "--group",
                "aocs",
            ],
            ops: json!([{ "op": "create_part", "id": "RW_W", "group": "aocs", "kind": "reaction_wheel" }]),
        },
        Edit {
            name: "SetModeStates",
            cli: &["mode", "set", "NOMINAL", "RW_W", "on", "--duty", "peak=0.1"],
            ops: json!([{ "op": "set_mode_states", "cells": [
                { "mode": "NOMINAL", "part": "RW_W", "state": "on", "duty": { "peak": 0.1 } }] }]),
        },
        Edit {
            name: "Connect",
            cli: &["net", "connect", "RW_W.PWR", "P12V"],
            ops: json!([{ "op": "set_port", "part": "RW_W", "port": "PWR", "net": "P12V" }]),
        },
        Edit {
            name: "SetViewLayout",
            cli: &[
                "view",
                "set",
                "electrical",
                "RW_W",
                "--pos",
                "420,120",
                "--size",
                "160,60",
            ],
            ops: json!([{ "op": "set_view_layout", "view": "electrical",
                "nodes": [{ "id": "RW_W", "x": 420, "y": 120, "w": 160, "h": 60 }] }]),
        },
    ]
}

fn record_edits() -> Vec<Edit> {
    vec![
        Edit {
            name: "SetMountPoint",
            cli: &[
                "part",
                "point",
                "PANEL_MZ",
                "SLOT_9",
                "--position",
                "30 0 5 mm",
            ],
            ops: json!([{ "op": "set_mount_point", "part": "PANEL_MZ", "point": "SLOT_9",
                "pose": { "position": { "x": 30.0, "y": 0.0, "z": 5.0, "unit": "mm" } } }]),
        },
        Edit {
            name: "CreateCatalogItem",
            cli: &[
                "catalog",
                "add",
                "local/rw_30mNms",
                "--kind",
                "reaction_wheel",
                "--label",
                "30 mNms RW",
            ],
            ops: json!([{ "op": "create_catalog_item", "id": "local/rw_30mNms", "kind": "reaction_wheel",
                "label": "30 mNms RW" }]),
        },
        Edit {
            name: "SetCatalogValue",
            cli: &["catalog", "set", "local/rw_30mNms", "mass", "0.9 kg"],
            ops: json!([{ "op": "set_catalog_value", "item": "local/rw_30mNms", "attr": "mass",
                "value": { "type": "scalar", "data": { "value": 0.9, "unit": "kg" } } }]),
        },
        Edit {
            name: "SetSetting",
            cli: &["setting", "set", "analysis.power.initial_soc", "0.8"],
            ops: json!([{ "op": "set_setting", "path": "analysis.power.initial_soc",
                "value": { "type": "number", "value": 0.8 } }]),
        },
    ]
}

fn def_edits() -> Vec<Edit> {
    vec![
        Edit {
            name: "CreateNet",
            cli: &[
                "net",
                "add",
                "P5V_X",
                "--kind",
                "power",
                "--source",
                "PCDU.OUT1",
                "--rail",
                "5 V",
                "--limit",
                "2 A",
            ],
            ops: json!([{ "op": "create_net", "net": { "id": "P5V_X", "kind": "power", "source": "PCDU.OUT1",
                "rail": { "value": 5.0, "unit": "V" }, "limit": { "value": 2.0, "unit": "A" } } }]),
        },
        Edit {
            name: "UpdateNet",
            cli: &[
                "net", "replace", "P5V_X", "--kind", "power", "--label", "5 V rail", "--rail",
                "5 V",
            ],
            ops: json!([{ "op": "update_net", "net": { "id": "P5V_X", "kind": "power", "label": "5 V rail",
                "rail": { "value": 5.0, "unit": "V" } } }]),
        },
        Edit {
            name: "SetNetField",
            cli: &[
                "net", "set", "P5V_X", "--rail", "5.1 V", "--master", "OBC.CAN1",
            ],
            ops: json!([
                { "op": "set_net_field", "net": "P5V_X", "field": { "field": "rail", "value": { "value": 5.1, "unit": "V" } } },
                { "op": "set_net_field", "net": "P5V_X", "field": { "field": "master", "value": "OBC.CAN1" } }]),
        },
        Edit {
            name: "UnsetNetField",
            cli: &["net", "unset", "P5V_X", "label"],
            ops: json!([{ "op": "set_net_field", "net": "P5V_X", "field": { "field": "label", "value": null } }]),
        },
    ]
}

fn case_edits() -> Vec<Edit> {
    vec![
        Edit {
            name: "SetCase",
            cli: &[
                "case",
                "set",
                "hot_new",
                "--label",
                "Hot",
                "--mode",
                "MISSION",
                "--beta",
                "60 deg",
                "--orbits",
                "3",
                "--epoch",
                "2027-08-15T12:30:00Z",
            ],
            ops: json!([{ "op": "set_case", "case": { "id": "hot_new", "label": "Hot", "mode": "MISSION",
                "beta": { "value": 60.0, "unit": "deg" }, "orbits": 3, "epoch": "2027-08-15T12:30:00Z" } }]),
        },
        Edit {
            name: "SetSource",
            cli: &[
                "source",
                "set",
                "ds_new",
                "--kind",
                "datasheet",
                "--label",
                "New datasheet",
                "--ref",
                "https://example.com/a.pdf",
                "--revision",
                "Rev. C",
                "--retrieved",
                "2026-10-15",
                "--sha256",
                "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
                "--distribution",
                "on_request",
            ],
            ops: json!([{ "op": "set_source", "source": { "id": "ds_new", "kind": "datasheet",
                "label": "New datasheet", "ref": "https://example.com/a.pdf", "revision": "Rev. C",
                "retrieved": "2026-10-15",
                "sha256": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
                "distribution": "on_request" } }]),
        },
        Edit {
            name: "SetHarness",
            cli: &[
                "harness",
                "set",
                "H_NEW",
                "--from",
                "PCDU.OUT3",
                "--to",
                "RW_W.PWR",
                "--nets",
                "P12V",
                "--wire",
                "local/awg24_ptfe",
                "--cores",
                "2",
                "--length",
                "0.3 m",
            ],
            ops: json!([{ "op": "set_harness", "harness": { "id": "H_NEW", "from": "PCDU.OUT3", "to": "RW_W.PWR",
                "nets": ["P12V"], "wire": "local/awg24_ptfe", "cores": 2, "length": { "value": 0.3, "unit": "m" } } }]),
        },
    ]
}

fn schema_edits() -> Vec<Edit> {
    vec![
        Edit {
            name: "CreateAttribute",
            cli: &[
                "attr",
                "add",
                "max_dipole2",
                "--label",
                "Max dipole 2",
                "--type",
                "scalar",
                "--dimension",
                "MagneticDipoleMoment",
                "--units",
                "A*m^2",
            ],
            ops: json!([{ "op": "create_attribute", "def": { "id": "max_dipole2", "label": "Max dipole 2",
                "type": "scalar", "dimension": "MagneticDipoleMoment", "units": ["A*m^2"] } }]),
        },
        Edit {
            name: "CreateKind",
            cli: &[
                "kind",
                "add",
                "wire2",
                "--label",
                "Wire 2",
                "--states",
                "off,on",
                "--required",
                "mass",
            ],
            ops: json!([{ "op": "create_kind", "def": { "id": "wire2", "label": "Wire 2",
                "states": ["off", "on"], "required": ["mass"] } }]),
        },
    ]
}

fn run_http(rt: &tokio::runtime::Runtime, root: &Path, ops: &Value) {
    post_json(rt, root, "/api/v1/ops", json!({ "ops": ops }));
}

#[test]
fn cli_and_http_write_identical_files() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let before = snapshot_files(&cli_root);
    for edit in edits() {
        run_cli(&cli_root, edit.cli);
        run_http(&rt, &http_root, &edit.ops);
        assert_same_files(&cli_root, &http_root, edit.name);
    }
    assert_ne!(
        before,
        snapshot_files(&cli_root),
        "the edit did not change the files"
    );
}

#[test]
fn dry_run_writes_nothing_on_both_sides() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t, root) = demo_copy();
    let before = snapshot_files(&root);
    run_cli(&root, &["set", "RW_X.mass", "700 g", "--dry-run"]);
    let ws = Workspace::open(&root).unwrap();
    let router = fune_server::router(Arc::new(ws));
    let body = json!({ "ops": edits()[0].ops, "dry_run": true }).to_string();
    let req = Request::post("/api/v1/ops")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    let status = rt.block_on(async { router.oneshot(req).await.unwrap().status() });
    assert_eq!(status, StatusCode::OK);
    assert_eq!(before, snapshot_files(&root));
}

/// 系の所属と配分キーの編集・読み出しは CLI と HTTP で同じ。
#[test]
fn system_membership_and_budget_keys_match_http() {
    use common::{fune, get_json};
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    run_cli(&cli_root, &["part", "mv", "RW_X", "payload"]);
    run_http(
        &rt,
        &http_root,
        &json!([{ "op": "move_part", "id": "RW_X", "to": "payload" }]),
    );
    let key = "budget.mass.alloc.payload";
    run_cli(&cli_root, &["setting", "set", key, "1.8 kg"]);
    let ops = post_json(
        &rt,
        &http_root,
        "/api/v1/ops/from_text",
        json!({ "path": key, "value": "1.8 kg" }),
    );
    post_json(&rt, &http_root, "/api/v1/ops", json!({ "ops": [ops] }));
    assert_same_files(&cli_root, &http_root, "system and allocation");
    for (args, endpoint) in [
        (vec!["part", "ls"], "/api/v1/parts"),
        (vec!["budget", "mass"], "/api/v1/budgets/mass"),
    ] {
        let output = fune()
            .arg("-C")
            .arg(&cli_root)
            .arg("--json")
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let cli: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(cli, get_json(&rt, &http_root, endpoint));
    }
    let parts = get_json(&rt, &http_root, "/api/v1/parts");
    let wheel = parts
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "RW_X")
        .unwrap();
    assert_eq!(wheel["system"], "payload");
    assert!(wheel.get("subsystem").is_none());
}
