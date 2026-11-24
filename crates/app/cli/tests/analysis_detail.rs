//! 重い任意の出力（`fune analyze <id> --detail` と `POST /analyses/{id}` の `detail`）: 既定の結果には出ず、頼むと
//! CLI と HTTP が同じものを返す。解析 `structure.modal_fem` のモードの形で確かめる。
mod common;
use common::{demo_copy, fune, post_json};
use serde_json::{Value, json};
use std::path::Path;

fn analyze(root: &Path, id: &str, detail: bool) -> Value {
    let mut args = vec!["-C", root.to_str().unwrap(), "--json", "analyze", id];
    if detail {
        args.push("--detail");
    }
    let out = fune().args(args).output().unwrap();
    assert!(
        out.status.success() || out.status.code() == Some(1),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn detail_is_opt_in_and_the_same_over_cli_and_http() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_temp, root) = demo_copy();
    let id = "structure.modal_fem";

    let plain = analyze(&root, id, false);
    assert_eq!(plain["status"], "ok");
    assert!(plain.get("detail").is_none());

    let cli = analyze(&root, id, true);
    let http = post_json(
        &rt,
        &root,
        &format!("/api/v1/analyses/{id}"),
        json!({ "detail": true }),
    );
    let http_plain = post_json(&rt, &root, &format!("/api/v1/analyses/{id}"), json!({}));
    assert!(http_plain.get("detail").is_none());
    assert_eq!(http_plain, plain, "the default result is the same");

    let detail = &cli["detail"];
    assert_eq!(
        detail, &http["detail"],
        "CLI and HTTP return the same detail"
    );
    assert_eq!(detail["normalization"], "max_translation");
    let nodes = detail["mesh"]["nodes"].as_array().unwrap().len() as u64;
    assert_eq!(nodes, plain["output"]["model"]["nodes"].as_u64().unwrap());
    assert_eq!(
        detail["modes"].as_array().unwrap().len(),
        plain["output"]["modes"].as_array().unwrap().len()
    );
    // 詳細を頼んでも、ほかの欄は変わらない
    let mut without = cli.clone();
    without.as_object_mut().unwrap().remove("detail");
    assert_eq!(without, plain);
}

#[test]
fn detail_is_ignored_by_analyses_without_one() {
    let (_temp, root) = demo_copy();
    let id = "structure.mass_properties";
    assert_eq!(analyze(&root, id, true), analyze(&root, id, false));
}
