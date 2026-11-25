//! パリティテスト（CI 必須）: 属性を `[attr.<名前>]` の表で書いた部品ファイル（デモの推進器）に、まだ無い属性を足す。
//! `fune set` と `POST /ops/from_text` は同じ service の関数を通るので、書かれるファイルはバイト単位で一致し、
//! 足した属性は正規の形と位置（値の行は先頭の `[attr]`、出典付きは `[attr.<名前>]`）に入る。

mod common;

use common::{assert_same_files, demo_copy, post_json, run_cli};
use serde_json::json;

const FILE: &str = "parts/propulsion/EPT.toml";

#[test]
fn adding_an_attribute_to_a_table_style_file_matches_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    for root in [&cli_root, &http_root] {
        run_cli(root, &["unset", "EPT.isp"]);
    }
    // 出典なし: `[attr]` の値の行（下の `[attr.<名前>]` より前）
    run_cli(&cli_root, &["set", "EPT.isp", "1200 s"]);
    let op = post_json(
        &rt,
        &http_root,
        "/api/v1/ops/from_text",
        json!({ "path": "EPT.isp", "value": "1200 s" }),
    );
    post_json(&rt, &http_root, "/api/v1/ops", json!({ "ops": [op] }));
    assert_same_files(&cli_root, &http_root, "set EPT.isp");
    let text = std::fs::read_to_string(http_root.join(FILE)).unwrap();
    let header = text
        .find("\n[attr]\nisp = { value = 1200.0, unit = \"s\" }\n")
        .unwrap();
    assert!(header < text.find("[attr.mass]").unwrap(), "{text}");
    run_cli(&http_root, &["fmt", "--check"]);
    // 出典付き: `[attr.isp]` が ID の順の位置に入る（ファイルの終わりではない）
    for root in [&cli_root, &http_root] {
        run_cli(root, &["unset", "EPT.isp"]);
    }
    run_cli(
        &cli_root,
        &["set", "EPT.isp", "1200 s", "--source", "sys_judgement"],
    );
    let op = post_json(
        &rt,
        &http_root,
        "/api/v1/ops/from_text",
        json!({ "path": "EPT.isp", "value": "1200 s", "source": "sys_judgement" }),
    );
    post_json(&rt, &http_root, "/api/v1/ops", json!({ "ops": [op] }));
    assert_same_files(&cli_root, &http_root, "set EPT.isp --source");
    let text = std::fs::read_to_string(http_root.join(FILE)).unwrap();
    assert!(
        text.find("[attr.direction]").unwrap() < text.find("[attr.isp]").unwrap()
            && text.find("[attr.isp]").unwrap() < text.find("[attr.size]").unwrap(),
        "{text}"
    );
    assert!(!text.contains("\n[attr]\n"), "{text}");
    run_cli(&http_root, &["fmt", "--check"]);
}
