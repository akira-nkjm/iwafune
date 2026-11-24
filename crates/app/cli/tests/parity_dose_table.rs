//! パリティテスト（CI 必須）: 放射線の環境の表（`conditions.orbit.dose_rate_by_shielding`。曲線の設定の値）と、部品の遮蔽厚さ・許容線量の
//! 書き込みと解除。既存の Op（`SetSetting`・`SetValue`）だけで編集する（設計の軌道は `fune set` と `POST /ops/from_text`、フェーズの軌道は
//! `fune phase set-field` と `CommandText::PhaseField`）。表の文字列は、書かれていなくても欄の型（model の `OrbitSettings::field_shape`）で読む。
//! CLI と HTTP が書くファイルはバイト単位で一致し、書いたファイルは `fune fmt --check` と `fune check` を通る。

mod common;

use common::{assert_same_files, demo_copy, post_json, run_cli};
use serde_json::{Value, json};
use std::path::Path;

const TABLE: &str = "0.5:400 1:120 2:30 4:5 8:0.8 (mm -> Gy/yr)";

fn http_command(rt: &tokio::runtime::Runtime, root: &Path, body: Value) {
    let ops = post_json(rt, root, "/api/v1/ops/from_command", body);
    post_json(rt, root, "/api/v1/ops", json!({ "ops": ops }));
}

fn http_text(rt: &tokio::runtime::Runtime, root: &Path, uri: &str, body: Value) {
    let op = post_json(rt, root, uri, body);
    post_json(rt, root, "/api/v1/ops", json!({ "ops": [op] }));
}

#[test]
fn the_dose_table_and_part_values_are_written_alike_by_the_cli_and_http() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let original = std::fs::read(cli_root.join("conditions/orbit.toml")).unwrap();

    let path = "conditions.orbit.dose_rate_by_shielding";
    run_cli(&cli_root, &["set", path, TABLE]);
    http_text(
        &rt,
        &http_root,
        "/api/v1/ops/from_text",
        json!({ "path": path, "value": TABLE }),
    );
    assert_same_files(&cli_root, &http_root, path);
    let text = std::fs::read_to_string(http_root.join("conditions/orbit.toml")).unwrap();
    assert!(
        text.contains("at = [0.5, 1.0, 2.0, 4.0, 8.0]")
            && text.contains("at_unit = \"mm\"")
            && text.contains("values = [400.0, 120.0, 30.0, 5.0, 0.8]")
            && text.contains("unit = \"Gy/yr\""),
        "{text}"
    );

    // 部品の遮蔽厚さと許容線量（既存の属性の書き込み）
    for (attr, value) in [("OBC.shielding", "2 mm"), ("OBC.tid_tolerance", "100 Gy")] {
        run_cli(&cli_root, &["set", attr, value]);
        http_text(
            &rt,
            &http_root,
            "/api/v1/ops/from_text",
            json!({ "path": attr, "value": value }),
        );
        assert_same_files(&cli_root, &http_root, attr);
    }

    // フェーズの軌道の表
    let phase_table = "0.5:200 1:60 (mm -> Gy/yr)";
    run_cli(
        &cli_root,
        &[
            "phase",
            "set-field",
            "DISPOSAL",
            "orbit.dose_rate_by_shielding",
            phase_table,
        ],
    );
    http_command(
        &rt,
        &http_root,
        json!({ "command": "phase_field", "phase": "DISPOSAL", "path": "orbit.dose_rate_by_shielding", "value": phase_table }),
    );
    assert_same_files(&cli_root, &http_root, "phase table");

    run_cli(&http_root, &["fmt", "--check"]);
    run_cli(&http_root, &["check"]);

    // 解除すると書く前のファイルに戻る
    run_cli(&cli_root, &["unset", path]);
    http_text(
        &rt,
        &http_root,
        "/api/v1/ops/unset_from_text",
        json!({ "path": path }),
    );
    assert_same_files(&cli_root, &http_root, "unset");
    assert_eq!(
        std::fs::read(http_root.join("conditions/orbit.toml")).unwrap(),
        original
    );
}
