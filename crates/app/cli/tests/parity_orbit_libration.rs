//! パリティテスト（CI 必須）: 軌道の出どころ `libration`（ラグランジュ点・ハロー軌道）の欄（`source`・`epoch`・`primary`・`secondary`・
//! `point`・`amplitude_z`・`branch`）の書き込みと解除。既存の Op だけで編集する（設計の軌道は `fune set` と `POST /ops/from_text`・
//! `/ops/unset_from_text`、フェーズの軌道は `fune phase set-field` と `CommandText::PhaseField`）。文字列で書く欄（天体・点・枝）は、
//! 書かれていなくても欄の型（model の `OrbitSettings::field_shape`）で読む。CLI と HTTP が書くファイルはバイト単位で一致し、
//! 書いたファイルは `fune fmt --check` を通り、解除すると書く前のファイルに戻る。

mod common;

use common::{assert_same_files, demo_copy, post_json, run_cli};
use serde_json::{Value, json};
use std::path::Path;

fn http_command(rt: &tokio::runtime::Runtime, root: &Path, body: Value) {
    let ops = post_json(rt, root, "/api/v1/ops/from_command", body);
    post_json(rt, root, "/api/v1/ops", json!({ "ops": ops }));
}

fn http_text(rt: &tokio::runtime::Runtime, root: &Path, uri: &str, body: Value) -> Value {
    let op = post_json(rt, root, uri, body);
    post_json(rt, root, "/api/v1/ops", json!({ "ops": [op.clone()] }));
    op
}

const KEYS: [(&str, &str); 7] = [
    ("epoch", "2030-01-01T00:00:00Z"),
    ("primary", "sun"),
    ("secondary", "earth"),
    ("point", "l2"),
    ("amplitude_z", "120000 km"),
    ("branch", "south"),
    ("source", "libration"),
];

#[test]
fn libration_orbit_edits_match_cli_on_the_design_orbit_and_on_a_phase() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let original = std::fs::read(cli_root.join("conditions/orbit.toml")).unwrap();

    for (key, value) in KEYS {
        let path = format!("conditions.orbit.{key}");
        run_cli(&cli_root, &["set", &path, value]);
        http_text(
            &rt,
            &http_root,
            "/api/v1/ops/from_text",
            json!({ "path": path, "value": value }),
        );
        assert_same_files(&cli_root, &http_root, &path);
    }
    let text = std::fs::read_to_string(http_root.join("conditions/orbit.toml")).unwrap();
    for expected in [
        "\"libration\"",
        "\"sun\"",
        "\"earth\"",
        "\"l2\"",
        "\"south\"",
        "{ value = 120000.0, unit = \"km\" }",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
    run_cli(&http_root, &["fmt", "--check"]);
    // `epoch` はデモの軌道が元から持つ欄なので、解除せず元の値に戻す
    for (key, _) in KEYS.iter().filter(|(k, _)| *k != "epoch") {
        let path = format!("conditions.orbit.{key}");
        run_cli(&cli_root, &["unset", &path]);
        let op = http_text(
            &rt,
            &http_root,
            "/api/v1/ops/unset_from_text",
            json!({ "path": path }),
        );
        assert_eq!(op["op"], "set_setting", "{op}");
        assert_same_files(&cli_root, &http_root, &path);
    }
    let (path, value) = ("conditions.orbit.epoch", "2027-04-01T00:00:00Z");
    run_cli(&cli_root, &["set", path, value]);
    http_text(
        &rt,
        &http_root,
        "/api/v1/ops/from_text",
        json!({ "path": path, "value": value }),
    );
    assert_same_files(&cli_root, &http_root, path);
    assert_eq!(
        std::fs::read(http_root.join("conditions/orbit.toml")).unwrap(),
        original,
        "unsetting restores the file"
    );

    // フェーズの軌道
    run_cli(&cli_root, &["phase", "add", "HALO", "--order", "5"]);
    http_command(
        &rt,
        &http_root,
        json!({ "command": "phase_add", "id": "HALO", "order": "5" }),
    );
    let phase_keys = [
        ("primary", "sun"),
        ("secondary", "earth"),
        ("point", "l1"),
        ("epoch", "2030-01-01T00:00:00Z"),
        ("source", "libration"),
    ];
    for (key, value) in phase_keys {
        let path = format!("orbit.{key}");
        run_cli(&cli_root, &["phase", "set-field", "HALO", &path, value]);
        http_command(
            &rt,
            &http_root,
            json!({ "command": "phase_field", "phase": "HALO", "path": path, "value": value }),
        );
        assert_same_files(&cli_root, &http_root, &path);
    }
    let text = std::fs::read_to_string(http_root.join("phases/HALO.toml")).unwrap();
    assert!(
        text.contains("\"libration\"") && text.contains("\"l1\""),
        "{text}"
    );
    run_cli(&http_root, &["fmt", "--check"]);
    run_cli(&http_root, &["check"]);
}
