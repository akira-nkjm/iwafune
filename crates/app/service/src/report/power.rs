//! モードごとの電力表（機器 × モード）。値は `power.mode_load` の型付きの出力（`ModeLoadOutput`）の平均消費電力と合計をそのまま使う。

use super::column::{label_of, number_in, pick_unit_largest};
use super::common::{Counted, empty_table, footer, note_unavailable, run};
use super::{Ctx, Out};
use crate::dto::{ReportItem, ReportLang};
use crate::error::Result;
use crate::units::percent;
use fune_report::labels::{Text, t};
use fune_report::model::{Align, Cell, Column, Row, RowStyle, Table};
use sat_analysis::power::mode_load::{
    ModeLoad, ModeLoadMode, ModeLoadPartResult, ModeLoadStateShare,
};
use std::collections::BTreeMap;

/// モードの合計の欄を選ぶ（SI の値）。
type Pick = fn(&ModeLoadMode) -> f64;

pub(super) fn build(ctx: &Ctx, out: &mut Out) -> Result<()> {
    let caption = t(ctx.lang, Text::PowerCaption).to_owned();
    let run = run::<ModeLoad>(ctx, None)?;
    super::values::kpis(ctx, out, &run.result, None);
    let output = match run.counted()? {
        Counted::Yes { output, .. } => output,
        Counted::No(reason) => {
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
    let modes = &output.modes;
    let all_means = modes
        .iter()
        .flat_map(|m| &m.parts)
        .filter_map(|p| p.mean.map(|q| q.value()))
        .chain(modes.iter().map(|m| m.peak_with_growth.value()));
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

fn columns(ctx: &Ctx, modes: &[ModeLoadMode], unit: &str) -> Vec<Column> {
    let mut cols = vec![Column {
        header: t(ctx.lang, Text::Component).to_owned(),
        unit: None,
        align: Align::Left,
    }];
    cols.extend(modes.iter().map(|m| Column {
        header: m.label.clone(),
        unit: Some(label_of(unit)),
        align: Align::Right,
    }));
    cols
}

/// 機器の行（系ごとの見出しつき）。機器はモード順の最初に出た順に並べ、系は ID 順。
fn part_rows(ctx: &Ctx, modes: &[ModeLoadMode], unit: &str) -> Vec<Row> {
    let mut groups: BTreeMap<&str, Vec<&ModeLoadPartResult>> = BTreeMap::new();
    for p in modes.iter().flat_map(|m| &m.parts) {
        let entry = groups.entry(p.group.as_str()).or_default();
        if !entry.iter().any(|e| e.part == p.part) {
            entry.push(p);
        }
    }
    let mut rows = Vec::new();
    for (group, parts) in groups {
        rows.push(Row {
            style: RowStyle::Group,
            cells: vec![Cell::text(group_label(ctx.lang, group))],
        });
        for first in parts {
            let id = &first.part;
            let mut cells = vec![Cell::text(format!("{} ({id})", first.label))];
            cells.extend(modes.iter().map(|m| part_cell(ctx.lang, m, id, unit)));
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
fn part_cell(lang: ReportLang, mode: &ModeLoadMode, id: &str, unit: &str) -> Cell {
    let Some(p) = mode.parts.iter().find(|p| p.part == id) else {
        return Cell::empty();
    };
    let Some(mean) = p.mean else {
        return Cell::text("-");
    };
    let mut cell = number_in(mean.value(), unit);
    let states = state_text(lang, &p.states);
    cell.detail = (!states.is_empty()).then_some(states);
    cell
}

/// 状態の表示（1 つだけならその名前、複数なら割合も添える）。
fn state_text(lang: ReportLang, states: &[ModeLoadStateShare]) -> String {
    let label = |s: &ModeLoadStateShare| match fune_model::builtin::state_label(&s.state) {
        Some(l) => super::common::pick_label(lang, &l.label, l.label_en.as_deref()),
        None => s.state.clone(),
    };
    if let [one] = states {
        return label(one);
    }
    states
        .iter()
        .map(|s| {
            let fraction = percent(s.fraction.value()).display;
            format!("{} {fraction}", label(s))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// モードの合計の行（余裕なしと成長余裕込み。蝕中が日照中と違うときは蝕中も）とピーク。
fn total_rows(ctx: &Ctx, modes: &[ModeLoadMode], unit: &str) -> Vec<Row> {
    let differs = |a: Pick, b: Pick| modes.iter().any(|m| a(m) != b(m));
    let mut specs: Vec<(Text, Pick, RowStyle)> = vec![
        (
            Text::TotalWithoutMargin,
            |m| m.total.value(),
            RowStyle::Total,
        ),
        (
            Text::TotalWithMargin,
            |m| m.total_with_growth.value(),
            RowStyle::Total,
        ),
    ];
    if differs(|m| m.total.value(), |m| m.total_eclipse.value())
        || differs(
            |m| m.total_with_growth.value(),
            |m| m.total_eclipse_with_growth.value(),
        )
    {
        specs.push((
            Text::EclipseWithoutMargin,
            |m| m.total_eclipse.value(),
            RowStyle::Subtotal,
        ));
        specs.push((
            Text::EclipseWithMargin,
            |m| m.total_eclipse_with_growth.value(),
            RowStyle::Subtotal,
        ));
    }
    specs.push((
        Text::PeakWithMargin,
        |m| m.peak_with_growth.value(),
        RowStyle::Margin,
    ));
    specs
        .into_iter()
        .map(|(label, pick, style)| {
            let mut cells = vec![Cell::text(t(ctx.lang, label))];
            cells.extend(modes.iter().map(|m| number_in(pick(m), unit)));
            Row { style, cells }
        })
        .collect()
}
