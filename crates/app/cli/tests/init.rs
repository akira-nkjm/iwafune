//! `fune init [DIR] [--name NAME]`: 作る場所（ディレクトリ）と名前の決め方、既存のディレクトリ、`-C`、`--json`。

mod common;

use common::{fune, snapshot_files};
use std::path::Path;

/// 雛形のファイル（相対パス → 本文）。名前以外は、ディレクトリを作る形になる前の `fune init <名前>` と同じ（バイト単位）。
fn expected_files(name: &str) -> Vec<(&'static str, String)> {
    vec![
        (
            ".gitignore",
            ".fune/\n".to_owned(),
        ),
        (
            "budget/allocation.toml",
            "[mass]\nsystem_margin = 0.1\n\n[mass.alloc]\n".to_owned(),
        ),
        (
            "budget/policy.toml",
            "[growth]\noff_the_shelf = 0.05\nmodified      = 0.1\nnew           = 0.2\n\n[uncertainty]\noff_the_shelf = 0.03\nmodified      = 0.1\nnew           = 0.2\n\n[harness]\nallowance = 0.05\n".to_owned(),
        ),
        (
            "fune.toml",
            format!(
                "[project]\nname   = \"{name}\"\nschema = 2\ntoml   = \"1.0\"\n\n[[catalog]]\nname = \"local\"\npath = \"catalog\"\n"
            ),
        ),
        (
            "modes/SAFE.toml",
            "description = \"Run only the minimum equipment\"\n\n[states]\n".to_owned(),
        ),
    ]
}

/// `root` の中身が `name` の雛形とバイト単位で同じことを確かめる。
fn assert_project(root: &Path, name: &str) {
    let got = snapshot_files(root);
    let want = expected_files(name);
    let keys: Vec<&str> = got.keys().map(String::as_str).collect();
    let want_keys: Vec<&str> = want.iter().map(|(p, _)| *p).collect();
    assert_eq!(keys, want_keys);
    for (path, text) in want {
        assert_eq!(got[path], text.as_bytes(), "{path}");
    }
}

/// `cwd` で `fune <args>` を動かす。
fn run(cwd: &Path, args: &[&str]) -> std::process::Output {
    fune().current_dir(cwd).args(args).output().unwrap()
}

fn stdout(o: &std::process::Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &std::process::Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

#[test]
fn name_creates_a_directory_of_that_name() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["init", "sample"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_project(&tmp.path().join("sample"), "sample");
    // 次に打つコマンドを出す
    let text = stdout(&out);
    assert!(text.contains("cd sample"), "{text}");
    assert!(text.contains("fune status"), "{text}");
    // 作ったプロジェクトは check と fmt --check を通る
    let root = tmp.path().join("sample");
    assert_eq!(run(&root, &["check"]).status.code(), Some(0));
    assert_eq!(run(&root, &["fmt", "--check"]).status.code(), Some(0));
}

#[test]
fn without_arguments_uses_the_current_directory_name() {
    let tmp = tempfile::tempdir().unwrap();
    let cur = tmp.path().join("cur_sat");
    std::fs::create_dir(&cur).unwrap();
    let out = run(&cur, &["init"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_project(&cur, "cur_sat");
    assert!(!stdout(&out).contains("cd "), "{}", stdout(&out));
}

#[test]
fn name_option_alone_keeps_the_current_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["init", "--name", "mysat"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_project(tmp.path(), "mysat");
}

#[test]
fn nested_path_creates_parents_and_names_after_the_last_part() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["init", "path/to/sample"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_project(&tmp.path().join("path/to/sample"), "sample");
}

#[test]
fn directory_and_name_can_differ() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["init", "sample", "--name", "mysat"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_project(&tmp.path().join("sample"), "mysat");
}

/// 名前を `--name` で渡したときのファイルは、ディレクトリ名から決めたときと同じ（バイト単位）。
#[test]
fn explicit_name_and_derived_name_write_the_same_bytes() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["init", "alpha"]).status.success());
    std::fs::create_dir(tmp.path().join("beta")).unwrap();
    let beta = tmp.path().join("beta");
    assert!(run(&beta, &["init", "--name", "alpha"]).status.success());
    assert_eq!(
        snapshot_files(&tmp.path().join("alpha")),
        snapshot_files(&beta)
    );
}

#[test]
fn dash_c_is_the_base_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let out = fune()
        .arg("-C")
        .arg(tmp.path().join("work"))
        .args(["init", "sample"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    assert_project(&tmp.path().join("work/sample"), "sample");
    assert!(stdout(&out).contains("work/sample"), "{}", stdout(&out));
    // DIR なしなら `-C` のディレクトリそのもの（名前はその名前）
    let out = fune()
        .arg("-C")
        .arg(tmp.path().join("other"))
        .arg("init")
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    assert_project(&tmp.path().join("other"), "other");
}

#[test]
fn existing_directory_is_used_and_existing_files_are_kept() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("sample");
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(dir.join("notes.txt"), "mine").unwrap();
    std::fs::write(dir.join(".gitignore"), "target/\n").unwrap();
    let out = run(tmp.path(), &["init", "sample"]);
    assert!(out.status.success(), "{}", stderr(&out));
    // 他のファイルは触らない。既にある雛形のファイル（`.gitignore`）は上書きしない
    assert_eq!(
        std::fs::read_to_string(dir.join("notes.txt")).unwrap(),
        "mine"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join(".gitignore")).unwrap(),
        "target/\n"
    );
    assert!(dir.join("fune.toml").exists());
}

#[test]
fn existing_project_is_refused_and_left_alone() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["init", "sample"]).status.success());
    let before = snapshot_files(&tmp.path().join("sample"));
    let out = run(tmp.path(), &["init", "sample", "--name", "other"]);
    assert_eq!(out.status.code(), Some(3), "{}", stderr(&out));
    assert!(stderr(&out).contains("already exists"), "{}", stderr(&out));
    assert_eq!(snapshot_files(&tmp.path().join("sample")), before);
}

#[test]
fn unusable_directory_name_asks_for_name_and_creates_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["init", "a/b\"c"]);
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    assert!(stderr(&out).contains("--name"), "{}", stderr(&out));
    assert!(
        !tmp.path().join("a").exists(),
        "a half-made directory remains"
    );
    // 現在のディレクトリの名前が使えないとき
    let bad = tmp.path().join("bad\"dir");
    std::fs::create_dir(&bad).unwrap();
    let out = run(&bad, &["init"]);
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    assert!(stderr(&out).contains("--name"), "{}", stderr(&out));
    assert!(snapshot_files(&bad).is_empty());
    // `--name` を付ければ作れる
    assert!(run(&bad, &["init", "--name", "ok"]).status.success());
    assert_project(&bad, "ok");
}

#[test]
fn json_output_names_the_project_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["--json", "init", "sample"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["project"]["name"], "sample");
    let root = std::path::PathBuf::from(v["project"]["root"].as_str().unwrap());
    assert_eq!(root.file_name().unwrap(), "sample");
    assert_eq!(
        root.canonicalize().unwrap(),
        tmp.path().join("sample").canonicalize().unwrap()
    );
    assert_eq!(v["created"][0], "fune.toml");
}
