//! 回線バジェットの入力表と結果表（回線 × 地上局の組ごとに 1 対）。値は `comm.link_budget` の型付きの入力と出力、判定は解析の判定。

use super::column::{auto_cell, display, label_of, number_in};
use super::common::{Counted, empty_table, footer, note_unavailable, run, verdict_text};
use super::{Ctx, Out};
use crate::dto::{AnalysisRunResult, ReportItem, ReportLang};
use crate::error::Result;
use fune_report::labels::{Text, t, tr};
use fune_report::model::{Align, Cell, Column, Row, RowStyle, Table};
use sat_analysis::comm::link_budget::{
    LinkBudget, LinkBudgetDirection, LinkBudgetLink, LinkBudgetLinkResult,
};
use sat_analysis::spec::Verdict;
use sat_units::typed::{Decibel, DecibelPerKelvin};

pub(super) fn build(ctx: &Ctx, out: &mut Out) -> Result<()> {
    let run = run::<LinkBudget>(ctx, None)?;
    super::values::kpis(ctx, out, &run.result, None);
    let (input, output) = match run.counted()? {
        Counted::Yes { input, output } => (input, output),
        Counted::No(reason) => {
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
        }
    };
    for (inp, res) in input.links.iter().zip(&output.links) {
        let subject = format!("{}@{}", inp.name, inp.station);
        let scenario = scenario(ctx.lang, inp);
        let verdict = verdict_of(&run.result, &subject);
        out.tables.push(input_table(ctx, inp, &subject, &scenario));
        out.tables
            .push(result_table(ctx, res, &subject, &scenario, verdict));
    }
    Ok(())
}

/// 題に入れる場面（回線・向き・地上局・周波数）。
fn scenario(lang: ReportLang, link: &LinkBudgetLink) -> String {
    let direction = direction_label(lang, link.direction);
    let band = display(link.rf.frequency.value(), "Frequency");
    format!("{} ({direction}) @ {}, {band}", link.name, link.station)
}

/// 回線の向きの表示名。語彙は解析の表示名（`analysis::labels`）のもので、英語は文中に置くので小文字にする。
fn direction_label(lang: ReportLang, direction: LinkBudgetDirection) -> String {
    let l = crate::analysis::labels::link_direction(direction);
    match lang {
        ReportLang::Ja => l.label,
        ReportLang::En => l.label_en.to_lowercase(),
    }
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

fn input_table(ctx: &Ctx, link: &LinkBudgetLink, subject: &str, scenario: &str) -> Table {
    let rf = &link.rf;
    let db = |v: Option<Decibel>| v.map(Decibel::value);
    let specs = [
        (Text::Frequency, Some(rf.frequency.value()), "Frequency"),
        (Text::TxPower, Some(rf.tx_power.value()), "Power"),
        (Text::TxGain, Some(rf.tx_gain_db.value()), "Decibel"),
        (Text::TxLoss, db(rf.tx_loss_db), "Decibel"),
        (Text::RxGain, db(rf.rx_gain_db), "Decibel"),
        (Text::RxLoss, db(rf.rx_loss_db), "Decibel"),
        (
            Text::NoiseTemperature,
            rf.system_noise_temperature.map(|q| q.value()),
            "unit:K",
        ),
        (
            Text::RxGOverT,
            rf.rx_g_over_t_dbk.map(DecibelPerKelvin::value),
            "DecibelPerKelvin",
        ),
        (Text::AtmosphericLoss, db(rf.atmospheric_loss_db), "Decibel"),
        (
            Text::PolarizationLoss,
            db(rf.polarization_loss_db),
            "Decibel",
        ),
        (Text::PointingLoss, db(rf.pointing_loss_db), "Decibel"),
        (Text::OtherLoss, db(rf.other_loss_db), "Decibel"),
        (Text::DataRate, Some(link.data_rate.value()), "DataRate"),
        (
            Text::RequiredEbN0,
            Some(link.required_eb_n0_db.value()),
            "Decibel",
        ),
        (Text::Altitude, link.altitude.map(|q| q.value()), "Length"),
        (Text::Elevation, Some(link.elevation.value()), "Angle"),
    ];
    let caption = tr(ctx.lang, Text::LinkInputCaption, &[scenario]);
    table(ctx, "input", subject, caption, rows(ctx, &specs))
}

fn result_table(
    ctx: &Ctx,
    res: &LinkBudgetLinkResult,
    subject: &str,
    scenario: &str,
    verdict: Option<Verdict>,
) -> Table {
    let db = |v: Option<Decibel>| v.map(Decibel::value);
    let specs = [
        (Text::SlantRange, Some(res.slant_range.value()), "Length"),
        (Text::RangeMin, Some(res.range_min.value()), "Length"),
        (Text::LightTime, Some(res.light_time.value()), "Time"),
        (Text::Eirp, Some(res.eirp_dbw.value()), "DecibelWatt"),
        (Text::Fspl, Some(res.fspl_db.value()), "Decibel"),
        (Text::ExtraLoss, Some(res.extra_loss_db.value()), "Decibel"),
        (
            Text::GOverT,
            Some(res.g_over_t_dbk.value()),
            "DecibelPerKelvin",
        ),
        (Text::CN0, Some(res.c_over_n0_dbhz.value()), "DecibelHertz"),
        (Text::EbN0, Some(res.eb_n0_db.value()), "Decibel"),
        (
            Text::RequiredEbN0,
            Some(res.required_eb_n0_db.value()),
            "Decibel",
        ),
        (Text::LinkMargin, Some(res.margin_db.value()), "Decibel"),
        (
            Text::RequiredMargin,
            Some(res.required_margin_db.value()),
            "Decibel",
        ),
        (
            Text::MaxDataRate,
            Some(res.max_data_rate.value()),
            "DataRate",
        ),
        (Text::Pfd, db(res.pfd_dbw_m2), "Decibel"),
        (Text::PfdLimit, db(res.pfd_limit_db), "Decibel"),
        (Text::PfdMargin, db(res.pfd_margin_db), "Decibel"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_uses_the_analysis_vocabulary_in_lowercase_for_english() {
        assert_eq!(
            direction_label(ReportLang::En, LinkBudgetDirection::Uplink),
            "uplink"
        );
        assert_eq!(
            direction_label(ReportLang::Ja, LinkBudgetDirection::Downlink),
            "ダウンリンク"
        );
    }
}
