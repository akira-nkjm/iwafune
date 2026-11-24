//! パリティテスト（CI 必須）: 設定の一覧（文字列の列）の編集。値の型は欄の形（`fune_model::design::setting_lists`）で決まり、
//! 文字列から推し量らない。CLI（`fune set`・`fune setting set`）と `POST /ops/from_text` は同じ service の関数を通るので、
//! 書かれるファイルはバイト単位で一致する。

mod common;

use common::{assert_same_files, demo_copy, post_json, run_cli};
use serde_json::json;

/// 書かれていない一覧の欄にも、カンマ区切りと TOML の配列で一覧を書ける。消せば欄ごと無くなる。
#[test]
fn list_settings_match_cli_and_use_the_key_shape() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    let edits = [
        // まだ無い節点（`[nodes]` に無いキー）
        ("analysis.thermal.nodes.PROP", "EPT, XE_TANK"),
        // 既にある節点を配列で書き換える（順序も保つ）
        ("analysis.thermal.nodes.PROP", "[\"XE_TANK\", \"EPT\"]"),
        ("analysis.thermal.nodes.PROP", "EPT"),
    ];
    for (path, value) in edits {
        // `fune set` と `fune setting set` は同じ Op
        run_cli(&cli_root, &["set", path, value]);
        let op = post_json(
            &rt,
            &http_root,
            "/api/v1/ops/from_text",
            json!({ "path": path, "value": value }),
        );
        assert_eq!(op["op"], "set_setting", "{op}");
        assert_eq!(op["value"]["type"], "list", "{op}");
        post_json(&rt, &http_root, "/api/v1/ops", json!({ "ops": [op] }));
        assert_same_files(&cli_root, &http_root, &format!("{path} {value}"));
    }
    let thermal = std::fs::read_to_string(http_root.join("analysis/thermal.toml")).unwrap();
    assert!(thermal.contains("PROP     = [\"EPT\"]"), "{thermal}");

    // 消してから、書かれていない欄に書く（今の値の型に頼らない）
    let path = "analysis.comm.stations";
    run_cli(&cli_root, &["setting", "unset", path]);
    let op = post_json(
        &rt,
        &http_root,
        "/api/v1/ops/unset_from_text",
        json!({ "path": path }),
    );
    post_json(&rt, &http_root, "/api/v1/ops", json!({ "ops": [op] }));
    assert_same_files(&cli_root, &http_root, "unset stations");
    run_cli(&cli_root, &["setting", "set", path, "TOKYO_GS"]);
    let op = post_json(
        &rt,
        &http_root,
        "/api/v1/ops/from_text",
        json!({ "path": path, "value": "TOKYO_GS" }),
    );
    assert_eq!(
        op["value"],
        json!({ "type": "list", "value": ["TOKYO_GS"] })
    );
    post_json(&rt, &http_root, "/api/v1/ops", json!({ "ops": [op] }));
    assert_same_files(&cli_root, &http_root, "set stations");
    let comm = std::fs::read_to_string(http_root.join("analysis/comm.toml")).unwrap();
    assert!(comm.contains("stations = [\"TOKYO_GS\"]"), "{comm}");
}

/// 一覧の中の ID は、ファイルに書いたときと同じ検査（`ref.*`）を受ける。部品の名前の変更は一覧の中も書き換える。
#[test]
fn list_elements_are_checked_and_renamed() {
    let (_t, root) = demo_copy();
    let out = common::fune()
        .arg("-C")
        .arg(&root)
        .args(["set", "analysis.thermal.nodes.PROP", "EPT, NO_SUCH_PART"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("NO_SUCH_PART"), "{err}");
    run_cli(
        &root,
        &["set", "analysis.thermal.nodes.PROP", "EPT, XE_TANK"],
    );
    run_cli(&root, &["part", "rename", "XE_TANK", "XE_TANK2"]);
    let thermal = std::fs::read_to_string(root.join("analysis/thermal.toml")).unwrap();
    assert!(thermal.contains("[\"EPT\", \"XE_TANK2\"]"), "{thermal}");
}

/// `to_cli`（`fune history` と「CLI としてコピー」）は一覧をカンマ区切りで出し、それを打ち直すと同じファイルになる。
#[test]
fn list_setting_prints_back_as_a_comma_separated_command() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t, root) = demo_copy();
    let result = post_json(
        &rt,
        &root,
        "/api/v1/ops",
        json!({ "ops": [{ "op": "set_setting", "path": "analysis.thermal.nodes.PROP",
            "value": { "type": "list", "value": ["EPT", "XE_TANK"] } }], "dry_run": true }),
    );
    assert_eq!(
        result["cli"],
        json!(["fune setting set analysis.thermal.nodes.PROP EPT,XE_TANK"])
    );
}
