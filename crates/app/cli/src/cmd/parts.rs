//! 部品を読むコマンド: `fune part ls`・`fune part show`（`--ops` で下書きを当てる。と、値の出所の表示）。

use super::common::{Handled, UNSET, provenance};
use crate::args::PartCmd;
use crate::diag;
use crate::output::{C, Out};
use fune_service::dto::{Origin, PartViewRequest, PartsQuery, ValueData, ValueView};
use fune_service::{Workspace, query};

/// `fune part ls|show`
pub fn part(ws: &Workspace, cmd: &PartCmd, out: &Out) -> Handled {
    match cmd {
        PartCmd::Ls { group, kind } => {
            let q = PartsQuery {
                group: group.clone(),
                kind: kind.clone(),
            };
            part_ls(ws, &q, out).map(Some)
        }
        PartCmd::Show { id, ops } => {
            let req = PartViewRequest {
                ops: match ops {
                    Some(path) => super::ops::ops_from_file(path)?,
                    None => Vec::new(),
                },
            };
            part_show(ws, id, &req, out).map(Some)
        }
        _ => Ok(None),
    }
}

const PART_COLUMNS: [&str; 3] = ["mass", "power", "temp_op"];

fn part_ls(ws: &Workspace, q: &PartsQuery, out: &Out) -> anyhow::Result<u8> {
    let rows = query::list_parts(ws, q)?;
    if out.json {
        out.json(&rows)?;
        return Ok(0);
    }
    let mut t = out.table(&[
        "ID",
        "Subsystem",
        "Kind",
        "Definition",
        "Count",
        "Mass",
        "Power",
        "Operating temp",
    ]);
    t = t.numeric(&[4, 5, 6]);
    for r in &rows {
        let mut cells = vec![
            C::Strong(r.id.clone()),
            C::Text(
                r.system
                    .map(|s| query::system_id(s).to_owned())
                    .unwrap_or_default(),
            ),
            C::Text(r.kind.clone().unwrap_or_default()),
            C::Dim(r.catalog.clone().unwrap_or_else(|| "—".into())),
            C::Num(r.count.to_string()),
        ];
        for attr in PART_COLUMNS {
            cells.push(compact_cell(r.values.get(attr)));
        }
        t.row(cells);
    }
    t.print();
    println!("{}", out.dim(format!("{} parts", rows.len())));
    Ok(0)
}

fn value_cell(v: Option<&ValueView>) -> C {
    match v.map(|v| (v.origin, v.display.as_deref())) {
        None => C::Dim("—".into()),
        Some((Origin::Unset, _) | (_, None)) => C::Dim(UNSET.into()),
        Some((_, Some(text))) => C::Num(text.to_owned()),
    }
}

/// 一覧の表では、状態ごとの値は `on`（無ければ最大の状態）だけを出す。
fn compact_cell(v: Option<&ValueView>) -> C {
    let Some(ValueView {
        value: ValueData::ByState { states },
        ..
    }) = v
    else {
        return value_cell(v);
    };
    let pick = states
        .iter()
        .find(|(s, _)| s.as_str() == "on")
        .or_else(|| states.iter().max_by(|a, b| a.1.si.total_cmp(&b.1.si)));
    match pick {
        Some((s, q)) => C::Num(format!("{} ({s})", q.display)),
        None => value_cell(v),
    }
}

fn part_show(ws: &Workspace, id: &str, req: &PartViewRequest, out: &Out) -> anyhow::Result<u8> {
    let p = query::part_view(ws, id, req)?;
    if out.json {
        out.json(&p)?;
        return Ok(0);
    }
    let r = &p.part;
    out.heading(&r.id);
    let def = r
        .catalog
        .clone()
        .unwrap_or_else(|| format!("kind {}", r.kind.as_deref().unwrap_or_default()));
    println!("{}", out.dim(format!("{def}  ×{}  {}", r.count, r.file)));
    for section in &p.sections {
        let mut t = out.table(&[
            query::english(&section.label, section.label_en.as_deref()),
            "Value",
            "Source",
        ]);
        t = t.numeric(&[1]);
        for attr in &section.attrs {
            if let Some(v) = r.values.get(attr) {
                t.row(vec![
                    C::Text(attr.clone()),
                    value_cell(Some(v)),
                    C::Dim(provenance(v)),
                ]);
            }
        }
        t.print();
    }
    if !r.ports.is_empty() {
        let mut t = out.table(&["Port", "Net"]);
        for port in &r.ports {
            t.row(vec![
                C::Text(port.id.clone()),
                C::Text(port.net.clone().unwrap_or_else(|| "—".into())),
            ]);
        }
        t.print();
    }
    if !p.diagnostics.is_empty() {
        eprint!("{}", diag::render(&p.diagnostics, out.color));
    }
    Ok(0)
}
