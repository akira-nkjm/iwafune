//! パリティテスト（CI 必須）: 軌道の出どころ（`source`・`maneuver`）の設定と解除。既存の Op だけで編集する
//! （設計の軌道は `fune set` と `POST /ops/from_text`・`/ops/unset_from_text`、フェーズの軌道は `fune phase set-field|unset-field` と
//! `CommandText::PhaseField`）。CLI と HTTP が書くファイルはバイト単位で一致し、解除すると書く前のファイルに戻る
//! （出どころの既定 `elements` は書かない）。

mod common;

use common::{assert_same_files, demo_copy, get_json, post_json, run_cli};
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

#[test]
fn source_and_maneuver_edits_match_cli_on_the_design_orbit_and_on_a_phase() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let original = std::fs::read(cli_root.join("conditions/orbit.toml")).unwrap();

    let add = [
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
        "--depart",
        "2028-12-10T00:00:00Z",
        "--arrive",
        "2029-09-25T00:00:00Z",
    ];
    run_cli(&cli_root, &add);
    http_command(
        &rt,
        &http_root,
        json!({ "command": "maneuver_add", "id": "MV_MARS", "category": "insertion",
            "method": "impulsive", "body": "mars", "perigee_altitude": "300 km",
            "depart": "2028-12-10T00:00:00Z", "arrive": "2029-09-25T00:00:00Z" }),
    );
    assert_same_files(&cli_root, &http_root, "maneuver add");

    // 設計の軌道: 書いて、解除する
    for (path, value) in [
        ("conditions.orbit.maneuver", "MV_MARS"),
        ("conditions.orbit.source", "transfer"),
    ] {
        run_cli(&cli_root, &["set", path, value]);
        http_text(
            &rt,
            &http_root,
            "/api/v1/ops/from_text",
            json!({ "path": path, "value": value }),
        );
        assert_same_files(&cli_root, &http_root, path);
    }
    let text = std::fs::read_to_string(http_root.join("conditions/orbit.toml")).unwrap();
    assert!(
        text.contains("source          = \"transfer\"")
            && text.contains("maneuver        = \"MV_MARS\""),
        "{text}"
    );
    for path in ["conditions.orbit.source", "conditions.orbit.maneuver"] {
        run_cli(&cli_root, &["unset", path]);
        let op = http_text(
            &rt,
            &http_root,
            "/api/v1/ops/unset_from_text",
            json!({ "path": path }),
        );
        assert_eq!(op["op"], "set_setting", "{op}");
        assert_same_files(&cli_root, &http_root, path);
    }
    assert_eq!(
        std::fs::read(http_root.join("conditions/orbit.toml")).unwrap(),
        original,
        "unsetting restores the file (the default source is never written)"
    );

    // フェーズの軌道
    run_cli(&cli_root, &["phase", "add", "CRUISE", "--order", "5"]);
    http_command(
        &rt,
        &http_root,
        json!({ "command": "phase_add", "id": "CRUISE", "order": "5" }),
    );
    for (path, value) in [
        ("orbit.maneuver", Some("MV_MARS")),
        ("orbit.source", Some("transfer")),
    ] {
        let value = value.unwrap();
        run_cli(&cli_root, &["phase", "set-field", "CRUISE", path, value]);
        http_command(
            &rt,
            &http_root,
            json!({ "command": "phase_field", "phase": "CRUISE", "path": path, "value": value }),
        );
        assert_same_files(&cli_root, &http_root, path);
    }
    let text = std::fs::read_to_string(http_root.join("phases/CRUISE.toml")).unwrap();
    assert!(
        text.contains("[orbit]")
            && text.contains("source")
            && text.contains("\"transfer\"")
            && text.contains("\"MV_MARS\""),
        "{text}"
    );
    // フェーズの一覧: 軌道の欄に選んで入れる欄が並び、巡航のマヌーバが分かる
    let phases = get_json(&rt, &http_root, "/api/v1/phases");
    let cruise = phases
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "CRUISE")
        .unwrap();
    assert_eq!(cruise["orbit_from"], "MV_MARS");
    let keys: Vec<&Value> = cruise["orbit"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| &f["key"])
        .collect();
    assert_eq!(keys, ["maneuver", "source"]);
    let maneuver_choices = &cruise["orbit"][0]["choices"];
    assert_eq!(maneuver_choices[0]["value"], "MV_MARS");
    assert_eq!(cruise["orbit"][1]["choices"][1]["value"], "transfer");
    for path in ["orbit.source", "orbit.maneuver"] {
        run_cli(&cli_root, &["phase", "unset-field", "CRUISE", path]);
        http_command(
            &rt,
            &http_root,
            json!({ "command": "phase_field", "phase": "CRUISE", "path": path }),
        );
        assert_same_files(&cli_root, &http_root, path);
    }
}
