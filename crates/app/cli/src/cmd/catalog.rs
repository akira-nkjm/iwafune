//! `fune catalog ls|show`（カタログ品を読む）。編集は `records::catalog`。

use super::common::Handled;
use crate::args::CatalogCmd;
use crate::output::{C, Out};
use fune_service::{Workspace, query};

/// `fune catalog ls|show`
pub fn read(ws: &Workspace, cmd: &CatalogCmd, out: &Out) -> Handled {
    let items = match cmd {
        CatalogCmd::Ls | CatalogCmd::Show { .. } => query::design_view(ws)?.catalog,
        _ => return Ok(None),
    };
    if let CatalogCmd::Show { id } = cmd {
        let item = items
            .into_iter()
            .find(|i| &i.id == id)
            .ok_or_else(|| anyhow::anyhow!("カタログ品がありません: {id}"))?;
        if out.json {
            out.json(&item)?;
            return Ok(Some(0));
        }
        out.heading(&item.id);
        let mut t = out.table(&["属性", "値"]);
        for (attr, v) in &item.values {
            t.row(vec![C::Text(attr.clone()), C::Text(v.display.clone())]);
        }
        t.print();
        return Ok(Some(0));
    }
    if out.json {
        out.json(&items)?;
        return Ok(Some(0));
    }
    let mut t = out
        .table(&["ID", "種別", "名前", "型番", "属性"])
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
