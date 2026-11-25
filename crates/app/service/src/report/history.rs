//! 時間変化の図（SVG）。温度（節点ごと・許容範囲の破線・蝕の影）と、発電・消費電力と電池の充電状態（蝕の影）。
//! 系列は解析の時系列の型付きの出力、蝕の区間は `power.orbit_generation` / `power.energy_balance` の `eclipses` をそのまま使う。

use super::column::{label_of, pick_unit_largest};
use super::common::{
    Counted, case_label, effective, footer, note_unavailable, pick_label, remember_case, run,
};
use super::{Ctx, Out};
use crate::analysis::channels::display_series;
use crate::dto::{DisplayChannel, ReportItem};
use crate::error::Result;
use fune_report::chart::{self, Chart, Limit, Panel, Series};
use fune_report::files::figure_path;
use fune_report::labels::{Text, t, tr};
use fune_report::model::Figure;
use sat_analysis::power::{EnergyBalance, OrbitGeneration};
use sat_analysis::spec::ChannelKind;
use sat_analysis::thermal::range::ThermalRangeKind;
use sat_analysis::thermal::transient::{ThermalTransient, ThermalTransientNodeResult};
use sat_astro::timeline::Window;
use sat_units::parse_unit;

/// 温度の時間変化（解析ケースごとに 1 枚）。
pub(super) fn thermal(ctx: &Ctx, case: Option<&str>, out: &mut Out) -> Result<()> {
    let run = run::<ThermalTransient>(ctx, case)?;
    let case = effective(&run.result, case);
    remember_case(out, &case);
    super::values::kpis(ctx, out, &run.result, case.as_deref());
    let caption = tr(
        ctx.lang,
        Text::ThermalHistoryCaption,
        &[&case_label(ctx, case.as_deref())],
    );
    let output = match run.counted()? {
        Counted::Yes { output, .. } => output,
        Counted::No(reason) => {
            unavailable(ctx, out, ReportItem::ThermalHistory, case, caption, &reason);
            return Ok(());
        }
    };
    let series = &output.series;
    let t_si = &series.t;
    let eclipses = eclipse_intervals(ctx, case.as_deref())?;
    let tu = time_unit(t_si);
    let conv = |u: &str, si: f64| parse_unit(u).map_or(si, |u| u.from_si(si));
    let x: Vec<f64> = t_si.iter().map(|&v| conv(&tu, v)).collect();
    let mut series_list = Vec::new();
    for ch in &series.channels {
        if ch.kind != ChannelKind::NodeTemperature {
            continue;
        }
        let y = ch.values.iter().map(|&v| conv("degC", v)).collect();
        series_list.push(Series {
            label: ch.subject.clone().unwrap_or_else(|| ch.key.clone()),
            x: x.clone(),
            y,
        });
    }
    let panel = Panel {
        y_label: format!("{} [{}]", t(ctx.lang, Text::Temperature), label_of("degC")),
        series: series_list,
        limits: temperature_limits(ctx, &output.nodes),
    };
    let chart = Chart {
        x_label: format!("{} [{}]", t(ctx.lang, Text::Time), label_of(&tu)),
        panels: vec![panel],
        shade: shade(&eclipses, &tu, t_si.last().copied()),
        shade_label: t(ctx.lang, Text::Eclipse).to_owned(),
    };
    add_figure(ctx, out, ReportItem::ThermalHistory, case, caption, &chart);
    Ok(())
}

/// 発電・消費電力と電池の充電状態の時間変化（解析ケースごとに 1 枚）。
pub(super) fn power(ctx: &Ctx, case: Option<&str>, out: &mut Out) -> Result<()> {
    let run = run::<EnergyBalance>(ctx, case)?;
    let case = effective(&run.result, case);
    remember_case(out, &case);
    super::values::kpis(ctx, out, &run.result, case.as_deref());
    let caption = tr(
        ctx.lang,
        Text::PowerHistoryCaption,
        &[&case_label(ctx, case.as_deref())],
    );
    let output = match run.counted()? {
        Counted::Yes { output, .. } => output,
        Counted::No(reason) => {
            unavailable(ctx, out, ReportItem::PowerHistory, case, caption, &reason);
            return Ok(());
        }
    };
    let Some(entry) = output.cases.first() else {
        let reason = "no battery case in the result";
        unavailable(ctx, out, ReportItem::PowerHistory, case, caption, reason);
        return Ok(());
    };
    let series = display_series(&entry.series);
    let t_si = &series.t;
    let tu = time_unit(t_si);
    let x: Vec<f64> = t_si
        .iter()
        .map(|&v| parse_unit(&tu).map_or(v, |u| u.from_si(v)))
        .collect();
    let channel = |kind: ChannelKind| series.channels.iter().find(|c| c.channel.kind == kind);
    let powers: Vec<&DisplayChannel> = [ChannelKind::Generation, ChannelKind::Load]
        .into_iter()
        .filter_map(channel)
        .collect();
    let pu = pick_unit_largest(
        "Power",
        powers.iter().flat_map(|c| c.channel.values.iter().copied()),
    );
    let mut panels = vec![Panel {
        y_label: format!("{} [{}]", t(ctx.lang, Text::Power), label_of(&pu)),
        series: powers.iter().map(|c| to_series(ctx, c, &x, &pu)).collect(),
        limits: Vec::new(),
    }];
    if let Some(soc) = channel(ChannelKind::Soc) {
        panels.push(Panel {
            y_label: format!("{} [%]", t(ctx.lang, Text::StateOfCharge)),
            series: vec![to_series(ctx, soc, &x, "%")],
            limits: soc_limit(ctx, output.min_allowed_soc.value()),
        });
    }
    let chart = Chart {
        x_label: format!("{} [{}]", t(ctx.lang, Text::Time), label_of(&tu)),
        panels,
        shade: shade(&output.eclipses, &tu, t_si.last().copied()),
        shade_label: t(ctx.lang, Text::Eclipse).to_owned(),
    };
    add_figure(ctx, out, ReportItem::PowerHistory, case, caption, &chart);
    Ok(())
}

/// 横軸の単位（経過時間の最大に合う単位）。
fn time_unit(t_si: &[f64]) -> String {
    pick_unit_largest("Time", t_si.iter().copied())
}

fn to_series(ctx: &Ctx, channel: &DisplayChannel, x: &[f64], unit: &str) -> Series {
    let from = |v: f64| parse_unit(unit).map_or(v, |u| u.from_si(v));
    let label = pick_label(ctx.lang, &channel.label, Some(&channel.label_en));
    Series {
        label,
        x: x.to_vec(),
        y: channel.channel.values.iter().map(|&v| from(v)).collect(),
    }
}

/// 蝕の区間（秒）を表示単位に直し、系列の最後より先は切る。
fn shade(eclipses: &[Window], unit: &str, end_si: Option<f64>) -> Vec<(f64, f64)> {
    let from = |v: f64| parse_unit(unit).map_or(v, |u| u.from_si(v));
    let end = end_si.unwrap_or(f64::INFINITY);
    eclipses
        .iter()
        .map(|e| (e.start.value(), e.end.value()))
        .filter(|(a, _)| *a < end)
        .map(|(a, b)| (from(a), from(b.min(end))))
        .collect()
}

/// 同じケースの発電の解析から蝕の区間を取る（温度の解析は蝕の区間を出さない）。取れなければ影なし。
fn eclipse_intervals(ctx: &Ctx, case: Option<&str>) -> Result<Vec<Window>> {
    Ok(match run::<OrbitGeneration>(ctx, case)?.counted()? {
        Counted::Yes { output, .. } => output.eclipses.clone(),
        Counted::No(_) => Vec::new(),
    })
}

/// 節点の許容範囲（動作の範囲。同じ範囲の節点はまとめて 1 本）。
fn temperature_limits(ctx: &Ctx, nodes: &[ThermalTransientNodeResult]) -> Vec<Limit> {
    let mut groups: Vec<(f64, f64, Vec<&str>)> = Vec::new();
    for n in nodes {
        let pick = n
            .ranges
            .iter()
            .find(|r| r.kind == ThermalRangeKind::Operating)
            .or_else(|| n.ranges.first());
        let Some(range) = pick.map(|r| &r.range) else {
            continue;
        };
        let (lo, hi) = (range.min.value(), range.max.value());
        match groups.iter_mut().find(|g| g.0 == lo && g.1 == hi) {
            Some(g) => g.2.push(&n.node),
            None => groups.push((lo, hi, vec![&n.node])),
        }
    }
    groups
        .into_iter()
        .map(|(lo, hi, names)| {
            let c = |v: f64| parse_unit("degC").map_or(v, |u| u.from_si(v));
            let span = format!(
                "{} – {} {}",
                super::column::number_in(lo, "degC").text,
                super::column::number_in(hi, "degC").text,
                label_of("degC")
            );
            Limit {
                label: tr(
                    ctx.lang,
                    Text::AllowableLegend,
                    &[
                        t(ctx.lang, Text::RangeOperating),
                        &format!("{} [{span}]", names.join(", ")),
                    ],
                ),
                lo: Some(c(lo)),
                hi: Some(c(hi)),
            }
        })
        .collect()
}

/// 電池の充電状態の下限（DoD の上限から求めた解析の値）。
fn soc_limit(ctx: &Ctx, min_allowed_soc: f64) -> Vec<Limit> {
    vec![Limit {
        label: tr(
            ctx.lang,
            Text::AllowableLegend,
            &[
                t(ctx.lang, Text::StateOfCharge),
                &format!(
                    "≥ {} %",
                    super::column::number_in(min_allowed_soc, "%").text
                ),
            ],
        ),
        lo: parse_unit("%").ok().map(|u| u.from_si(min_allowed_soc)),
        hi: None,
    }]
}

/// 図を SVG にして出力に足す。
fn add_figure(
    ctx: &Ctx,
    out: &mut Out,
    item: ReportItem,
    case: Option<String>,
    caption: String,
    chart: &Chart,
) {
    let file = figure_path(item.as_str(), case.as_deref());
    out.svgs.push((file.clone(), chart::render(chart)));
    out.figures.push(Figure {
        item,
        case,
        caption,
        file: Some(file),
        x_label: chart.x_label.clone(),
        panels: chart::summarize(chart),
        shaded: chart.shade.len(),
        notes: footer(ctx),
        unavailable: None,
    });
}

/// 数えられなかった図（理由つきの空の図）。
fn unavailable(
    ctx: &Ctx,
    out: &mut Out,
    item: ReportItem,
    case: Option<String>,
    caption: String,
    reason: &str,
) {
    note_unavailable(out, item, case.as_deref(), reason);
    out.figures.push(Figure {
        item,
        case,
        caption,
        file: None,
        x_label: String::new(),
        panels: Vec::new(),
        shaded: 0,
        notes: footer(ctx),
        unavailable: Some(tr(ctx.lang, Text::NotAvailable, &[reason])),
    });
}
