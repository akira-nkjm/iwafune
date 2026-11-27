//! 編集のコマンドの共通部分: Op の列を `fune_service::edit::apply` に渡し、結果（変わったファイル・差分・診断）を表示する。
//! 名詞ごとの引数 → Op の対応は `nouns`。

use crate::args::EditOpts;
use crate::diag;
use crate::output::Out;
use fune_service::dto::{ApplyRequest, ApplyResult, FileChangeKind, HistoryAction, Op};
use fune_service::edit;
use fune_service::{Workspace, edit::apply};
use std::path::Path;

/// `fune init [DIR] [--name NAME | --demo]`
pub fn init(
    base: &Path,
    dir: Option<&Path>,
    name: Option<&str>,
    demo: bool,
    out: &Out,
) -> anyhow::Result<u8> {
    let r = if demo {
        edit::init_demo(base, dir)?
    } else {
        edit::init(base, dir, name)?
    };
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
    if let Some(next) = dir.map(|d| next_steps(base, d)) {
        println!("Next:");
        println!("  cd {next}");
        println!("  fune status");
        if demo {
            println!("  fune gui");
        }
    } else if demo {
        println!("Next: fune status | fune gui");
    } else {
        println!("Next: fune status");
    }
    Ok(0)
}

/// `fune init DIR` のあとに打つ `cd` の行き先（`-C` で基準が指定されていれば、それを頭に付ける）。
fn next_steps(base: &Path, dir: &Path) -> String {
    base.join(dir)
        .display()
        .to_string()
        .trim_start_matches("./")
        .to_owned()
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
    print_removed(r, out);
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

/// 置き換えで消えるキー（`set` に渡さなかった欄）の知らせ。誤りではないので警告として、書く前（dry run）にも書いた後にも出す。
fn print_removed(r: &ApplyResult, out: &Out) {
    let verb = if r.dry_run {
        "Would remove from"
    } else {
        "Removed from"
    };
    for removed in &r.removed {
        out.warn(format!(
            "{verb} {}: {} (not given to set)",
            removed.path,
            removed.keys.join(", ")
        ));
    }
    if !r.removed.is_empty() && !r.dry_run {
        eprintln!(
            "  {}",
            out.dim("`fune undo` restores them; to change one field and keep the rest, see the --help of the command")
        );
    }
}

fn short(rev: &str) -> &str {
    rev.get(..15).unwrap_or(rev)
}
