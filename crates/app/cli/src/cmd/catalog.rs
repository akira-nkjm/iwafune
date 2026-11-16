//! `fune catalog ls|show`（カタログ品を読む）と `fune catalog fetch|update`（共有カタログの取得と更新）。
//! 編集は `records::catalog`。編集のあとの案内（`fune catalog source add|rm` の次にすること）は [`after_edit`]。

use super::common::Handled;
use crate::args::{CatalogCmd, CatalogSourceCmd, EditOpts};
use crate::output::{C, Out};
use fune_service::dto::{
    CatalogFetchRequest, CatalogItemChangeKind, CatalogSyncResult, CatalogUpdateRequest,
};
use fune_service::{Workspace, query, shared_catalog};

/// commit を表示するときの桁数。
const SHORT_COMMIT: usize = 12;

/// `fune catalog ls|show|fetch|update`
pub fn read(ws: &Workspace, cmd: &CatalogCmd, out: &Out) -> Handled {
    let result = match cmd {
        CatalogCmd::Fetch { locked } => Some(shared_catalog::fetch(
            ws,
            &CatalogFetchRequest { locked: *locked },
        )?),
        CatalogCmd::Update { name } => Some(shared_catalog::update(
            ws,
            &CatalogUpdateRequest { name: name.clone() },
        )?),
        _ => None,
    };
    if let Some(result) = result {
        show_sync(&result, out)?;
        return Ok(Some(0));
    }
    if let CatalogCmd::Show { id } = cmd {
        let item = query::catalog_item(ws, id)?;
        if out.json {
            out.json(&item)?;
            return Ok(Some(0));
        }
        out.heading(&item.id);
        let mut t = out.table(&["Attribute", "Value"]);
        for (attr, v) in &item.values {
            let value = super::common::value_text(v.display.as_deref());
            t.row(vec![C::Text(attr.clone()), C::Text(value)]);
        }
        t.print();
        return Ok(Some(0));
    }
    let CatalogCmd::Ls = cmd else {
        return Ok(None);
    };
    let items = query::design_view(ws)?.catalog;
    if out.json {
        out.json(&items)?;
        return Ok(Some(0));
    }
    let mut t = out
        .table(&["ID", "Kind", "Name", "Model", "Attributes"])
        .numeric(&[4]);
    for i in &items {
        t.row(vec![
            C::Strong(i.id.clone()),
            C::Text(i.kind.clone()),
            C::Text(i.label.clone().unwrap_or_default()),
            C::Dim(i.part_number.clone().unwrap_or_default()),
            C::Num(i.values.len().to_string()),
        ]);
    }
    t.print();
    Ok(Some(0))
}

/// 取得・更新の結果（共有カタログごとの commit と、変わった品と値）。
fn show_sync(r: &CatalogSyncResult, out: &Out) -> anyhow::Result<()> {
    if out.json {
        return out.json(r);
    }
    if r.catalogs.is_empty() {
        println!("{}", out.dim("No shared catalogs ([[catalog]] git = …)"));
        return Ok(());
    }
    let short = |c: &str| c.get(..SHORT_COMMIT).unwrap_or(c).to_owned();
    let mut t = out.table(&["Catalog", "git", "rev", "commit", "Previous commit"]);
    for c in &r.catalogs {
        let previous = match c.previous.as_deref() {
            Some(p) if p == c.commit => C::Dim("(same)".into()),
            Some(p) => C::Text(short(p)),
            None => C::Dim("—".into()),
        };
        t.row(vec![
            C::Strong(c.name.clone()),
            C::Dim(c.git.clone()),
            C::Text(c.rev.clone().unwrap_or_default()),
            C::Text(short(&c.commit)),
            previous,
        ]);
    }
    t.print();
    for item in &r.items {
        let mark = match item.change {
            CatalogItemChangeKind::Added => "+",
            CatalogItemChangeKind::Removed => "-",
            CatalogItemChangeKind::Changed => "~",
        };
        println!("{mark} {}", item.id);
        for v in &item.values {
            let text = |s: &Option<String>| s.clone().unwrap_or_else(|| "—".into());
            println!("    {}: {} → {}", v.key, text(&v.before), text(&v.after));
        }
    }
    if r.lock_changed {
        out.ok("Rewrote catalog.lock (commit it)");
    } else {
        out.ok("catalog.lock is unchanged");
    }
    Ok(())
}

/// 共有カタログを足した・外したあとの案内（足すだけでは取得しない。外しても lock の記録は次の fetch まで残る）。
/// `--json` と `--dry-run` では出さない。
pub fn after_edit(cmd: &CatalogCmd, opts: &EditOpts, out: &Out) {
    let CatalogCmd::Source(source) = cmd else {
        return;
    };
    if out.json || opts.dry_run {
        return;
    }
    let text = match source {
        CatalogSourceCmd::Add { name, .. } => format!(
            "Added shared catalog `{name}` to fune.toml (not fetched yet). Run `fune catalog fetch`, then commit catalog.lock"
        ),
        CatalogSourceCmd::Rm { name, .. } => {
            format!(
                "Any `{name}` entry in catalog.lock is dropped by the next `fune catalog fetch`"
            )
        }
    };
    println!("{}", out.dim(text));
}
