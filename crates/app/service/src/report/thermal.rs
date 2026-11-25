//! 温度の結果表。節点ごとに、定常解析のケース（高温・低温など）の温度を列に並べ、全ケースの最低・最高、許容範囲、判定を添える。
//! 値と判定は `thermal.steady` の結果（`envelope` の判定）をそのまま使う。

use super::column::{label_of, number_in, pick_unit_smallest};
use super::common::{
    case_label, effective, empty_table, footer, note_unavailable, remember_case, run, verdict_text,
    why_not,
};
use super::json::{get, list, num, text};
use super::{Ctx, Out};
use crate::dto::{ReportItem, ReportLang};
use crate::error::Result;
use fune_report::labels::{Text, t, tr};
use fune_report::model::{Align, Cell, Column, Row, RowStyle, Table};
use sat_analysis::spec::Verdict;
use serde_json::Value;

pub(super) fn build(ctx: &Ctx, case: Option<&str>, out: &mut Out) -> Result<()> {
    let result = run(ctx, "thermal.steady", case)?;
    let case = effective(&result, case);
    remember_case(out, &case);
    super::values::kpis(ctx, out, &result, case.as_deref());
    let caption = tr(
        ctx.lang,
        Text::ThermalCaption,
        &[&case_label(ctx, case.as_deref())],
    );
    let output = match (why_not(&result), &result.output) {
        (None, Some(o)) => o,
        (reason, _) => {
            let reason = reason.unwrap_or_default();
            out.tables.push(empty_table(
                ctx,
                ReportItem::ThermalResult,
                case.as_deref(),
                caption,
                &reason,
            ));
            note_unavailable(out, ReportItem::ThermalResult, case.as_deref(), &reason);
            return Ok(());
        }
    };
    let cases = list(output, "cases");
    let envelope = list(output, "envelope");
    let temps = cases
        .iter()
        .flat_map(|c| list(c, "nodes"))
        .filter_map(|n| num(n, "temperature"))
        .chain(envelope.iter().filter_map(|e| num(e, "min")))
        .chain(envelope.iter().filter_map(|e| num(e, "max")));
    let unit = pick_unit_smallest("Temperature", temps);
    let kind = range_kind(envelope);
    let rows = envelope
        .iter()
        .map(|e| node_row(ctx.lang, e, cases, &unit, kind))
        .collect();
    out.tables.push(Table {
        item: ReportItem::ThermalResult,
        case,
        subject: None,
        variant: None,
        caption,
        columns: columns(ctx, cases, &unit, kind),
        rows,
        notes: notes(ctx, output),
        unavailable: None,
    });
    Ok(())
}

/// 許容範囲として載せる種類（動作があれば動作、無ければ最初に出てくる種類）。
fn range_kind(envelope: &[Value]) -> Option<&str> {
    let kinds: Vec<&str> = envelope
        .iter()
        .flat_map(|e| list(e, "ranges"))
        .filter_map(|r| text(r, "kind"))
        .collect();
    kinds
        .iter()
        .find(|k| **k == "operating")
        .or_else(|| kinds.first())
        .copied()
}

fn kind_label(lang: ReportLang, kind: Option<&str>) -> &'static str {
    t(
        lang,
        match kind {
            Some("storage") => Text::RangeStorage,
            Some("performance") => Text::RangePerformance,
            _ => Text::RangeOperating,
        },
    )
}

fn columns(ctx: &Ctx, cases: &[Value], unit: &str, kind: Option<&str>) -> Vec<Column> {
    let temp = Some(label_of(unit));
    let col = |header: String, unit: Option<String>, align| Column {
        header,
        unit,
        align,
    };
    let mut cols = vec![col(t(ctx.lang, Text::Node).to_owned(), None, Align::Left)];
    cols.extend(cases.iter().map(|c| {
        let id = text(c, "id").unwrap_or_default();
        let label = text(c, "label").unwrap_or(id);
        col(label.to_owned(), temp.clone(), Align::Right)
    }));
    cols.push(col(
        t(ctx.lang, Text::Minimum).to_owned(),
        temp.clone(),
        Align::Right,
    ));
    cols.push(col(
        t(ctx.lang, Text::Maximum).to_owned(),
        temp.clone(),
        Align::Right,
    ));
    cols.push(col(
        tr(
            ctx.lang,
            Text::AllowableRange,
            &[kind_label(ctx.lang, kind)],
        ),
        temp,
        Align::Center,
    ));
    cols.push(col(
        t(ctx.lang, Text::Verdict).to_owned(),
        None,
        Align::Center,
    ));
    cols
}

fn node_row(lang: ReportLang, env: &Value, cases: &[Value], unit: &str, kind: Option<&str>) -> Row {
    let node = text(env, "node").unwrap_or_default();
    let mut cells = vec![Cell::text(node)];
    cells.extend(cases.iter().map(|c| {
        list(c, "nodes")
            .iter()
            .find(|n| text(n, "node") == Some(node))
            .and_then(|n| num(n, "temperature"))
            .map_or_else(Cell::empty, |v| number_in(v, unit))
    }));
    let cell = |key: &str| num(env, key).map_or_else(Cell::empty, |v| number_in(v, unit));
    cells.push(cell("min"));
    cells.push(cell("max"));
    cells.push(Cell::text(range_text(env, unit, kind)));
    cells.push(Cell::text(verdict_cell(lang, env)));
    Row {
        style: RowStyle::Body,
        cells,
    }
}

/// 許容範囲の文字（`下限 – 上限`。片方だけなら片方）。
fn range_text(env: &Value, unit: &str, kind: Option<&str>) -> String {
    let range = list(env, "ranges")
        .iter()
        .find(|r| text(r, "kind") == kind)
        .and_then(|r| get(r, "range"));
    let Some(range) = range else {
        return String::new();
    };
    let end = |key: &str| num(range, key).map(|v| number_in(v, unit).text);
    match (end("min"), end("max")) {
        (Some(lo), Some(hi)) => format!("{lo} – {hi}"),
        (Some(lo), None) => format!("≥ {lo}"),
        (None, Some(hi)) => format!("≤ {hi}"),
        (None, None) => String::new(),
    }
}

fn verdict_cell(lang: ReportLang, env: &Value) -> &'static str {
    get(env, "verdict")
        .and_then(|v| serde_json::from_value::<Verdict>(v.clone()).ok())
        .map_or("", |v| verdict_text(lang, v))
}

fn notes(ctx: &Ctx, output: &Value) -> Vec<String> {
    let q = |key: &str| {
        get(output, "margin")
            .and_then(|m| num(m, key))
            .map_or_else(String::new, |v| {
                super::column::display(v, "TemperatureDifference")
            })
    };
    let mut notes = vec![tr(
        ctx.lang,
        Text::ThermalNote,
        &[&q("uncertainty"), &q("warning_band")],
    )];
    notes.extend(footer(ctx));
    notes
}
