//! `fune report export`: サービスが返したファイルの内容を `--out` に書く（書き出しは派生のファイルで、設計ファイルは変えない）。

use crate::args::{ReportCmd, ReportExportArgs};
use crate::output::{C, Out};
use anyhow::Context;
use fune_service::dto::{ReportExport, ReportRequest};
use fune_service::{Workspace, report};
use std::path::{Path, PathBuf};

/// `fune report …`
pub fn run(ws: &Workspace, cmd: &ReportCmd, out: &Out) -> anyhow::Result<u8> {
    match cmd {
        ReportCmd::Export(a) => export(ws, a, out),
    }
}

fn export(ws: &Workspace, a: &ReportExportArgs, out: &Out) -> anyhow::Result<u8> {
    let items = a
        .items
        .iter()
        .map(|i| report::parse_item(i))
        .collect::<Result<Vec<_>, _>>()?;
    let result = report::export(
        ws,
        &ReportRequest {
            items,
            cases: a.cases.clone(),
            lang: report::parse_lang(&a.lang)?,
        },
    )?;
    let dir = out_dir(ws, a.out.as_deref());
    write_files(&dir, &result)?;
    if out.json {
        let paths: Vec<&str> = result.files.iter().map(|f| f.path.as_str()).collect();
        out.json(&serde_json::json!({
            "out": dir,
            "revision": result.revision,
            "files": paths,
            "unavailable": result.unavailable,
        }))?;
        return Ok(0);
    }
    out.ok(format!(
        "Wrote {} files to {} (revision {})",
        result.files.len(),
        dir.display(),
        result.revision
    ));
    print_unavailable(&result, out);
    Ok(0)
}

/// 出力先（`--out` の相対パスは今いる場所から、無ければプロジェクトの根の既定の場所）。
fn out_dir(ws: &Workspace, given: Option<&Path>) -> PathBuf {
    given.map_or_else(|| ws.root().join(report::DEFAULT_DIR), Path::to_path_buf)
}

/// ファイルを書く。前回の書き出しの図（`figs/*.svg`）のうち今回無いものは消す（古い図を残さない）。
fn write_files(dir: &Path, result: &ReportExport) -> anyhow::Result<()> {
    for f in &result.files {
        let path = dir.join(&f.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Cannot create {}", parent.display()))?;
        }
        std::fs::write(&path, &f.content)
            .with_context(|| format!("Cannot write {}", path.display()))?;
    }
    remove_stale_figures(dir, result)
}

fn remove_stale_figures(dir: &Path, result: &ReportExport) -> anyhow::Result<()> {
    let figs = dir.join("figs");
    let Ok(entries) = std::fs::read_dir(&figs) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let rel = format!("figs/{}", entry.file_name().to_string_lossy());
        let svg = path.extension().is_some_and(|e| e == "svg");
        if svg && !result.files.iter().any(|f| f.path == rel) {
            std::fs::remove_file(&path)
                .with_context(|| format!("Cannot remove {}", path.display()))?;
        }
    }
    Ok(())
}

fn print_unavailable(result: &ReportExport, out: &Out) {
    if result.unavailable.is_empty() {
        return;
    }
    let mut t = out.table(&["Item", "Case", "Not available because"]);
    for u in &result.unavailable {
        t.row(vec![
            C::Strong(u.item.as_str().to_owned()),
            C::Text(u.case.clone().unwrap_or_default()),
            C::Text(u.reason.clone()),
        ]);
    }
    t.print();
}
