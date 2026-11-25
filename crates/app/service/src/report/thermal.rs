//! 温度の結果表。節点ごとに、定常解析のケース（高温・低温など）の温度を列に並べ、全ケースの最低・最高、許容範囲、判定を添える。
//! 値と判定は `thermal.steady` の型付きの出力（`ThermalSteadyOutput`。`envelope` の判定）をそのまま使う。

use super::column::{label_of, number_in, pick_unit_smallest};
use super::common::{
    Counted, case_label, effective, empty_table, footer, note_unavailable, remember_case, run,
    verdict_text,
};
use super::{Ctx, Out};
use crate::dto::{ReportItem, ReportLang};
use crate::error::Result;
use fune_report::labels::{Text, t, tr};
use fune_report::model::{Align, Cell, Column, Row, RowStyle, Table};
use sat_analysis::thermal::range::ThermalRangeKind;
use sat_analysis::thermal::steady::{
    ThermalSteady, ThermalSteadyCaseResult, ThermalSteadyEnvelope, ThermalSteadyOutput,
};

pub(super) fn build(ctx: &Ctx, case: Option<&str>, out: &mut Out) -> Result<()> {
    let run = run::<ThermalSteady>(ctx, case)?;
    let case = effective(&run.result, case);
    remember_case(out, &case);
    super::values::kpis(ctx, out, &run.result, case.as_deref());
    let caption = tr(
        ctx.lang,
        Text::ThermalCaption,
        &[&case_label(ctx, case.as_deref())],
    );
    let output = match run.counted()? {
        Counted::Yes { output, .. } => output,
        Counted::No(reason) => {
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
    let cases = &output.cases;
    let envelope = &output.envelope;
    let temps = cases
        .iter()
        .flat_map(|c| &c.nodes)
        .map(|n| n.temperature.value())
        .chain(envelope.iter().map(|e| e.min.value()))
        .chain(envelope.iter().map(|e| e.max.value()));
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
fn range_kind(envelope: &[ThermalSteadyEnvelope]) -> Option<ThermalRangeKind> {
    let kinds: Vec<ThermalRangeKind> = envelope
        .iter()
        .flat_map(|e| &e.ranges)
        .map(|r| r.kind)
        .collect();
    kinds
        .iter()
        .find(|k| **k == ThermalRangeKind::Operating)
        .or_else(|| kinds.first())
        .copied()
}

fn kind_label(lang: ReportLang, kind: Option<ThermalRangeKind>) -> &'static str {
    t(
        lang,
        match kind {
            Some(ThermalRangeKind::Storage) => Text::RangeStorage,
            Some(ThermalRangeKind::Performance) => Text::RangePerformance,
            Some(ThermalRangeKind::Operating) | None => Text::RangeOperating,
        },
    )
}

fn columns(
    ctx: &Ctx,
    cases: &[ThermalSteadyCaseResult],
    unit: &str,
    kind: Option<ThermalRangeKind>,
) -> Vec<Column> {
    let temp = Some(label_of(unit));
    let col = |header: String, unit: Option<String>, align| Column {
        header,
        unit,
        align,
    };
    let mut cols = vec![col(t(ctx.lang, Text::Node).to_owned(), None, Align::Left)];
    cols.extend(
        cases
            .iter()
            .map(|c| col(c.label.clone(), temp.clone(), Align::Right)),
    );
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

fn node_row(
    lang: ReportLang,
    env: &ThermalSteadyEnvelope,
    cases: &[ThermalSteadyCaseResult],
    unit: &str,
    kind: Option<ThermalRangeKind>,
) -> Row {
    let mut cells = vec![Cell::text(env.node.as_str())];
    cells.extend(cases.iter().map(|c| {
        c.nodes
            .iter()
            .find(|n| n.node == env.node)
            .map_or_else(Cell::empty, |n| number_in(n.temperature.value(), unit))
    }));
    cells.push(number_in(env.min.value(), unit));
    cells.push(number_in(env.max.value(), unit));
    cells.push(Cell::text(range_text(env, unit, kind)));
    cells.push(Cell::text(verdict_text(lang, env.verdict)));
    Row {
        style: RowStyle::Body,
        cells,
    }
}

/// 許容範囲の文字（`下限 – 上限`）。その種類の範囲が無ければ空。
fn range_text(env: &ThermalSteadyEnvelope, unit: &str, kind: Option<ThermalRangeKind>) -> String {
    let Some(range) = env
        .ranges
        .iter()
        .find(|r| Some(r.kind) == kind)
        .map(|r| &r.range)
    else {
        return String::new();
    };
    let end = |v: f64| number_in(v, unit).text;
    format!("{} – {}", end(range.min.value()), end(range.max.value()))
}

fn notes(ctx: &Ctx, output: &ThermalSteadyOutput) -> Vec<String> {
    let q = |v: f64| super::column::display(v, "TemperatureDifference");
    let mut notes = vec![tr(
        ctx.lang,
        Text::ThermalNote,
        &[
            &q(output.margin.uncertainty.value()),
            &q(output.margin.warning_band.value()),
        ],
    )];
    notes.extend(footer(ctx));
    notes
}
