//! 編集のコマンドの共通部分: Op の列を `fune_service::edit::apply` に渡し、結果（変わったファイル・差分・診断）を表示する。
//! 名詞ごとの引数 → Op の対応は `nouns`。

use crate::args::EditOpts;
use crate::diag;
use crate::output::Out;
use fune_service::dto::{ApplyRequest, ApplyResult, FileChangeKind, HistoryAction, Op};
use fune_service::edit;
use fune_service::{Workspace, edit::apply};
use std::path::Path;

/// `fune init <name>`
pub fn init(dir: &Path, name: &str, out: &Out) -> anyhow::Result<u8> {
    let r = edit::init(dir, name)?;
    if out.json {
        out.json(&r)?;
        return Ok(0);
    }
    out.ok(format!(
        "Created project {}: {}",
        r.project.name, r.project.root
    ));
    for f in &r.created {
        println!("  {}", out.dim(f));
    }
    Ok(0)
}

/// Op の列を適用して結果を表示する。
pub fn run_ops(ws: &Workspace, ops: Vec<Op>, e: &EditOpts, out: &Out) -> anyhow::Result<u8> {
    let req = ApplyRequest {
        ops,
        base_revision: e.base_revision.clone(),
        dry_run: e.dry_run,
    };
    let r = apply(ws, &req)?;
    print_apply(&r, out)?;
    Ok(0)
}

/// `fune history`（古い順。取り消し済みの編集には `undone` を付ける）。
pub fn history(ws: &Workspace, out: &Out) -> anyhow::Result<u8> {
    let h = fune_service::history::view(ws)?;
    if out.json {
        out.json(&h)?;
        return Ok(0);
    }
    if h.entries.is_empty() {
        println!("No edits in the history");
        return Ok(0);
    }
    for (i, e) in h.entries.iter().enumerate() {
        let mark = if e.undone { "undone" } else { "      " };
        let what = e.cli.first().map_or("(no command)", String::as_str);
        println!(
            "{:>4}  {}  {what}  {}",
            i + 1,
            out.dim(mark),
            out.dim(format!("{} file(s)", e.files.len()))
        );
    }
    println!(
        "{}",
        out.dim(format!(
            "(keeps the last {} edits; `fune undo` / `fune redo` move along this list)",
            h.limit
        ))
    );
    Ok(0)
}

/// `fune undo` / `fune redo`（`redo` が true ならやり直し）。
pub fn step(ws: &Workspace, redo: bool, out: &Out) -> anyhow::Result<u8> {
    let r = if redo {
        fune_service::history::redo(ws)?
    } else {
        fune_service::history::undo(ws)?
    };
    if out.json {
        out.json(&r)?;
        return Ok(0);
    }
    let head = match r.action {
        HistoryAction::Undo => "Undid",
        HistoryAction::Redo => "Redid",
    };
    for f in &r.changed {
        println!("  {} {}", out.dim(mark(f.change)), file_label(f));
    }
    let what = r.edit_cli.first().map_or("the last edit", String::as_str);
    out.ok(format!(
        "{head}: {what}  {}",
        out.dim(format!("rev {}", short(&r.revision)))
    ));
    Ok(0)
}

fn mark(change: FileChangeKind) -> &'static str {
    match change {
        FileChangeKind::Created => "created",
        FileChangeKind::Modified => "modified",
        FileChangeKind::Deleted => "deleted",
        FileChangeKind::Renamed => "moved",
    }
}

fn file_label(f: &fune_service::dto::ChangedFile) -> String {
    match &f.from {
        Some(p) => format!("{p} → {}", f.path),
        None => f.path.clone(),
    }
}

fn print_apply(r: &ApplyResult, out: &Out) -> anyhow::Result<()> {
    if out.json {
        return out.json(r);
    }
    if r.dry_run {
        for f in &r.changed {
            out.diff(&f.diff);
        }
        out.ok(format!(
            "dry-run: {} files would change (nothing written)",
            r.changed.len()
        ));
    } else {
        for f in &r.changed {
            println!("  {} {}", out.dim(mark(f.change)), file_label(f));
        }
        out.ok(format!(
            "Wrote {} files  {}",
            r.changed.len(),
            out.dim(format!("rev {}", short(&r.revision)))
        ));
    }
    if !r.diagnostics.is_empty() {
        eprint!("{}", diag::render(&r.diagnostics, out.color));
    }
    Ok(())
}

fn short(rev: &str) -> &str {
    rev.get(..15).unwrap_or(rev)
}
