//! `fune units ls|parse`

use crate::args::UnitsCmd;
use crate::output::{C, Out};
use fune_service::dto::{FormatItem, FormatRequest, ParseQuantityRequest, UnitMapping, UnitsQuery};
use fune_service::units;

/// 単位のコマンド。
pub fn run(cmd: &UnitsCmd, out: &Out) -> anyhow::Result<u8> {
    match cmd {
        UnitsCmd::Ls { dimension } => ls(dimension.clone(), out),
        UnitsCmd::Parse { input, dimension } => parse(input, dimension.clone(), out),
        UnitsCmd::Format {
            values,
            dimension,
            unit,
        } => format(values, dimension, unit.as_deref(), out),
    }
}

fn ls(dimension: Option<String>, out: &Out) -> anyhow::Result<u8> {
    let catalog = units::units_catalog(&UnitsQuery { dimension })?;
    if out.json {
        out.json(&catalog)?;
        return Ok(0);
    }
    let mut t = out.table(&["記号", "次元", "SI 換算", "種類"]);
    for u in &catalog.units {
        let kind = match u.mapping {
            UnitMapping::Linear => "",
            UnitMapping::Affine => "affine",
            UnitMapping::Logarithmic => "対数",
        };
        let factor = if u.offset == 0.0 {
            format!("× {}", u.scale_text)
        } else {
            format!("× {} + {}", u.scale_text, units::number_text(u.offset))
        };
        t.row(vec![
            C::Strong(u.symbol.clone()),
            C::Text(u.dimension.clone()),
            C::Num(factor),
            C::Dim(kind.to_owned()),
        ]);
    }
    t.print();
    println!("{}", out.dim(format!("{} 単位", catalog.units.len())));
    Ok(0)
}

fn parse(input: &str, dimension: Option<String>, out: &Out) -> anyhow::Result<u8> {
    let r = units::parse_quantity(&ParseQuantityRequest {
        input: input.to_owned(),
        dimension,
    })?;
    if out.json {
        out.json(&r)?;
        return Ok(0);
    }
    let q = &r.quantity;
    let mut t = out.table(&["入力", "次元", "SI"]);
    t.row(vec![
        C::Num(q.display.clone()),
        C::Text(q.dimension.clone()),
        C::Num(r.si.display.clone()),
    ]);
    t.print();
    Ok(0)
}

fn format(values: &[f64], dimension: &str, unit: Option<&str>, out: &Out) -> anyhow::Result<u8> {
    let req = FormatRequest {
        items: values
            .iter()
            .map(|si| FormatItem {
                si: *si,
                dimension: dimension.to_owned(),
                unit: unit.map(str::to_owned),
            })
            .collect(),
    };
    let r = units::format_quantities(&req)?;
    if out.json {
        out.json(&r)?;
        return Ok(0);
    }
    let mut t = out.table(&["SI", "表示"]).numeric(&[0, 1]);
    for (si, f) in values.iter().zip(&r.items) {
        t.row(vec![C::Num(format!("{si:e}")), C::Num(f.display.clone())]);
    }
    t.print();
    Ok(0)
}
