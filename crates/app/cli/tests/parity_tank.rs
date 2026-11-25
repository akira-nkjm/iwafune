//! パリティテスト（CI 必須）: スラスタが推薬を取るタンク（属性 `tank`）を `fune set` と `POST /ops/from_text` で書くと、
//! ファイルはバイト単位で一致する。タンクの部品の名前を変えれば `tank` も追従し、まだ名指されているタンクは消せない。

mod common;

use common::{assert_same_files, demo_copy, fune, post_json, run_cli};
use serde_json::json;

const THRUSTER: &str = "parts/propulsion/EPT.toml";

#[test]
fn naming_the_tank_matches_cli_and_follows_a_rename() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    run_cli(&cli_root, &["set", "EPT.tank", "XE_TANK"]);
    let op = post_json(
        &rt,
        &http_root,
        "/api/v1/ops/from_text",
        json!({ "path": "EPT.tank", "value": "XE_TANK" }),
    );
    post_json(&rt, &http_root, "/api/v1/ops", json!({ "ops": [op] }));
    assert_same_files(&cli_root, &http_root, "set EPT.tank");
    let text = std::fs::read_to_string(cli_root.join(THRUSTER)).unwrap();
    assert!(
        text.contains("tank") && text.contains("\"XE_TANK\""),
        "{text}"
    );
    run_cli(&cli_root, &["fmt", "--check"]);
    run_cli(&cli_root, &["check"]);
    // タンクの部品を消そうとしても、名指されている間は拒まれる
    let refused = fune()
        .arg("-C")
        .arg(&cli_root)
        .args(["part", "rm", "XE_TANK"])
        .output()
        .unwrap();
    assert!(!refused.status.success());
    // 名前の変更は `tank` に追従する（CLI と HTTP で同じファイル）
    run_cli(&cli_root, &["part", "rename", "XE_TANK", "TANK_XE"]);
    post_json(
        &rt,
        &http_root,
        "/api/v1/ops",
        json!({ "ops": [{ "op": "rename_part", "id": "XE_TANK", "to": "TANK_XE" }] }),
    );
    assert_same_files(&cli_root, &http_root, "rename the tank");
    let text = std::fs::read_to_string(cli_root.join(THRUSTER)).unwrap();
    assert!(
        text.contains("\"TANK_XE\"") && !text.contains("\"XE_TANK\""),
        "{text}"
    );
    run_cli(&cli_root, &["check"]);
}
