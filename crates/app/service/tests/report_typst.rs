//! 書き出したファイルが Typst でそのまま組めること（`typst` が PATH に無ければ何もしない）。
//! 例の文書と、別のディレクトリの文書から `#import` する場合（`data.json` と図の相対パスの解決）を確かめる。

mod common_report;

use common_report::{demo, export, write_all};
use fune_service::dto::ReportRequest;
use std::process::Command;

fn typst_available() -> bool {
    Command::new("typst")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn compile(
    dir: &std::path::Path,
    root: Option<&std::path::Path>,
    input: &str,
    output: &str,
) -> String {
    let mut command = Command::new("typst");
    command.current_dir(dir).arg("compile");
    if let Some(root) = root {
        command.arg("--root").arg(root);
    }
    let out = command.args([input, output]).output().unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(out.status.success(), "typst compile failed:\n{stderr}");
    stderr
}

#[test]
fn exported_files_compile_with_typst() {
    if !typst_available() {
        eprintln!("typst is not installed; skipping");
        return;
    }
    let r = export(
        &demo(),
        &ReportRequest {
            cases: vec!["hot_imaging".into(), "cold_safe".into()],
            ..ReportRequest::default()
        },
    );
    let tmp = tempfile::tempdir().unwrap();
    let report = tmp.path().join("report");
    write_all(&r, &report);
    let warnings = compile(&report, None, "example.typ", "example.pdf");
    assert!(warnings.trim().is_empty(), "typst warnings:\n{warnings}");
    assert!(report.join("example.pdf").metadata().unwrap().len() > 10_000);
    // 別の場所にある文書から取り込む（パスは iwafune.typ のあるディレクトリから解決される）
    let doc = tmp.path().join("doc");
    std::fs::create_dir_all(&doc).unwrap();
    std::fs::write(
        doc.join("main.typ"),
        "#import \"../report/iwafune.typ\": *\n\
         Mass #value(\"part.RW_X.mass\") of #value(\"budget.mass.total\", unit: false).\n\
         #link-budget(link: \"uhf_down@TOKYO_GS\")\n\
         #thermal-history(case: \"cold_safe\")\n\
         #power-history(case: \"hot_imaging\")\n",
    )
    .unwrap();
    let warnings = compile(&doc, Some(tmp.path()), "main.typ", "main.pdf");
    assert!(warnings.trim().is_empty(), "typst warnings:\n{warnings}");
}

#[test]
fn asking_for_a_case_that_was_not_exported_fails_clearly() {
    if !typst_available() {
        eprintln!("typst is not installed; skipping");
        return;
    }
    let r = export(&demo(), &ReportRequest::default());
    let tmp = tempfile::tempdir().unwrap();
    write_all(&r, tmp.path());
    std::fs::write(
        tmp.path().join("bad.typ"),
        "#import \"iwafune.typ\": *\n#thermal-history(case: \"nope\")\n",
    )
    .unwrap();
    let out = Command::new("typst")
        .current_dir(tmp.path())
        .args(["compile", "bad.typ", "bad.pdf"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("nothing exported for thermal-history"));
}
