//! パリティテスト（CI 必須）: 設計書の表と図の書き出し。CLI（`fune report export --out`）が書いたファイルと、
//! HTTP（`POST /api/v1/report/export`）が返したファイルの内容が、パスも本文もバイト単位で一致する。
//! 書き出しは設計ファイルを変えない（プロジェクトの中に `exports/` を作るのは既定の出力先のときだけ）。

mod common;

use common::{demo_copy, fune, post_json, run_cli, snapshot_files};
use serde_json::{Value, json};
use std::path::Path;

fn assert_same(cli_out: &Path, http: &Value, what: &str) {
    let on_disk = snapshot_files(cli_out);
    let files = http["files"].as_array().unwrap();
    let mut paths: Vec<&str> = files.iter().map(|f| f["path"].as_str().unwrap()).collect();
    let mut disk: Vec<&str> = on_disk.keys().map(String::as_str).collect();
    paths.sort_unstable();
    disk.sort_unstable();
    assert_eq!(paths, disk, "the set of files differs ({what})");
    for f in files {
        let path = f["path"].as_str().unwrap();
        assert_eq!(
            on_disk[path],
            f["content"].as_str().unwrap().as_bytes(),
            "{path} differs ({what})"
        );
    }
}

#[test]
fn report_export_matches_http() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_t, root) = demo_copy();
    let out = tempfile::tempdir().unwrap();
    let cases: [(&[&str], Value); 3] = [
        (&[], json!({})),
        (
            &[
                "--case",
                "cold_safe",
                "--item",
                "thermal-result",
                "--item",
                "thermal-history",
                "--lang",
                "ja",
            ],
            json!({ "cases": ["cold_safe"], "items": ["thermal-result", "thermal-history"], "lang": "ja" }),
        ),
        (
            &["--item", "link-budget", "--item", "mass"],
            json!({ "items": ["link-budget", "mass"] }),
        ),
    ];
    for (i, (args, body)) in cases.iter().enumerate() {
        let dir = out.path().join(format!("run{i}"));
        let dir_text = dir.to_str().unwrap();
        let mut full = vec!["report", "export", "--out", dir_text];
        full.extend(args.iter());
        run_cli(&root, &full);
        let http = post_json(&rt, &root, "/api/v1/report/export", body.clone());
        assert_same(&dir, &http, &format!("{args:?}"));
    }
}

#[test]
fn default_output_goes_under_exports_report_and_the_design_is_untouched() {
    let (_t, root) = demo_copy();
    let before = snapshot_files(&root);
    run_cli(&root, &["report", "export", "--item", "mass"]);
    let after = snapshot_files(&root);
    let new: Vec<&String> = after.keys().filter(|k| !before.contains_key(*k)).collect();
    assert!(
        new.iter().all(|k| k.starts_with("exports/report/")),
        "{new:?}"
    );
    assert!(new.iter().any(|k| *k == "exports/report/data.json"));
    for (path, bytes) in &before {
        assert_eq!(after.get(path), Some(bytes), "{path} changed");
    }
    // 書き出したものがあっても、設計の検査と整形は通る
    run_cli(&root, &["check"]);
    run_cli(&root, &["fmt", "--check"]);
}

#[test]
fn rewriting_removes_stale_figures() {
    let (_t, root) = demo_copy();
    let out = tempfile::tempdir().unwrap();
    let dir = out.path().to_str().unwrap();
    run_cli(
        &root,
        &["report", "export", "--out", dir, "--case", "cold_safe"],
    );
    assert!(
        out.path()
            .join("figs/thermal-history-cold_safe.svg")
            .exists()
    );
    run_cli(
        &root,
        &["report", "export", "--out", dir, "--case", "hot_imaging"],
    );
    assert!(
        !out.path()
            .join("figs/thermal-history-cold_safe.svg")
            .exists()
    );
    assert!(
        out.path()
            .join("figs/thermal-history-hot_imaging.svg")
            .exists()
    );
}

#[test]
fn bad_arguments_are_refused() {
    let (_t, root) = demo_copy();
    for args in [
        &["report", "export", "--item", "pdf"][..],
        &["report", "export", "--case", "no_such_case"][..],
        &["report", "export", "--lang", "fr"][..],
    ] {
        let out = fune().arg("-C").arg(&root).args(args).output().unwrap();
        assert!(!out.status.success(), "{args:?} should fail");
    }
}
