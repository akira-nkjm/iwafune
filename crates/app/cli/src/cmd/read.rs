//! 読むだけのコマンド: status・check・part ls/show・get・mode ls/table・net ls・attr ls・kind ls・setting ls。
//! 名詞のコマンドで読み取りでないもの（`part add` など）は `None` を返し、`edit` に回す。

use crate::args::{AttrCmd, KindCmd, ModeCmd, NetCmd, SequenceCmd, SettingCmd};
use crate::diag;
use crate::output::{C, Out};
use fune_service::dto::{QuantityView, ValueData, ValueQuery};
use fune_service::{Workspace, query};
use std::collections::BTreeMap;

use super::common::{Handled, UNSET, value_text};

/// `fune status`
pub fn status(ws: &Workspace, out: &Out) -> anyhow::Result<u8> {
    let p = query::project_info(ws)?;
    if out.json {
        out.json(&p)?;
        return Ok(0);
    }
    out.heading(&p.name);
    let branch = p.branch.as_deref().unwrap_or("—");
    println!(
        "{}",
        out.dim(format!(
            "{}  branch {branch}  rev {}",
            p.root,
            short(&p.revision)
        ))
    );
    let c = &p.counts;
    println!(
        "parts {}  groups {}  modes {}  nets {}  harnesses {}  cases {}",
        c.parts, c.groups, c.modes, c.nets, c.harnesses, c.cases
    );
    print_counts(out, p.diagnostics.errors, p.diagnostics.warnings);
    Ok(0)
}

/// `fune check`
pub fn check(ws: &Workspace, deny_warnings: bool, out: &Out) -> anyhow::Result<u8> {
    let r = query::check(ws)?;
    let failed = r.counts.errors > 0 || (deny_warnings && r.counts.warnings > 0);
    if out.json {
        out.json(&r)?;
    } else {
        if !r.diagnostics.is_empty() {
            eprint!("{}", diag::render(&r.diagnostics, out.color));
        }
        print_counts(out, r.counts.errors, r.counts.warnings);
    }
    Ok(u8::from(failed))
}

fn print_counts(out: &Out, errors: u32, warnings: u32) {
    match (errors, warnings) {
        (0, 0) => out.ok("Check: no problems"),
        (0, w) => out.warn(format!("Check: {w} warnings")),
        (e, w) => out.fail(format!("Check: {e} errors, {w} warnings")),
    }
}

/// `fune get RW_X.power --state on`
pub fn get(ws: &Workspace, path: &str, state: Option<&str>, out: &Out) -> anyhow::Result<u8> {
    let a = query::get_value(
        ws,
        path,
        &ValueQuery {
            state: state.map(str::to_owned),
        },
    )?;
    if out.json {
        out.json(&a)?;
        return Ok(0);
    }
    let shown = a.selected.as_ref().map_or_else(
        || value_text(a.value.display.as_deref()),
        |q| q.display.clone(),
    );
    println!("{shown}  {}", out.dim(super::common::provenance(&a.value)));
    Ok(0)
}

/// `fune sequence ls`（読み取りだけ。書き込みは `nouns::sequence`）。
pub fn sequence(ws: &Workspace, cmd: &SequenceCmd, out: &Out) -> Handled {
    if !matches!(cmd, SequenceCmd::Ls) {
        return Ok(None);
    }
    let modes = query::design_view(ws)?.modes;
    if out.json {
        out.json(&modes.sequences)?;
        return Ok(Some(0));
    }
    let mut t = out.table(&["Name", "In use", "Anchor", "Segments"]);
    for s in &modes.sequences {
        let anchor = s.anchor.as_ref().map_or_else(String::new, |a| {
            let offset = a
                .offset
                .as_ref()
                .map_or(String::new(), |q| format!(" + {}", q.display));
            format!("{}{offset}", a.from)
        });
        let segments: Vec<String> = s
            .segments
            .iter()
            .map(|g| format!("{} {}", g.mode, g.duration.display))
            .collect();
        t.row(vec![
            C::Strong(s.name.clone()),
            C::Text(
                if modes.active_sequence.as_deref() == Some(s.name.as_str()) {
                    "yes"
                } else {
                    ""
                }
                .into(),
            ),
            C::Text(anchor),
            C::Text(segments.join(", ")),
        ]);
    }
    t.print();
    Ok(Some(0))
}

/// `fune mode ls|table`
pub fn mode(ws: &Workspace, cmd: &ModeCmd, out: &Out) -> Handled {
    let table = match cmd {
        ModeCmd::Ls => false,
        ModeCmd::Table => true,
        ModeCmd::View { system, opts } => {
            return super::pages::mode_view(ws, system, opts, out).map(Some);
        }
        _ => return Ok(None),
    };
    let modes = query::design_view(ws)?.modes;
    if out.json {
        out.json(&modes)?;
        return Ok(Some(0));
    }
    if !table {
        let mut t = out.table(&["ID", "Name", "Description", "Max duration"]);
        for m in &modes.modes {
            t.row(vec![
                C::Strong(m.id.clone()),
                C::Text(m.label.clone()),
                C::Dim(m.description.clone().unwrap_or_default()),
                C::Text(
                    m.max_duration
                        .as_ref()
                        .map(|q| q.display.clone())
                        .unwrap_or_default(),
                ),
            ]);
        }
        t.print();
        return Ok(Some(0));
    }
    let mut headers = vec!["Part"];
    headers.extend(modes.modes.iter().map(|m| m.id.as_str()));
    let mut t = out.table(&headers);
    for row in &modes.rows {
        let mut cells = vec![C::Strong(row.part.clone())];
        for m in &modes.modes {
            cells.push(match row.cells.get(&m.id) {
                None => C::Dim("not specified".into()),
                Some(cell) => {
                    let mut text = cell_text(&cell.state, row.shares.get(&m.id));
                    if let Some(n) = cell.active {
                        text.push_str(&format!(" [active={n}]"));
                    }
                    if let Some(s) = &cell.spare_state {
                        text.push_str(&format!(" [spare={s}]"));
                    }
                    if let Some(e) = row.eclipse_cells.get(&m.id) {
                        text.push_str(&format!(" [eclipse={}]", e.state));
                    }
                    C::Text(text)
                }
            });
        }
        t.row(cells);
    }
    t.print();
    Ok(Some(0))
}

/// モード表のセルの文言（`rx（tx 10.0 %）`）。割合の表示（`%`）は service が整形したもの（`ModeRow.shares`）。
fn cell_text(state: &str, shares: Option<&BTreeMap<String, QuantityView>>) -> String {
    let shares: Vec<String> = shares
        .into_iter()
        .flatten()
        .map(|(s, q)| format!("{s} {}", q.display))
        .collect();
    if shares.is_empty() {
        state.to_owned()
    } else {
        format!("{state} ({})", shares.join(", "))
    }
}

/// `fune net ls`
pub fn net(ws: &Workspace, cmd: &NetCmd, out: &Out) -> Handled {
    if !matches!(cmd, NetCmd::Ls) {
        return Ok(None);
    }
    let nets = query::design_view(ws)?.nets;
    if out.json {
        out.json(&nets)?;
        return Ok(Some(0));
    }
    let mut t = out.table(&["ID", "Kind", "Source", "Rail", "Connected ports"]);
    t = t.numeric(&[3]);
    for n in &nets {
        t.row(vec![
            C::Strong(n.id.clone()),
            C::Text(n.kind.to_string()),
            C::Text(
                n.source
                    .clone()
                    .or_else(|| n.master.clone())
                    .map(|r| r.to_string())
                    .unwrap_or_else(|| "—".into()),
            ),
            n.rail
                .as_ref()
                .map_or(C::Dim("—".into()), |q| C::Num(q.display.clone())),
            C::Text(
                n.members
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
        ]);
    }
    t.print();
    Ok(Some(0))
}

/// `fune attr ls`
pub fn attr(ws: &Workspace, cmd: &AttrCmd, out: &Out) -> Handled {
    if !matches!(cmd, AttrCmd::Ls) {
        return Ok(None);
    }
    let defs = query::defs_view(ws)?;
    if out.json {
        out.json(&defs.attributes)?;
        return Ok(Some(0));
    }
    let mut t = out.table(&["ID", "Dimension", "Unit", "By state", "Definition"]);
    for a in &defs.attributes {
        t.row(vec![
            C::Strong(a.id.clone()),
            C::Text(
                a.dimension
                    .clone()
                    .unwrap_or_else(|| a.value_type.to_string()),
            ),
            C::Text(a.units.join(", ")),
            C::Text(if a.by_state { "✓" } else { "" }.into()),
            C::Dim(if a.builtin { "built-in" } else { "defs/" }.into()),
        ]);
    }
    t.print();
    Ok(Some(0))
}

/// `fune kind ls`
pub fn kind(ws: &Workspace, cmd: &KindCmd, out: &Out) -> Handled {
    if !matches!(cmd, KindCmd::Ls) {
        return Ok(None);
    }
    let defs = query::defs_view(ws)?;
    if out.json {
        out.json(&defs.kinds)?;
        return Ok(Some(0));
    }
    let mut t = out.table(&["ID", "role", "States", "Required attributes", "Definition"]);
    for k in &defs.kinds {
        t.row(vec![
            C::Strong(k.id.clone()),
            C::Text(k.role.map(|r| r.to_string()).unwrap_or_default()),
            C::Text(k.states.join(" / ")),
            C::Text(k.required.join(", ")),
            C::Dim(if k.builtin { "built-in" } else { "defs/" }.into()),
        ]);
    }
    t.print();
    Ok(Some(0))
}

/// `fune setting ls`
pub fn setting(ws: &Workspace, cmd: &SettingCmd, out: &Out) -> Handled {
    if !matches!(cmd, SettingCmd::Ls) {
        return Ok(None);
    }
    let settings = query::design_view(ws)?.settings;
    if out.json {
        out.json(&settings)?;
        return Ok(Some(0));
    }
    let mut t = out.table(&["Key", "Value", "File"]);
    t = t.numeric(&[1]);
    for s in &settings {
        let value = match (&s.value, &s.display) {
            (ValueData::Unset, _) | (_, None) => C::Dim(UNSET.into()),
            (_, Some(text)) => C::Num(text.clone()),
        };
        t.row(vec![
            C::Text(s.key.clone()),
            value,
            C::Dim(s.file.clone().unwrap_or_default()),
        ]);
    }
    t.print();
    Ok(Some(0))
}

fn short(rev: &str) -> &str {
    rev.get(..12).unwrap_or(rev)
}

/// `fune view ls <view>`
pub fn view(ws: &Workspace, cmd: &crate::args::ViewCmd, out: &Out) -> Handled {
    let crate::args::ViewCmd::Ls { view } = cmd else {
        return Ok(None);
    };
    let layout = query::view_layout(ws, view)?;
    if out.json {
        out.json(&layout)?;
        return Ok(Some(0));
    }
    let Some(layout) = layout else {
        out.warn(format!(
            "No layout for diagram {view} yet (view/{view}.toml)"
        ));
        return Ok(Some(0));
    };
    let mut t = out
        .table(&["Node", "x", "y", "w", "h"])
        .numeric(&[1, 2, 3, 4]);
    let opt = |v: Option<i32>| v.map_or(C::Dim("—".into()), |v| C::Num(v.to_string()));
    for (id, n) in &layout.nodes {
        t.row(vec![
            C::Strong(id.clone()),
            C::Num(n.x.to_string()),
            C::Num(n.y.to_string()),
            opt(n.w),
            opt(n.h),
        ]);
    }
    t.print();
    Ok(Some(0))
}

/// `fune case ls`
pub fn case(ws: &Workspace, cmd: &crate::args::CaseCmd, out: &Out) -> Handled {
    if !matches!(cmd, crate::args::CaseCmd::Ls) {
        return Ok(None);
    }
    let cases = query::design_view(ws)?.cases;
    if out.json {
        out.json(&cases)?;
        return Ok(Some(0));
    }
    let mut t = out.table(&["ID", "Name", "Modes", "Phase", "File"]);
    for c in &cases {
        t.row(vec![
            C::Strong(c.id.clone()),
            C::Text(c.label.clone()),
            C::Text(c.modes.join(" ")),
            C::Text(c.phase.clone().unwrap_or_default()),
            C::Dim(c.file.clone()),
        ]);
    }
    t.print();
    Ok(Some(0))
}

/// `fune source ls`
pub fn source(ws: &Workspace, cmd: &crate::args::SourceCmd, out: &Out) -> Handled {
    if !matches!(cmd, crate::args::SourceCmd::Ls) {
        return Ok(None);
    }
    let sources = query::design_view(ws)?.sources;
    if out.json {
        out.json(&sources)?;
        return Ok(Some(0));
    }
    let mut t = out.table(&[
        "ID",
        "Kind",
        "Name",
        "Reference",
        "Revision",
        "Distribution",
        "Date",
    ]);
    for s in sources.iter().map(|v| &v.source) {
        let distribution = s
            .distribution
            .map(|d| query::enum_label("distribution", &d));
        t.row(vec![
            C::Strong(s.id.to_string()),
            C::Text(s.kind.as_str().to_owned()),
            C::Text(s.label.clone().unwrap_or_default()),
            C::Dim(s.reference.clone().unwrap_or_default()),
            C::Dim(s.revision.clone().unwrap_or_default()),
            C::Dim(distribution.unwrap_or_default()),
            C::Dim(s.date.clone().unwrap_or_default()),
        ]);
    }
    t.print();
    Ok(Some(0))
}

/// `fune harness ls`
pub fn harness(ws: &Workspace, cmd: &crate::args::HarnessCmd, out: &Out) -> Handled {
    if !matches!(cmd, crate::args::HarnessCmd::Ls) {
        return Ok(None);
    }
    let harnesses = query::design_view(ws)?.harnesses;
    if out.json {
        out.json(&harnesses)?;
        return Ok(Some(0));
    }
    let mut t = out
        .table(&["ID", "From", "To", "Nets", "Length"])
        .numeric(&[4]);
    for h in &harnesses {
        t.row(vec![
            C::Strong(h.id.clone()),
            C::Text(h.from.to_string()),
            C::Text(h.to.to_string()),
            C::Text(h.nets.join(", ")),
            h.length
                .as_ref()
                .map_or(C::Dim("—".into()), |q| C::Num(q.display.clone())),
        ]);
    }
    t.print();
    Ok(Some(0))
}
