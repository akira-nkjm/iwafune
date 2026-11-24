//! `fune thermal tables`（`POST /thermal/tables` と同じ DTO を表にする）。値・出どころ・合計は service が作ったものを並べる。

use crate::args::ThermalCmd;
use crate::output::{C, Out};
use fune_service::Workspace;
use fune_service::dto::{ThermalCouplingKind, ThermalTablesRequest};

/// `fune thermal tables [--ops ops.json] [--case ID]`
pub fn thermal(ws: &Workspace, cmd: &ThermalCmd, out: &Out) -> anyhow::Result<Option<u8>> {
    let ThermalCmd::Tables { ops, case } = cmd;
    let req = ThermalTablesRequest {
        case: case.clone(),
        ops: match ops {
            Some(path) => super::ops::ops_from_file(path)?,
            None => Vec::new(),
        },
    };
    let view = fune_service::thermal_tables::thermal_tables(ws, &req)?;
    if out.json {
        out.json(&view)?;
        return Ok(Some(0));
    }
    out.heading("Couplings");
    if !view.couplings.available {
        println!("Not available (the analysis stopped on a missing input)");
    }
    let mut t = out
        .table(&["Kind", "Node A", "Node B", "Value", "Source"])
        .numeric(&[3]);
    for r in &view.couplings.rows {
        let kind = match r.kind {
            ThermalCouplingKind::Conductive => "conductive",
            ThermalCouplingKind::Radiative => "radiative",
        };
        t.row(vec![
            C::Text(kind.into()),
            C::Strong(r.a.clone()),
            C::Strong(r.b.clone()),
            C::Num(r.value.display.clone()),
            C::Text(r.source_label.label_en.clone()),
        ]);
    }
    t.print();
    out.heading("Surfaces");
    let mut t = out
        .table(&["Part", "Node", "Area", "Coatings (fraction) or part values"])
        .numeric(&[2]);
    for s in &view.surfaces {
        let optics = if let Some(u) = &s.uniform {
            format!("α {} ε {}", u.absorptivity.display, u.emissivity.display)
        } else {
            let list: Vec<String> = s
                .fractions
                .iter()
                .map(|f| format!("{} ({})", f.coating, f.fraction.display))
                .collect();
            format!("{} total {}", list.join(", "), s.total.display)
        };
        t.row(vec![
            C::Strong(s.part.clone()),
            C::Text(s.node.clone().unwrap_or_default()),
            C::Num(
                s.area
                    .as_ref()
                    .map(|a| a.display.clone())
                    .unwrap_or_default(),
            ),
            C::Text(optics),
        ]);
    }
    t.print();
    out.heading("Coatings");
    let mut t = out
        .table(&["ID", "Label", "Absorptivity", "Emissivity", "Used by"])
        .numeric(&[2, 3]);
    for c in &view.coatings {
        let q = |v: &Option<fune_service::dto::QuantityView>| {
            v.as_ref().map(|q| q.display.clone()).unwrap_or_default()
        };
        t.row(vec![
            C::Strong(c.id.clone()),
            C::Text(c.label.clone().unwrap_or_default()),
            C::Num(q(&c.absorptivity)),
            C::Num(q(&c.emissivity)),
            C::Text(c.used_by.join(", ")),
        ]);
    }
    t.print();
    Ok(Some(0))
}
