//! パリティテスト（CI 必須）: カタログ品のディレクトリ（`fune catalog add --subdir`・`mv`）と、同じ編集の `POST /ops`
//! は、結果のファイルがバイト単位で一致する。品の `dir`（`fune catalog show` と `GET /catalog/{id}`）も同じ。

mod common;

use common::{assert_same_files, demo_copy, fune, get_json, post_json, run_cli};
use serde_json::json;

const ITEM: &str = "local/rw_40mNms";

#[test]
fn add_in_a_directory_and_mv_match_http() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli) = demo_copy();
    let (_t2, http) = demo_copy();
    let steps = [
        (
            vec![
                "catalog",
                "add",
                ITEM,
                "--kind",
                "reaction_wheel",
                "--subdir",
                "vendor_a/wheels",
            ],
            json!([{ "op": "create_catalog_item", "id": ITEM, "kind": "reaction_wheel",
                "dir": "vendor_a/wheels" }]),
        ),
        (
            vec!["catalog", "set", ITEM, "mass", "1.1 kg"],
            json!([{ "op": "set_catalog_value", "item": ITEM, "attr": "mass",
                "value": { "type": "scalar", "data": { "value": 1.1, "unit": "kg" } } }]),
        ),
        (
            vec!["catalog", "mv", ITEM, "vendor_b"],
            json!([{ "op": "move_catalog_item", "item": ITEM, "dir": "vendor_b" }]),
        ),
        (
            vec!["catalog", "rename", ITEM, "local/rw_41mNms"],
            json!([{ "op": "rename_catalog_item", "from": ITEM, "to": "local/rw_41mNms" }]),
        ),
        (
            vec!["catalog", "mv", "local/rw_41mNms", "."],
            json!([{ "op": "move_catalog_item", "item": "local/rw_41mNms", "dir": "." }]),
        ),
    ];
    for (i, (args, ops)) in steps.iter().enumerate() {
        run_cli(&cli, args);
        post_json(&rt, &http, "/api/v1/ops", json!({ "ops": ops }));
        assert_same_files(&cli, &http, &format!("step {i}"));
        if i == 2 {
            assert!(cli.join("catalog/vendor_b/rw_40mNms.toml").is_file());
            let out = fune()
                .arg("-C")
                .arg(&cli)
                .arg("--json")
                .args(["catalog", "show", ITEM])
                .output()
                .unwrap();
            let shown: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
            assert_eq!(shown["dir"], json!(["vendor_b"]));
            assert_eq!(
                shown,
                get_json(&rt, &http, &format!("/api/v1/catalog/{ITEM}"))
            );
        }
    }
    assert!(cli.join("catalog/rw_41mNms.toml").is_file());
    assert!(!cli.join("catalog/vendor_b").exists());
    // 移したあとも設計は正規形で、検査も通る
    run_cli(&cli, &["fmt", "--check"]);
    run_cli(&cli, &["check"]);
}
