//! 回線バジェットの入力表と結果表（回線 × 地上局の組ごとに 1 対）。値は `comm.link_budget` の入力と出力、判定は解析の判定。

use super::column::{auto_cell, display, label_of, number_in};
use super::common::{empty_table, footer, note_unavailable, run, verdict_text, why_not};
use super::json::{get, list, num, text};
use super::{Ctx, Out};
use crate::dto::{AnalysisRunResult, ReportItem, ReportLang};
use crate::error::Result;
use fune_report::labels::{Text, t, tr};
use fune_report::model::{Align, Cell, Column, Row, RowStyle, Table};
use sat_analysis::spec::Verdict;
use serde_json::Value;

pub(super) fn build(ctx: &Ctx, out: &mut Out) -> Result<()> {
    let result = run(ctx, "comm.link_budget", None)?;
    super::values::kpis(ctx, out, &result, None);
    let (Some(input), Some(output), None) = (&result.input, &result.output, why_not(&result))
    else {
        let reason = why_not(&result).unwrap_or_default();
        let caption = tr(ctx.lang, Text::LinkResultCaption, &[""]);
        out.tables.push(empty_table(
            ctx,
            ReportItem::LinkBudget,
            None,
            caption.trim_end().trim_end_matches(':').to_owned(),
            &reason,
        ));
        note_unavailable(out, ReportItem::LinkBudget, None, &reason);
        return Ok(());
    };
    let required = num(output, "required_margin_db");
    for (inp, res) in list(input, "links").iter().zip(list(output, "links")) {
        let name = text(inp, "name").unwrap_or_default();
        let station = text(inp, "station").unwrap_or_default();
        let subject = format!("{name}@{station}");
        let scenario = scenario(ctx.lang, inp, name, station);
        let verdict = verdict_of(&result, &subject);
        out.tables.push(input_table(ctx, inp, &subject, &scenario));
        out.tables.push(result_table(
            ctx, res, &subject, &scenario, required, verdict,
        ));
    }
    Ok(())
}

/// 題に入れる場面（回線・向き・地上局・周波数）。
fn scenario(lang: ReportLang, link: &Value, name: &str, station: &str) -> String {
    let direction = match text(link, "direction") {
        Some("uplink") => t(lang, Text::Uplink),
        _ => t(lang, Text::Downlink),
    };
    let band = get(link, "rf")
        .and_then(|rf| num(rf, "frequency"))
        .map(|f| format!(", {}", display(f, "Frequency")))
        .unwrap_or_default();
    format!("{name} ({direction}) @ {station}{band}")
}

fn verdict_of(result: &AnalysisRunResult, subject: &str) -> Option<Verdict> {
    let key = format!("margin:{subject}");
    result
        .checks
        .iter()
        .find(|c| c.key == key)
        .map(|c| c.verdict)
}

fn columns(ctx: &Ctx) -> Vec<Column> {
    let col = |text: Text, align| Column {
        header: t(ctx.lang, text).to_owned(),
        unit: None,
        align,
    };
    vec![
        col(Text::Parameter, Align::Left),
        col(Text::Value, Align::Right),
        col(Text::Unit, Align::Left),
    ]
}

/// （名前, 値, 次元）の行を表の行にする。値が無い行は出さない。次元が `unit:K` の形なら、その単位で表す（雑音温度は K）。
fn rows(ctx: &Ctx, specs: &[(Text, Option<f64>, &str)]) -> Vec<Row> {
    specs
        .iter()
        .filter_map(|(label, value, dim)| {
            let value = (*value)?;
            let (cell, unit) = match dim.strip_prefix("unit:") {
                Some(unit) => (number_in(value, unit), label_of(unit)),
                None => auto_cell(value, dim),
            };
            Some(Row {
                style: RowStyle::Body,
                cells: vec![Cell::text(t(ctx.lang, *label)), cell, Cell::text(unit)],
            })
        })
        .collect()
}

fn table(ctx: &Ctx, variant: &str, subject: &str, caption: String, rows: Vec<Row>) -> Table {
    let mut notes = Vec::new();
    if variant == "result" {
        notes.push(t(ctx.lang, Text::LinkNote).to_owned());
    }
    notes.extend(footer(ctx));
    Table {
        item: ReportItem::LinkBudget,
        case: None,
        subject: Some(subject.to_owned()),
        variant: Some(variant.to_owned()),
        caption,
        columns: columns(ctx),
        rows,
        notes,
        unavailable: None,
    }
}

fn input_table(ctx: &Ctx, link: &Value, subject: &str, scenario: &str) -> Table {
    let rf = get(link, "rf").unwrap_or(link);
    let r = |key: &str| num(rf, key);
    let l = |key: &str| num(link, key);
    let specs = [
        (Text::Frequency, r("frequency"), "Frequency"),
        (Text::TxPower, r("tx_power"), "Power"),
        (Text::TxGain, r("tx_gain_db"), "Decibel"),
        (Text::TxLoss, r("tx_loss_db"), "Decibel"),
        (Text::RxGain, r("rx_gain_db"), "Decibel"),
        (Text::RxLoss, r("rx_loss_db"), "Decibel"),
        (
            Text::NoiseTemperature,
            r("system_noise_temperature"),
            "unit:K",
        ),
        (Text::RxGOverT, r("rx_g_over_t_dbk"), "DecibelPerKelvin"),
        (Text::AtmosphericLoss, r("atmospheric_loss_db"), "Decibel"),
        (Text::PolarizationLoss, r("polarization_loss_db"), "Decibel"),
        (Text::PointingLoss, r("pointing_loss_db"), "Decibel"),
        (Text::OtherLoss, r("other_loss_db"), "Decibel"),
        (Text::DataRate, l("data_rate"), "DataRate"),
        (Text::RequiredEbN0, l("required_eb_n0_db"), "Decibel"),
        (Text::Altitude, l("altitude"), "Length"),
        (Text::Elevation, l("elevation"), "Angle"),
    ];
    let caption = tr(ctx.lang, Text::LinkInputCaption, &[scenario]);
    table(ctx, "input", subject, caption, rows(ctx, &specs))
}

fn result_table(
    ctx: &Ctx,
    res: &Value,
    subject: &str,
    scenario: &str,
    required: Option<f64>,
    verdict: Option<Verdict>,
) -> Table {
    let r = |key: &str| num(res, key);
    let specs = [
        (Text::SlantRange, r("slant_range"), "Length"),
        (Text::RangeMin, r("range_min"), "Length"),
        (Text::LightTime, r("light_time"), "Time"),
        (Text::Eirp, r("eirp_dbw"), "DecibelWatt"),
        (Text::Fspl, r("fspl_db"), "Decibel"),
        (Text::ExtraLoss, r("extra_loss_db"), "Decibel"),
        (Text::GOverT, r("g_over_t_dbk"), "DecibelPerKelvin"),
        (Text::CN0, r("c_over_n0_dbhz"), "DecibelHertz"),
        (Text::EbN0, r("eb_n0_db"), "Decibel"),
        (Text::RequiredEbN0, r("required_eb_n0_db"), "Decibel"),
        (Text::LinkMargin, r("margin_db"), "Decibel"),
        (Text::RequiredMargin, required, "Decibel"),
        (Text::MaxDataRate, r("max_data_rate"), "DataRate"),
        (Text::Pfd, r("pfd_dbw_m2"), "Decibel"),
        (Text::PfdLimit, r("pfd_limit_db"), "Decibel"),
        (Text::PfdMargin, r("pfd_margin_db"), "Decibel"),
    ];
    let mut body = rows(ctx, &specs);
    if let Some(v) = verdict {
        body.push(Row {
            style: RowStyle::Total,
            cells: vec![
                Cell::text(t(ctx.lang, Text::Verdict)),
                Cell::text(verdict_text(ctx.lang, v)),
                Cell::empty(),
            ],
        });
    }
    let caption = tr(ctx.lang, Text::LinkResultCaption, &[scenario]);
    table(ctx, "result", subject, caption, body)
}
