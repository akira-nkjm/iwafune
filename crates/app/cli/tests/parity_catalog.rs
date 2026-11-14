//! パリティテスト（CI 必須）: 共有カタログの取得と更新。`fune catalog fetch|update` と `POST /catalog/fetch|update` は
//! 同じ service の関数なので、書かれる catalog.lock はバイト単位で一致し、結果（commit・変わった品と値）も同じ。
//! 共有カタログは一時ディレクトリに作った git のリポジトリ（`file://`。ネットワークは使わない）。

mod common;

use common::{assert_same_files, demo_copy, fune, post_json};
use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=test", "-c", "user.email=test@example.com"])
        .args([
            "-c",
            "init.defaultBranch=main",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

const ITEM: &str = "kind = \"reaction_wheel\"\nlabel = \"RW\"\nsource = \"ds_rw\"\n\n[attr]\nmass = { value = 150.0, unit = \"g\" }\n";

/// 品 1 つと出典 1 つの共有カタログ。
fn catalog_repo(dir: &Path) {
    std::fs::create_dir_all(dir.join("catalog")).unwrap();
    std::fs::create_dir_all(dir.join("sources")).unwrap();
    std::fs::write(dir.join("catalog/rw.toml"), ITEM).unwrap();
    std::fs::write(
        dir.join("sources/ds_rw.toml"),
        "kind = \"datasheet\"\nref = \"https://example.com/rw.pdf\"\n",
    )
    .unwrap();
    git(dir, &["init", "-q"]);
    git(dir, &["add", "."]);
    git(dir, &["commit", "-q", "-m", "first"]);
}

/// デモの写しの fune.toml に共有カタログ `lab` を足す。
fn add_shared(root: &Path, repo: &Path) {
    let toml = root.join("fune.toml");
    let mut text = std::fs::read_to_string(&toml).unwrap();
    text.push_str(&format!(
        "\n[[catalog]]\nname = \"lab\"\ngit  = \"file://{}\"\nrev  = \"main\"\n",
        repo.display()
    ));
    std::fs::write(&toml, text).unwrap();
}

/// `fune --json -C <root> catalog <args>` の結果の JSON。
fn cli_json(root: &Path, args: &[&str]) -> Value {
    let out = fune()
        .arg("--json")
        .arg("-C")
        .arg(root)
        .arg("catalog")
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "fune catalog {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn shared_catalog_fetch_and_update_match_cli() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("catalog-repo");
    catalog_repo(&repo);
    let (_t1, cli_root) = demo_copy();
    let (_t2, http_root) = demo_copy();
    for root in [&cli_root, &http_root] {
        add_shared(root, &repo);
    }
    let cli = cli_json(&cli_root, &["fetch"]);
    let http = post_json(&rt, &http_root, "/api/v1/catalog/fetch", json!({}));
    assert_eq!(cli, http);
    assert_eq!(cli["lock_changed"], true);
    assert_same_files(&cli_root, &http_root, "fetch");
    assert!(cli_root.join("catalog.lock").is_file());

    std::fs::write(repo.join("catalog/rw.toml"), ITEM.replace("150.0", "160.0")).unwrap();
    git(&repo, &["commit", "-q", "-am", "heavier"]);
    let cli = cli_json(&cli_root, &["update", "lab"]);
    let http = post_json(
        &rt,
        &http_root,
        "/api/v1/catalog/update",
        json!({ "name": "lab" }),
    );
    assert_eq!(cli, http);
    assert_eq!(cli["items"][0]["id"], "lab/rw");
    assert_eq!(cli["items"][0]["values"][0]["after"], "160 g");
    assert_same_files(&cli_root, &http_root, "update");

    // lock に合っていれば --locked でも通る（CI の手順）
    let cli = cli_json(&cli_root, &["fetch", "--locked"]);
    let http = post_json(
        &rt,
        &http_root,
        "/api/v1/catalog/fetch",
        json!({ "locked": true }),
    );
    assert_eq!(cli, http);
    assert_eq!(cli["lock_changed"], false);
}
