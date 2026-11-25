//! CLI のテストの共通部品。

#![allow(dead_code)]

use assert_cmd::Command;
use std::path::{Path, PathBuf};

/// 色なし・固定幅で `fune` を起動するコマンド。
pub fn fune() -> Command {
    let mut c = Command::cargo_bin("fune").unwrap();
    c.env("NO_COLOR", "1")
        .env("COLUMNS", "200")
        .arg("--color")
        .arg("never");
    c
}

/// fixtures/demo-6u の場所。
pub fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")
}

/// ディレクトリを丸ごと複製する。
pub fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// デモの設計データの一時コピー（`demo-6u` というディレクトリ名で置く）。
pub fn demo_copy() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("demo-6u");
    copy_dir(&fixture(), &root);
    (tmp, root)
}

/// ディレクトリの中のすべてのファイル（相対パス → 中身）。
pub fn snapshot_files(root: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    let mut out = std::collections::BTreeMap::new();
    collect(root, root, &mut out);
    out
}

fn collect(root: &Path, dir: &Path, out: &mut std::collections::BTreeMap<String, Vec<u8>>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let rel = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if rel.starts_with(".fune") {
            continue;
        }
        if path.is_dir() {
            collect(root, &path, out);
        } else {
            out.insert(rel, std::fs::read(&path).unwrap());
        }
    }
}

/// `fune -C <root> <args>` を実行し、成功したことを確かめる（パリティテスト用）。
pub fn run_cli(root: &Path, args: &[&str]) {
    let out = fune().arg("-C").arg(root).args(args).output().unwrap();
    assert!(
        out.status.success(),
        "fune {args:?} failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// `root` を開いた HTTP のルーターに JSON を POST し、200 であることを確かめて本文の JSON を返す（パリティテスト用）。
pub fn post_json(
    rt: &tokio::runtime::Runtime,
    root: &Path,
    uri: &str,
    body: serde_json::Value,
) -> serde_json::Value {
    let (status, text) = post_json_response(rt, root, uri, body);
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "POST {uri} failed: {text}"
    );
    text
}

/// HTTP の JSON と状態を返す（エラーを確かめるテストでも使う）。
pub fn post_json_response(
    rt: &tokio::runtime::Runtime,
    root: &Path,
    uri: &str,
    body: serde_json::Value,
) -> (axum::http::StatusCode, serde_json::Value) {
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use tower::ServiceExt;
    let ws = fune_service::Workspace::open(root).unwrap();
    let router = fune_server::router(std::sync::Arc::new(ws));
    let req = Request::post(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let (status, text) = rt.block_on(async {
        let res = router.oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        (status, String::from_utf8_lossy(&bytes).into_owned())
    });
    (
        status,
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("POST {uri}: {status}: {text}: {e}")),
    )
}

/// `root` を開いた HTTP のルーターに GET し、200 であることを確かめて本文の JSON を返す（パリティテスト用）。
pub fn get_json(rt: &tokio::runtime::Runtime, root: &Path, uri: &str) -> serde_json::Value {
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;
    let ws = fune_service::Workspace::open(root).unwrap();
    let router = fune_server::router(std::sync::Arc::new(ws));
    let req = Request::get(uri).body(Body::empty()).unwrap();
    let (status, text) = rt.block_on(async {
        let res = router.oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        (status, String::from_utf8_lossy(&bytes).into_owned())
    });
    assert_eq!(status, StatusCode::OK, "GET {uri} failed: {text}");
    serde_json::from_str(&text).unwrap()
}

/// 2 つのプロジェクトのファイルの集合と中身がバイト単位で一致することを確かめる（パリティテスト用）。
pub fn assert_same_files(cli: &Path, http: &Path, after: &str) {
    let a = snapshot_files(cli);
    let b = snapshot_files(http);
    let paths_a: Vec<&String> = a.keys().collect();
    let paths_b: Vec<&String> = b.keys().collect();
    assert_eq!(paths_a, paths_b, "the set of files differs after {after}");
    for (path, bytes) in &a {
        assert!(
            b.get(path) == Some(bytes),
            "{path} differs after {after}\n--- CLI\n{}\n--- HTTP\n{}",
            String::from_utf8_lossy(bytes),
            String::from_utf8_lossy(&b[path])
        );
    }
}

/// 成功した CLI の JSON を読む。
pub fn cli_json(root: &Path, args: &[&str]) -> serde_json::Value {
    let out = fune()
        .arg("-C")
        .arg(root)
        .arg("--json")
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{args:?}: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

/// 判定に違反があっても（終了コード 1）CLI の JSON を読む。形態や入力を確かめるテストで、判定そのものは見ないときに使う。
pub fn cli_json_any_verdict(root: &Path, args: &[&str]) -> serde_json::Value {
    let out = fune()
        .arg("-C")
        .arg(root)
        .arg("--json")
        .args(args)
        .output()
        .unwrap();
    assert!(
        matches!(out.status.code(), Some(0 | 1)),
        "{args:?}: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}
