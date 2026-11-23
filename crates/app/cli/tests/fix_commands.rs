//! `fune analyze` が出す直しのコマンドは、打つと本当に不足入力を消す（値と部品の参照）。
//! 候補を決めるのは `fune_service::fixes` で、CLI の表示も `--json` も GUI も同じ候補を使う。

mod common;

use common::{demo_copy, fune};
use serde_json::Value;
use std::path::Path;

/// `fune analyze <id> --json` の不足入力（終了コードは不足があれば 1 なので見ない）。
fn missing(root: &Path, id: &str) -> Vec<Value> {
    let out = fune()
        .arg("-C")
        .arg(root)
        .args(["--json", "analyze", id])
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    v["missing"].as_array().cloned().unwrap_or_default()
}

/// 不足入力のうち、パスが `path` のもの。
fn find<'a>(missing: &'a [Value], path: &str) -> Option<&'a Value> {
    missing.iter().find(|m| m["path"] == path)
}

/// 直しのコマンドを（置き換え欄を埋めて）打つ。`fune` を、同じプロジェクトを指す今のバイナリに置き換えてシェルで動かす。
fn run_fix(root: &Path, command: &str, replace: &[(&str, &str)]) {
    let mut text = command.to_owned();
    for (from, to) in replace {
        text = text.replace(from, to);
    }
    let bin = assert_cmd::cargo::cargo_bin("fune");
    let rest = text.strip_prefix("fune ").expect("a fune command");
    let line = format!("\"{}\" -C \"{}\" {rest}", bin.display(), root.display());
    let status = std::process::Command::new("sh")
        .args(["-c", &line])
        .status()
        .unwrap();
    assert!(status.success(), "{line}");
}

/// 部品の値: カタログ品の値を消して出る不足に、部品の上書きのコマンドを打つと、その部品の不足が消える。
#[test]
fn value_fix_removes_the_missing_input() {
    let (_tmp, root) = demo_copy();
    fune()
        .arg("-C")
        .arg(&root)
        .args(["catalog", "unset", "local/rw_10mNms", "max_momentum"])
        .assert()
        .success();
    let before = missing(&root, "aocs.rw_unloading");
    let m = find(&before, "RW_X.max_momentum").expect("missing");
    let fix = &m["fixes"][0];
    assert_eq!(fix["kind"], "set_value");
    run_fix(
        &root,
        fix["command"].as_str().unwrap(),
        &[("<value>", "10 mN*m*s")],
    );
    let after = missing(&root, "aocs.rw_unloading");
    assert!(find(&after, "RW_X.max_momentum").is_none());
    // 他の車輪はまだ不足のまま。品の側へ書く代わりのコマンドで、残りも消える
    let catalog = &find(&after, "RW_Y.max_momentum").unwrap()["fixes"][0]["catalog"];
    run_fix(
        &root,
        catalog["command"].as_str().unwrap(),
        &[("<value>", "10 mN*m*s")],
    );
    assert!(missing(&root, "aocs.rw_unloading").is_empty());
}

/// 部品の参照: スラスタの無いマヌーバの不足に、候補の部品を入れたコマンドを打つと不足が消える。
#[test]
fn reference_fix_removes_the_missing_input() {
    let (_tmp, root) = demo_copy();
    fune()
        .arg("-C")
        .arg(&root)
        .args([
            "maneuver",
            "set",
            "RAISE",
            "--category",
            "insertion",
            "--method",
            "propagated",
            "--altitude",
            "600 km",
        ])
        .assert()
        .success();
    let before = missing(&root, "propulsion.maneuvers");
    let m = find(&before, "maneuvers.RAISE.thruster").expect("missing");
    let fix = &m["fixes"][0];
    assert_eq!(fix["kind"], "pick_part");
    let part = fix["parts"][0].as_str().unwrap();
    run_fix(&root, fix["command"].as_str().unwrap(), &[("<part>", part)]);
    assert!(
        find(
            &missing(&root, "propulsion.maneuvers"),
            "maneuvers.RAISE.thruster"
        )
        .is_none()
    );
}

/// 人が読む表示にも、同じコマンドが出る。
#[test]
fn human_output_lists_the_commands() {
    let (_tmp, root) = demo_copy();
    fune()
        .arg("-C")
        .arg(&root)
        .args(["catalog", "unset", "local/rw_10mNms", "max_momentum"])
        .assert()
        .success();
    let out = fune()
        .arg("-C")
        .arg(&root)
        .args(["analyze", "aocs.rw_unloading"])
        .output()
        .unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains(r#"fune set RW_X.max_momentum "<value>""#),
        "{text}"
    );
    assert!(
        text.contains("fune catalog set local/rw_10mNms max_momentum"),
        "{text}"
    );
}
