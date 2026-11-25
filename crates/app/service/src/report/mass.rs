//! 質量表。部品の行は質量バジェットが数える 1 個あたりの質量・員数・成長余裕（`budget::mass_part_lines`）、
//! 小計・合計・余裕・推薬は `budget::budget` の結果をそのまま使う（ここで集計し直さない）。

use super::column::{label_of, number_in, percent_number, pick_unit_smallest, view_cell};
use super::common::{empty_table, footer, note_unavailable, part_name};
use super::{Ctx, Out};
use crate::budget::{self, PartLine};
use crate::dto::{BudgetKind, BudgetPropellant, BudgetRow, BudgetView, ReportItem};
use crate::error::Result;
use fune_report::labels::{Text, t, tr};
use fune_report::model::{Align, Cell, Column, Row, RowStyle, Table};

pub(super) fn build(ctx: &Ctx, out: &mut Out) -> Result<()> {
    let caption = t(ctx.lang, Text::MassCaption).to_owned();
    let view = budget::budget_cached(&ctx.session, BudgetKind::Mass, None)?;
    let Some(total) = view.total.as_ref().filter(|_| view.missing.is_empty()) else {
        let paths: Vec<&str> = view.missing.iter().map(|m| m.path.as_str()).collect();
        let reason = format!("missing inputs ({})", paths.join(", "));
        out.tables
            .push(empty_table(ctx, ReportItem::Mass, None, caption, &reason));
        note_unavailable(out, ReportItem::Mass, None, &reason);
        return Ok(());
    };
    let (lines, dry_total) = budget::mass_part_lines(ctx.snapshot())?;
    let unit = pick_unit_smallest("Mass", lines.iter().map(|l| l.value));
    let mut rows = Vec::new();
    for r in &view.rows {
        rows.extend(system_rows(ctx, r, &lines, &unit));
    }
    rows.extend(total_rows(ctx, &view, dry_total, &unit));
    out.tables.push(Table {
        item: ReportItem::Mass,
        case: None,
        subject: None,
        variant: None,
        caption,
        columns: columns(ctx, &unit),
        rows,
        notes: notes(ctx, &view),
        unavailable: None,
    });
    super::values::mass(ctx, out, &view, total, &lines);
    Ok(())
}

fn columns(ctx: &Ctx, unit: &str) -> Vec<Column> {
    let col = |text: Text, unit: Option<String>, align| Column {
        header: t(ctx.lang, text).to_owned(),
        unit,
        align,
    };
    let mass = Some(label_of(unit));
    vec![
        col(Text::Item, None, Align::Left),
        col(Text::Quantity, None, Align::Right),
        col(Text::UnitMass, mass.clone(), Align::Right),
        col(Text::Mass, mass.clone(), Align::Right),
        col(Text::Growth, Some("%".to_owned()), Align::Right),
        col(Text::MassWithGrowth, mass, Align::Right),
    ]
}

/// 配分の行 1 つ分（見出し・部品・小計）。部品が無い行は出さない。
fn system_rows(ctx: &Ctx, r: &BudgetRow, lines: &[PartLine], unit: &str) -> Vec<Row> {
    let mut mine: Vec<&PartLine> = lines.iter().filter(|l| l.row == r.node).collect();
    if mine.is_empty() {
        return Vec::new();
    }
    mine.sort_by(|a, b| a.part.cmp(&b.part));
    let name = system_label(ctx, r);
    let mut rows = vec![Row {
        style: RowStyle::Group,
        cells: vec![Cell::text(name.clone())],
    }];
    for l in mine {
        let each = l.value;
        let all = each * f64::from(l.count);
        rows.push(Row {
            style: RowStyle::Body,
            cells: vec![
                Cell::text(part_name(ctx, &l.part)),
                Cell::number(l.count.to_string(), f64::from(l.count)),
                number_in(each, unit),
                number_in(all, unit),
                Cell::number(percent_number(l.growth), l.growth),
                number_in(all * (1.0 + l.growth), unit),
            ],
        });
    }
    rows.push(Row {
        style: RowStyle::Subtotal,
        cells: vec![
            Cell::text(tr(ctx.lang, Text::Subtotal, &[&name])),
            Cell::empty(),
            Cell::empty(),
            view_cell(&r.current, unit),
            Cell::empty(),
            view_cell(&r.with_growth, unit),
        ],
    });
    rows
}

/// 配分の行の表示名（系は日英の名前、部品の配分は部品の名前、どれにも属さない行は「その他」）。
fn system_label(ctx: &Ctx, r: &BudgetRow) -> String {
    match (&r.node, &r.label) {
        (None, _) => {
            let other = fune_model::builtin::role_other_label();
            super::common::pick_label(ctx.lang, &other.label, other.label_en.as_deref())
        }
        (Some(node), None) => node.clone(),
        (Some(_), Some(label)) => super::common::pick_label(ctx.lang, label, r.label_en.as_deref()),
    }
}

/// 末尾の行（合計・上限・余裕・推薬込み）。
fn total_rows(ctx: &Ctx, view: &BudgetView, dry_total: f64, unit: &str) -> Vec<Row> {
    let line = |style, text: Text, mass: Option<Cell>, with_growth: Option<Cell>| Row {
        style,
        cells: vec![
            Cell::text(t(ctx.lang, text)),
            Cell::empty(),
            Cell::empty(),
            mass.unwrap_or_else(Cell::empty),
            Cell::empty(),
            with_growth.unwrap_or_else(Cell::empty),
        ],
    };
    let cell = |q: &Option<crate::dto::QuantityView>| q.as_ref().map(|q| view_cell(q, unit));
    let mut rows = vec![line(
        RowStyle::Total,
        Text::TotalDry,
        Some(number_in(dry_total, unit)),
        cell(&view.total),
    )];
    rows.push(line(RowStyle::Margin, Text::Limit, None, cell(&view.limit)));
    rows.push(line(
        RowStyle::Margin,
        Text::Margin,
        None,
        cell(&view.margin),
    ));
    if let Some(BudgetPropellant::Counted { loaded, wet, .. }) = &view.propellant {
        rows.push(line(
            RowStyle::Body,
            Text::Propellant,
            None,
            Some(view_cell(loaded, unit)),
        ));
        rows.push(line(
            RowStyle::Total,
            Text::TotalWet,
            None,
            Some(view_cell(&wet.total, unit)),
        ));
        rows.push(line(
            RowStyle::Margin,
            Text::Margin,
            None,
            cell(&wet.margin),
        ));
    }
    rows
}

fn notes(ctx: &Ctx, view: &BudgetView) -> Vec<String> {
    let margin = format!(
        "{}.",
        tr(ctx.lang, Text::SystemMargin, &[&view.system_margin.display])
    );
    let mut notes = vec![t(ctx.lang, Text::MassNote).to_owned(), margin];
    notes.extend(footer(ctx));
    notes
}
