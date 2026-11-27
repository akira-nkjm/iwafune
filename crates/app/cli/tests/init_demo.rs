//! `fune init --demo`: 同梱のデモのプロジェクトを作る。

mod common;
use common::{fixture, fune, snapshot_files};
use std::path::Path;

fn run(cwd: &Path, args: &[&str]) -> std::process::Output {
    fune().current_dir(cwd).args(args).output().unwrap()
}

fn stdout(o: &std::process::Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &std::process::Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

/// ディレクトリを作って書き、fixtures/demo-6u とバイト単位で同じで、check と fmt --check を通る。
#[test]
fn demo_creates_the_bundled_project() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["init", "my-demo", "--demo"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let root = tmp.path().join("my-demo");
    assert_eq!(snapshot_files(&root), snapshot_files(&fixture()));
    let text = stdout(&out);
    assert!(text.contains("cd my-demo"), "{text}");
    assert!(text.contains("fune gui"), "{text}");
    assert_eq!(run(&root, &["check"]).status.code(), Some(0));
    assert_eq!(run(&root, &["fmt", "--check"]).status.code(), Some(0));
    assert_eq!(run(&root, &["status"]).status.code(), Some(0));
}

#[test]
fn demo_without_a_directory_uses_the_current_one() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["init", "--demo", "--json"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(json["created"].as_array().is_some_and(|c| !c.is_empty()));
    assert_eq!(snapshot_files(tmp.path()), snapshot_files(&fixture()));
}

#[test]
fn demo_follows_dash_c() {
    let tmp = tempfile::tempdir().unwrap();
    let out = fune()
        .arg("-C")
        .arg(tmp.path().join("work"))
        .args(["init", "--demo"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        snapshot_files(&tmp.path().join("work")),
        snapshot_files(&fixture())
    );
}

#[test]
fn demo_refuses_an_existing_project() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["init", "sample"]).status.success());
    let before = snapshot_files(&tmp.path().join("sample"));
    let out = run(tmp.path(), &["init", "sample", "--demo"]);
    assert_eq!(out.status.code(), Some(3), "{}", stderr(&out));
    assert!(stderr(&out).contains("already exists"), "{}", stderr(&out));
    assert_eq!(snapshot_files(&tmp.path().join("sample")), before);
}

/// デモが書くファイルが 1 つでもあれば、何も書かずに断る（既にあるファイルは変えない）。
#[test]
fn demo_refuses_a_conflicting_file_and_writes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("sample");
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(dir.join(".gitignore"), "target/\n").unwrap();
    let out = run(tmp.path(), &["init", "sample", "--demo"]);
    assert_eq!(out.status.code(), Some(3), "{}", stderr(&out));
    assert!(stderr(&out).contains(".gitignore"), "{}", stderr(&out));
    let after = snapshot_files(&dir);
    assert_eq!(after.len(), 1);
    assert_eq!(after[".gitignore"], b"target/\n");
    // 新しく作ったディレクトリが断られたあとに残らない
    let fresh = tmp.path().join("fresh");
    std::fs::create_dir(&fresh).unwrap();
    std::fs::write(fresh.join("fune.toml"), "x").unwrap();
    assert!(
        !run(tmp.path(), &["init", "fresh", "--demo"])
            .status
            .success()
    );
}

#[test]
fn demo_cannot_be_combined_with_name() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["init", "sample", "--demo", "--name", "x"]);
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    assert!(stderr(&out).contains("--name"), "{}", stderr(&out));
    assert!(!tmp.path().join("sample").exists());
}
