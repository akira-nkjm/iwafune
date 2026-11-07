//! 編集のコマンドの共通部分: Op の列を `fune_service::edit::apply` に渡し、結果（変わったファイル・差分・診断）を表示する。
//! 名詞ごとの引数 → Op の対応は `nouns`。

use crate::args::EditOpts;
use crate::diag;
use crate::output::Out;
use fune_service::dto::{ApplyRequest, ApplyResult, FileChangeKind, Op};
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
        "プロジェクト {} を作りました: {}",
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

fn print_apply(r: &ApplyResult, out: &Out) -> anyhow::Result<()> {
    if out.json {
        return out.json(r);
    }
    if r.dry_run {
        for f in &r.changed {
            out.diff(&f.diff);
        }
        out.ok(format!(
            "dry-run: {} ファイルが変わります（書き込んでいません）",
            r.changed.len()
        ));
    } else {
        for f in &r.changed {
            let mark = match f.change {
                FileChangeKind::Created => "作成",
                FileChangeKind::Modified => "変更",
                FileChangeKind::Deleted => "削除",
                FileChangeKind::Renamed => "移動",
            };
            let from = f
                .from
                .as_ref()
                .map(|p| format!("{p} → "))
                .unwrap_or_default();
            println!("  {} {from}{}", out.dim(mark), f.path);
        }
        out.ok(format!(
            "{} ファイルを書き込みました  {}",
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
