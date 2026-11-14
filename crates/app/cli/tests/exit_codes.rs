//! 終了コード: `fune catalog show` の「無い」は service の NotFound（2）、`fune budget` は `--json` でも表でも同じ終了コード。

mod common;

use common::{demo_copy, fune};
use std::path::Path;

fn code(root: &Path, args: &[&str]) -> Option<i32> {
    fune()
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap()
        .status
        .code()
}

#[test]
fn catalog_show_unknown_id_is_not_found() {
    let (_tmp, root) = demo_copy();
    assert_eq!(
        code(&root, &["catalog", "show", "local/rw_10mNms"]),
        Some(0)
    );
    assert_eq!(code(&root, &["catalog", "show", "local/nope"]), Some(2));
    let out = fune()
        .arg("-C")
        .arg(&root)
        .args(["--json", "catalog", "show", "local/nope"])
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["error"]["code"], "not_found");
}

#[test]
fn budget_exit_code_is_the_same_for_json_and_table() {
    let (_tmp, root) = demo_copy();
    // 通る
    for flags in [&["budget", "mass"][..], &["--json", "budget", "mass"]] {
        assert_eq!(code(&root, flags), Some(0), "{flags:?}");
    }
    // 入力が足りない（データ量のモードの割合を消す）
    let ops = root.join("missing.json");
    std::fs::write(
        &ops,
        r#"[{ "op": "set_setting", "path": "analysis.power.mode_share", "value": null }]"#,
    )
    .unwrap();
    let ops = ops.to_str().unwrap();
    for flags in [
        &["budget", "data", "--ops", ops][..],
        &["--json", "budget", "data", "--ops", ops],
    ] {
        assert_eq!(code(&root, flags), Some(1), "{flags:?}");
    }
    // 違反（質量の上限を 1 kg に下げる）
    let file = root.join("budget/allocation.toml");
    let text = std::fs::read_to_string(&file).unwrap();
    let tight = text.replace(
        "limit         = { value = 12.0, unit = \"kg\" }",
        "limit         = { value = 1.0, unit = \"kg\" }",
    );
    assert_ne!(text, tight);
    std::fs::write(&file, tight).unwrap();
    for flags in [&["budget", "mass"][..], &["--json", "budget", "mass"]] {
        assert_eq!(code(&root, flags), Some(1), "{flags:?}");
    }
}
