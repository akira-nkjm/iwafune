//! パリティテスト（CI 必須）: 軌道の出どころ `state`（状態ベクトル）の欄（`source`・`epoch`・`central`・`position`・`velocity`・`span`）の
//! 書き込みと解除。既存の Op だけで編集する（設計の軌道は `fune set` と `POST /ops/from_text`・`/ops/unset_from_text`、フェーズの軌道は
//! `fune phase set-field|unset-field` と `CommandText::PhaseField`）。ベクトルの欄（`position`・`velocity`）の文字列は、
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

const KEYS: [(&str, &str); 5] = [
    ("epoch", "2030-01-01T00:00:00Z"),
    ("position", "7000 0 0 km"),
    ("velocity", "0 7.5 0 km/s"),
    ("span", "2 d"),
    ("source", "state"),
];

#[test]
fn state_orbit_edits_match_cli_on_the_design_orbit_and_on_a_phase() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let original = std::fs::read(cli_root.join("conditions/orbit.toml")).unwrap();

    // 設計の軌道: 書いて、`fmt --check` を通し、解除する
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
    assert!(
        text.contains("position        = { x = 7000.0, y = 0.0, z = 0.0, unit = \"km\" }")
            && text.contains("velocity        = { x = 0.0, y = 7.5, z = 0.0, unit = \"km/s\" }")
            && text.contains("span            = { value = 2.0, unit = \"d\" }")
            && text.contains("source          = \"state\""),
        "{text}"
    );
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

    // フェーズの軌道（太陽が原点）
    run_cli(&cli_root, &["phase", "add", "CRUISE", "--order", "5"]);
    http_command(
        &rt,
        &http_root,
        json!({ "command": "phase_add", "id": "CRUISE", "order": "5" }),
    );
    let phase_keys = [
        ("central", "sun"),
        ("position", "1.495978707e8 0 0 km"),
        ("velocity", "0 29.78 0 km/s"),
        ("span", "365 d"),
        ("epoch", "2030-01-01T00:00:00Z"),
        ("source", "state"),
    ];
    for (key, value) in phase_keys {
        let path = format!("orbit.{key}");
        run_cli(&cli_root, &["phase", "set-field", "CRUISE", &path, value]);
        http_command(
            &rt,
            &http_root,
            json!({ "command": "phase_field", "phase": "CRUISE", "path": path, "value": value }),
        );
        assert_same_files(&cli_root, &http_root, &path);
    }
    let text = std::fs::read_to_string(http_root.join("phases/CRUISE.toml")).unwrap();
    assert!(
        text.contains("position")
            && text.contains("x = 149597870.7")
            && text.contains("\"state\"")
            && text.contains("central"),
        "{text}"
    );
    run_cli(&http_root, &["fmt", "--check"]);
    run_cli(&http_root, &["check"]);
}
