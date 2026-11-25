//! モードごとの電力表（機器 × モード）。値は `power.mode_load` の平均消費電力と合計をそのまま使う。

use super::column::{label_of, number_in, pick_unit_largest};
use super::common::{empty_table, footer, note_unavailable, run, why_not};
use super::json::{list, num, text};
use super::{Ctx, Out};
use crate::dto::{ReportItem, ReportLang};
use crate::error::Result;
use crate::units::percent;
use fune_report::labels::{Text, t};
use fune_report::model::{Align, Cell, Column, Row, RowStyle, Table};
use serde_json::Value;
use std::collections::BTreeMap;

pub(super) fn build(ctx: &Ctx, out: &mut Out) -> Result<()> {
    let caption = t(ctx.lang, Text::PowerCaption).to_owned();
    let result = run(ctx, "power.mode_load", None)?;
    super::values::kpis(ctx, out, &result, None);
    let output = match (&why_not(&result), &result.output) {
        (None, Some(o)) => o,
        (reason, _) => {
            let reason = reason.clone().unwrap_or_default();
            out.tables.push(empty_table(
                ctx,
                ReportItem::ModePower,
                None,
                caption,
                &reason,
            ));
            note_unavailable(out, ReportItem::ModePower, None, &reason);
            return Ok(());
        }
    };
    let modes = list(output, "modes");
    let all_means = modes
        .iter()
        .flat_map(|m| list(m, "parts"))
        .filter_map(|p| num(p, "mean"))
        .chain(modes.iter().filter_map(|m| num(m, "peak_with_growth")));
    let unit = pick_unit_largest("Power", all_means);
    let mut rows = part_rows(ctx, modes, &unit);
    rows.extend(total_rows(ctx, modes, &unit));
    let mut notes = vec![t(ctx.lang, Text::PowerNote).to_owned()];
    notes.extend(footer(ctx));
    out.tables.push(Table {
        item: ReportItem::ModePower,
        case: None,
        subject: None,
        variant: None,
        caption,
        columns: columns(ctx, modes, &unit),
        rows,
        notes,
        unavailable: None,
    });
    Ok(())
}

fn columns(ctx: &Ctx, modes: &[Value], unit: &str) -> Vec<Column> {
    let mut cols = vec![Column {
        header: t(ctx.lang, Text::Component).to_owned(),
        unit: None,
        align: Align::Left,
    }];
    cols.extend(modes.iter().map(|m| {
        Column {
            header: text(m, "label")
                .or_else(|| text(m, "id"))
                .unwrap_or_default()
                .to_owned(),
            unit: Some(label_of(unit)),
            align: Align::Right,
        }
    }));
    cols
}

/// 機器の行（系ごとの見出しつき）。機器はモード順の最初に出た順に並べ、系は ID 順。
fn part_rows(ctx: &Ctx, modes: &[Value], unit: &str) -> Vec<Row> {
    let mut groups: BTreeMap<String, Vec<(String, &Value)>> = BTreeMap::new();
    for p in modes.iter().flat_map(|m| list(m, "parts")) {
        let id = text(p, "part").unwrap_or_default().to_owned();
        let group = text(p, "group").unwrap_or_default().to_owned();
        let entry = groups.entry(group).or_default();
        if !entry.iter().any(|(i, _)| *i == id) {
            entry.push((id, p));
        }
    }
    let mut rows = Vec::new();
    for (group, parts) in groups {
        rows.push(Row {
            style: RowStyle::Group,
            cells: vec![Cell::text(group_label(ctx.lang, &group))],
        });
        for (id, first) in parts {
            let name = text(first, "label").map_or(id.clone(), |l| format!("{l} ({id})"));
            let mut cells = vec![Cell::text(name)];
            cells.extend(modes.iter().map(|m| part_cell(ctx.lang, m, &id, unit)));
            rows.push(Row {
                style: RowStyle::Body,
                cells,
            });
        }
    }
    rows
}

fn group_label(lang: ReportLang, group: &str) -> String {
    match fune_model::builtin::labels().section.get(group) {
        Some(l) => super::common::pick_label(lang, &l.label, l.label_en.as_deref()),
        None => group.to_owned(),
    }
}

/// そのモードの機器の平均消費電力（状態と時間の割合を添える）。
fn part_cell(lang: ReportLang, mode: &Value, id: &str, unit: &str) -> Cell {
    let Some(p) = list(mode, "parts")
        .iter()
        .find(|p| text(p, "part") == Some(id))
    else {
        return Cell::empty();
    };
    let Some(mean) = num(p, "mean") else {
        return Cell::text("-");
    };
    let mut cell = number_in(mean, unit);
    let states = state_text(lang, list(p, "states"));
    cell.detail = (!states.is_empty()).then_some(states);
    cell
}

/// 状態の表示（1 つだけならその名前、複数なら割合も添える）。
fn state_text(lang: ReportLang, states: &[Value]) -> String {
    let label = |s: &Value| {
        let id = text(s, "state").unwrap_or_default();
        match fune_model::builtin::state_label(id) {
            Some(l) => super::common::pick_label(lang, &l.label, l.label_en.as_deref()),
            None => id.to_owned(),
        }
    };
    if let [one] = states {
        return label(one);
    }
    states
        .iter()
        .map(|s| {
            let fraction = percent(num(s, "fraction").unwrap_or(0.0)).display;
            format!("{} {fraction}", label(s))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// モードの合計の行（余裕なしと成長余裕込み。蝕中が日照中と違うときは蝕中も）とピーク。
fn total_rows(ctx: &Ctx, modes: &[Value], unit: &str) -> Vec<Row> {
    let differs = |a: &str, b: &str| {
        modes
            .iter()
            .any(|m| num(m, a).zip(num(m, b)).is_some_and(|(x, y)| x != y))
    };
    let mut specs = vec![
        (Text::TotalWithoutMargin, "total", RowStyle::Total),
        (Text::TotalWithMargin, "total_with_growth", RowStyle::Total),
    ];
    if differs("total", "total_eclipse")
        || differs("total_with_growth", "total_eclipse_with_growth")
    {
        specs.push((
            Text::EclipseWithoutMargin,
            "total_eclipse",
            RowStyle::Subtotal,
        ));
        specs.push((
            Text::EclipseWithMargin,
            "total_eclipse_with_growth",
            RowStyle::Subtotal,
        ));
    }
    specs.push((Text::PeakWithMargin, "peak_with_growth", RowStyle::Margin));
    specs
        .into_iter()
        .map(|(label, key, style)| {
            let mut cells = vec![Cell::text(t(ctx.lang, label))];
            cells.extend(modes.iter().map(|m| match num(m, key) {
                Some(v) => number_in(v, unit),
                None => Cell::empty(),
            }));
            Row { style, cells }
        })
        .collect()
}
