//! パリティテスト（CI 必須）: 熱のページの結合・表面の編集。GUI は下書きを `POST /ops/from_command`（`thermal_set`）で
//! `SetSetting` の列にしてから `POST /ops` で送る。CLI は同じ設定を `fune set`・`fune unset` で書く。
//! 書かれるファイルがバイト単位で一致することと、検査の誤り（逆向きの重複・使用中の表面材）が書き込まずに弾かれること、
//! 表（`POST /thermal/tables`）が出どころと下書きを当てた値を返すことを確かめる。

mod common;

use common::{assert_same_files, demo_copy, post_json, post_json_response, run_cli};
use serde_json::{Value, json};
use std::path::Path;

/// 打った引数 → Op の列（HTTP）→ 適用（HTTP）。
fn http_thermal(rt: &tokio::runtime::Runtime, root: &Path, cells: Value) {
    let body = json!({ "command": "thermal_set", "cells": cells });
    let ops = post_json(rt, root, "/api/v1/ops/from_command", body);
    assert!(
        ops.as_array()
            .unwrap()
            .iter()
            .all(|op| op["op"] == "set_setting"),
        "{ops}"
    );
    post_json(rt, root, "/api/v1/ops", json!({ "ops": ops }));
}

fn tables(rt: &tokio::runtime::Runtime, root: &Path, ops: Value) -> Value {
    post_json(rt, root, "/api/v1/thermal/tables", json!({ "ops": ops }))
}

fn row<'a>(t: &'a Value, kind: &str, a: &str, b: &str) -> Option<&'a Value> {
    t["couplings"]["rows"]
        .as_array()?
        .iter()
        .find(|r| r["kind"] == kind && r["a"] == a && r["b"] == b)
}

#[test]
fn thermal_edits_match_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli) = demo_copy();
    let (_t2, http) = demo_copy();
    // (CLI の引数の列, 下書きのセルの列)
    let steps: Vec<(Vec<Vec<&str>>, Value)> = vec![
        (
            vec![vec![
                "set",
                "analysis.thermal.conductive.CAM.PANEL_PZ",
                "0.3 W/K",
            ]],
            json!([{ "cell": "coupling", "kind": "conductive", "a": "CAM", "b": "PANEL_PZ", "value": "0.3 W/K" }]),
        ),
        (
            vec![vec![
                "set",
                "analysis.thermal.radiative.BAT.CAM",
                "0.01 m^2",
            ]],
            json!([{ "cell": "coupling", "kind": "radiative", "a": "BAT", "b": "CAM", "value": "0.01 m^2" }]),
        ),
        // 面積割合の `1` は整数でなく小数で書く（`fune set` も同じ）
        (
            vec![
                vec!["set", "analysis.thermal.coating.grey.label", "Grey"],
                vec!["set", "analysis.thermal.coating.grey.absorptivity", "0.5"],
                vec!["set", "analysis.thermal.coating.grey.emissivity", "0.7"],
                vec!["set", "analysis.thermal.surface.PANEL_PY.grey", "1"],
            ],
            json!([
                { "cell": "coating", "id": "grey", "absorptivity": "0.5", "emissivity": "0.7", "label": "Grey" },
                { "cell": "surface", "part": "PANEL_PY", "coating": "grey", "fraction": "1" },
            ]),
        ),
        (
            vec![
                vec![
                    "set",
                    "analysis.thermal.surface.PANEL_PX.white_paint",
                    "0.7",
                ],
                vec![
                    "set",
                    "analysis.thermal.surface.PANEL_PX.black_paint",
                    "0.3",
                ],
            ],
            json!([
                { "cell": "surface", "part": "PANEL_PX", "coating": "white_paint", "fraction": "0.7" },
                { "cell": "surface", "part": "PANEL_PX", "coating": "black_paint", "fraction": "0.3" },
            ]),
        ),
        (
            vec![vec!["unset", "analysis.thermal.conductive.CAM.PANEL_PZ"]],
            json!([{ "cell": "coupling", "kind": "conductive", "a": "CAM", "b": "PANEL_PZ", "value": null }]),
        ),
        (
            vec![
                vec!["unset", "analysis.thermal.surface.PANEL_PY"],
                vec!["unset", "analysis.thermal.coating.grey"],
            ],
            json!([
                { "cell": "surface_remove", "part": "PANEL_PY" },
                { "cell": "coating_remove", "id": "grey" },
            ]),
        ),
    ];
    for (n, (commands, cells)) in steps.into_iter().enumerate() {
        for args in &commands {
            run_cli(&cli, args);
        }
        http_thermal(&rt, &http, cells);
        assert_same_files(&cli, &http, &format!("step {n}"));
    }
    let text = std::fs::read_to_string(http.join("analysis/thermal.toml")).unwrap();
    assert!(
        !text.contains("grey") && text.contains("white_paint = 0.7"),
        "{text}"
    );
}

/// 表は解析が使った結合を出どころつきで返し、下書き（`ops`）を書き込まずに当てた値を返す。
#[test]
fn tables_show_sources_and_apply_the_draft() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t, root) = demo_copy();
    let saved = tables(&rt, &root, json!([]));
    assert_eq!(saved["couplings"]["available"], true);
    let explicit = row(&saved, "conductive", "BAT", "PANEL_MZ").expect("explicit pair");
    assert_eq!(explicit["source"], "explicit");
    assert_eq!(explicit["explicit"], true);
    assert_eq!(explicit["value"]["display"], "0.0300 W/K", "{explicit}");
    assert!(
        saved["couplings"]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["source"] == "mount" && r["explicit"] == false)
    );
    assert!(saved["couplings"]["nodes"].as_array().unwrap().len() > 10);
    assert!(
        saved["surfaces"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["part"] == "PANEL_PX" && s["fractions"].as_array().unwrap().len() == 2)
    );
    assert!(saved["coatings"].as_array().unwrap().len() >= 2);
    assert!(!saved["temperatures"].as_array().unwrap().is_empty());

    // 下書き: 明示の結合を 0 にして切り離す（保存はしない）
    let cells = json!([{ "cell": "coupling", "kind": "conductive", "a": "BAT", "b": "PANEL_MZ", "value": "0 W/K" }]);
    let ops = post_json(
        &rt,
        &root,
        "/api/v1/ops/from_command",
        json!({ "command": "thermal_set", "cells": cells }),
    );
    let draft = tables(&rt, &root, ops);
    assert_eq!(
        row(&draft, "conductive", "BAT", "PANEL_MZ").unwrap()["value"]["si"],
        0.0
    );
    assert_ne!(draft["temperatures"], saved["temperatures"]);
    let again = tables(&rt, &root, json!([]));
    assert_eq!(again, saved);
}

/// 検査の誤りは書き込まずに弾かれ、理由に検査の符号が出る（逆向きの重複・自分自身の対・未知の節点・使用中の表面材）。
#[test]
fn invalid_edits_are_refused_by_the_checks() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t, root) = demo_copy();
    let before = std::fs::read_to_string(root.join("analysis/thermal.toml")).unwrap();
    let refused = |cells: Value, code: &str| {
        let body = json!({ "command": "thermal_set", "cells": cells });
        let ops = post_json(&rt, &root, "/api/v1/ops/from_command", body);
        let (status, text) = post_json_response(&rt, &root, "/api/v1/ops", json!({ "ops": ops }));
        assert_ne!(status, axum::http::StatusCode::OK, "{text}");
        assert!(text.to_string().contains(code), "{code}: {text}");
    };
    let coupling = |a: &str, b: &str| json!([{ "cell": "coupling", "kind": "conductive", "a": a, "b": b, "value": "0.5 W/K" }]);
    refused(coupling("PANEL_MZ", "BAT"), "thermal.coupling_duplicate");
    refused(coupling("BAT", "BAT"), "thermal.coupling_self");
    refused(coupling("BAT", "NOPE"), "ref.thermal_node");
    refused(
        json!([{ "cell": "coating_remove", "id": "white_paint" }]),
        "ref.coating",
    );
    let after = std::fs::read_to_string(root.join("analysis/thermal.toml")).unwrap();
    assert_eq!(before, after);
}
