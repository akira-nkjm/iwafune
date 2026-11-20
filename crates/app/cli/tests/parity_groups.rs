//! パリティテスト: グループへの部品の出し入れを CLI（`fune part mv|add` の `aocs/NAV`）と
//! HTTP（`POST /api/v1/ops` の構造の Op）で行い、結果のファイルがバイト単位で一致することを確かめる。

mod common;

use common::{assert_same_files, demo_copy, post_json, run_cli};
use serde_json::{Value, json};

struct Step {
    name: &'static str,
    cli: &'static [&'static str],
    ops: Value,
}

fn steps() -> Vec<Step> {
    let into = |id: &str| json!({ "op": "move_part", "id": id, "to": "NAV", "under": ["aocs"] });
    vec![
        Step {
            name: "group first",
            cli: &["part", "mv", "GPS", "aocs/NAV"],
            ops: json!([into("GPS")]),
        },
        Step {
            name: "group second (existing group by path)",
            cli: &["part", "mv", "STT", "aocs/NAV"],
            ops: json!([into("STT")]),
        },
        Step {
            name: "add into the group",
            cli: &[
                "part",
                "add",
                "RW_W",
                "--kind",
                "reaction_wheel",
                "--group",
                "aocs/NAV",
            ],
            ops: json!([{ "op": "create_part", "id": "RW_W", "group": "NAV",
                "group_under": ["aocs"], "kind": "reaction_wheel" }]),
        },
        Step {
            name: "ungroup one",
            cli: &["part", "mv", "GPS", "aocs"],
            ops: json!([{ "op": "move_part", "id": "GPS", "to": "aocs" }]),
        },
        Step {
            name: "ungroup the rest (empty folder is removed)",
            cli: &["part", "mv", "STT", "aocs"],
            ops: json!([{ "op": "move_part", "id": "STT", "to": "aocs" }]),
        },
        Step {
            name: "ungroup the last",
            cli: &["part", "mv", "RW_W", "aocs"],
            ops: json!([{ "op": "move_part", "id": "RW_W", "to": "aocs" }]),
        },
    ]
}

#[test]
fn grouping_through_cli_and_http_writes_identical_files() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    for step in steps() {
        run_cli(&cli_root, step.cli);
        post_json(&rt, &http_root, "/api/v1/ops", json!({ "ops": step.ops }));
        assert_same_files(&cli_root, &http_root, step.name);
        if step.name.starts_with("group second") {
            assert!(cli_root.join("parts/aocs/NAV/GPS.toml").exists());
            assert!(!cli_root.join("parts/aocs/GPS.toml").exists());
        }
    }
    assert!(
        !cli_root.join("parts/aocs/NAV").exists(),
        "the emptied group folder must be removed"
    );
    assert!(cli_root.join("parts/aocs/RW_W.toml").exists());
}
