//! パリティテスト（CI 必須）: 出典のディレクトリ（`fune source set --subdir`・`mv`）と、同じ編集の `POST /ops` は、
//! 結果のファイルがバイト単位で一致する。出典の `dir`（`fune source ls` と `GET /design`）も同じ。

mod common;

use common::{assert_same_files, demo_copy, fune, get_json, post_json, run_cli};
use serde_json::json;

const SET: [&str; 6] = ["source", "set", "src_new", "--kind", "judgement", "--label"];

#[test]
fn set_in_a_directory_and_mv_match_http() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t1, cli) = demo_copy();
    let (_t2, http) = demo_copy();
    let new = |dir: Option<&str>, label: &str| {
        let mut o = json!({ "op": "set_source",
            "source": { "id": "src_new", "kind": "judgement", "label": label } });
        if let Some(d) = dir {
            o["dir"] = json!(d);
        }
        json!([o])
    };
    let steps = [
        (
            [&SET[..], &["One", "--subdir", "notes/a"]].concat(),
            new(Some("notes/a"), "One"),
        ),
        // 既にある出典は今のファイルに書く
        ([&SET[..], &["Two"]].concat(), new(None, "Two")),
        (
            vec!["source", "mv", "src_new", "notes/b"],
            json!([{ "op": "move_source", "id": "src_new", "dir": "notes/b" }]),
        ),
        (
            vec!["source", "mv", "ds_rw10", "datasheets"],
            json!([{ "op": "move_source", "id": "ds_rw10", "dir": "datasheets" }]),
        ),
        (
            vec!["source", "mv", "src_new", "."],
            json!([{ "op": "move_source", "id": "src_new", "dir": "." }]),
        ),
    ];
    for (i, (args, ops)) in steps.iter().enumerate() {
        run_cli(&cli, args);
        post_json(&rt, &http, "/api/v1/ops", json!({ "ops": ops }));
        assert_same_files(&cli, &http, &format!("step {i}"));
        if i == 3 {
            assert!(cli.join("sources/datasheets/ds_rw10.toml").is_file());
            let out = fune()
                .arg("-C")
                .arg(&cli)
                .arg("--json")
                .args(["source", "ls"])
                .output()
                .unwrap();
            let listed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
            let moved = |v: &serde_json::Value| {
                v.as_array()
                    .unwrap()
                    .iter()
                    .find(|s| s["id"] == "ds_rw10")
                    .map(|s| s["dir"].clone())
            };
            assert_eq!(moved(&listed), Some(json!(["datasheets"])));
            let design = get_json(&rt, &http, "/api/v1/design");
            assert_eq!(moved(&design["sources"]), Some(json!(["datasheets"])));
        }
    }
    assert!(cli.join("sources/src_new.toml").is_file());
    assert!(!cli.join("sources/notes").exists());
    run_cli(&cli, &["fmt", "--check"]);
    run_cli(&cli, &["check"]);
}
