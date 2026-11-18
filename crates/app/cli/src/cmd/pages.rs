//! `fune power-page`・`fune mode view`（`GET /power_page`・`POST /system_modes/{system}` と同じ DTO を表にする）。
//! 値・合計・判定は service が作ったものをそのまま並べる（CLI は計算しない）。

use super::analyze::{print_missing, verdict_cell};
use crate::output::{C, Out};
use fune_service::dto::{
    AnalysisRunStatus, ModeSummaryField, ModeSummaryLabel, ModeValuesView, PartViewRequest,
    PowerBudgetView, QuantityView, SystemModeView, SystemModesView, Verdict,
};
use fune_service::{Workspace, power_page, system_modes};

fn dash(q: Option<&QuantityView>) -> C {
    q.map_or(C::Dim("—".into()), |q| C::Num(q.display.clone()))
}

fn supply(b: Option<&PowerBudgetView>) -> C {
    dash(b.map(|b| &b.supply))
}

/// `fune power-page [--case ID]`。違反のモードか、走らなかった解析があれば 1 で終わる。
pub fn power_page(ws: &Workspace, case: Option<&str>, out: &Out) -> anyhow::Result<u8> {
    let page = power_page::power_page(ws, case)?;
    let failed = page
        .modes
        .iter()
        .any(|m| m.verdict == Some(Verdict::Violation))
        || page
            .analyses
            .iter()
            .any(|a| a.result.status != AnalysisRunStatus::Ok);
    if out.json {
        out.json(&page)?;
        return Ok(u8::from(failed));
    }
    out.heading("Power page");
    let mut t = out
        .table(&["", "Mode", "Sunlit supply", "Eclipse supply"])
        .numeric(&[2, 3]);
    for m in &page.modes {
        t.row(vec![
            m.verdict.map_or(C::Dim(String::new()), verdict_cell),
            C::Strong(m.id.clone()),
            supply(m.sunlit.as_ref()),
            supply(m.eclipse.as_ref()),
        ]);
    }
    t.print();
    let mut t = out.table(&["Analysis", "Status", "Detail"]);
    for a in &page.analyses {
        let r = &a.result;
        let (status, ok) = match r.status {
            AnalysisRunStatus::Ok => ("ok", true),
            AnalysisRunStatus::MissingInputs => ("missing inputs", false),
            AnalysisRunStatus::Failed => ("failed", false),
        };
        let detail = match r.status {
            AnalysisRunStatus::Ok => String::new(),
            AnalysisRunStatus::MissingInputs => format!("{} missing", r.missing.len()),
            AnalysisRunStatus::Failed => r.error.clone().unwrap_or_default(),
        };
        t.row(vec![
            C::Strong(r.id.clone()),
            C::Verdict(status.to_owned(), ok),
            C::Dim(detail),
        ]);
    }
    t.print();
    println!("{}", out.dim("Use --json for the full page"));
    Ok(u8::from(failed))
}

/// `fune mode view <SYSTEM> [--ops ops.json]`。入力が足りないか違反の判定があれば 1 で終わる。
pub fn mode_view(
    ws: &Workspace,
    system: &str,
    ops: Option<&std::path::Path>,
    out: &Out,
) -> anyhow::Result<u8> {
    let req = PartViewRequest {
        ops: match ops {
            Some(path) => super::ops::ops_from_file(path)?,
            None => Vec::new(),
        },
    };
    let view = system_modes::system_modes(ws, system_modes::parse_system(system)?, &req)?;
    let failed = !view.missing.is_empty() || has_violation(&view);
    if out.json {
        out.json(&view)?;
        return Ok(u8::from(failed));
    }
    for eclipse in [false, true] {
        print_values(&view, eclipse, out);
    }
    print_aocs(&view, out);
    print_missing(&view.missing, out);
    Ok(u8::from(failed))
}

fn has_violation(view: &SystemModesView) -> bool {
    view.modes.iter().any(|m| {
        [&m.sunlit, &m.eclipse]
            .iter()
            .any(|v| v.verdict == Some(Verdict::Violation))
            || m.aocs.as_ref().is_some_and(|a| {
                [&a.rw, &a.unloading, &a.sensor_fov]
                    .iter()
                    .any(|s| s.verdict == Some(Verdict::Violation))
            })
    })
}

fn label_text(label: ModeSummaryLabel) -> &'static str {
    match label {
        ModeSummaryLabel::Power => "Total (with growth margin)",
        ModeSummaryLabel::DataRate => "Generation rate (before compression and packetization)",
        ModeSummaryLabel::Heat => "Internal heat dissipation",
        ModeSummaryLabel::Available => "Available",
        ModeSummaryLabel::Margin => "Margin (after system reserve)",
    }
}

fn values_of(m: &SystemModeView, eclipse: bool) -> &ModeValuesView {
    if eclipse { &m.eclipse } else { &m.sunlit }
}

/// 部品 × モードの値の表と、足もとの合計・収支の行（日照中・蝕中のどちらか）。
fn print_values(view: &SystemModesView, eclipse: bool, out: &Out) {
    if view.summary.is_empty() {
        return;
    }
    out.heading(if eclipse { "Eclipse" } else { "Sunlit" });
    let mut headers = vec!["Part"];
    headers.extend(view.modes.iter().map(|m| m.id.as_str()));
    let numeric: Vec<usize> = (1..headers.len()).collect();
    let mut t = out.table(&headers).numeric(&numeric);
    let parts: std::collections::BTreeSet<&String> = view
        .modes
        .iter()
        .flat_map(|m| values_of(m, eclipse).parts.keys())
        .collect();
    for part in parts {
        let mut cells = vec![C::Strong(part.clone())];
        cells.extend(view.modes.iter().map(|m| {
            dash(
                values_of(m, eclipse)
                    .parts
                    .get(part)
                    .and_then(Option::as_ref),
            )
        }));
        t.row(cells);
    }
    for row in &view.summary {
        let mut cells = vec![C::Strong(label_text(row.label).to_owned())];
        cells.extend(view.modes.iter().map(|m| {
            let v = values_of(m, eclipse);
            let q = match row.field {
                ModeSummaryField::Total => &v.total,
                ModeSummaryField::Available => &v.available,
                ModeSummaryField::Margin => &v.margin,
            };
            match (row.verdict, v.verdict, q) {
                (true, Some(verdict), Some(q)) => {
                    let mark = if verdict == Verdict::Ok { "✔" } else { "⚠" };
                    C::Num(format!("{} {mark}", q.display))
                }
                _ => dash(q.as_ref()),
            }
        }));
        t.row(cells);
    }
    t.print();
}

/// AOCS のモードごとの姿勢則と、解析の状態。
fn print_aocs(view: &SystemModesView, out: &Out) {
    if view.modes.iter().all(|m| m.aocs.is_none()) {
        return;
    }
    let mut t = out.table(&["Mode", "Attitude law", "RW", "Unloading", "Sensor FOV"]);
    let state = |s: &fune_service::dto::AocsAnalysisState| match s.status {
        AnalysisRunStatus::Ok => C::Verdict(
            "ok".to_owned(),
            s.verdict.is_none_or(|v| v != Verdict::Violation),
        ),
        AnalysisRunStatus::MissingInputs => C::Verdict("missing inputs".to_owned(), false),
        AnalysisRunStatus::Failed => C::Verdict("failed".to_owned(), false),
    };
    for m in &view.modes {
        let Some(a) = &m.aocs else { continue };
        t.row(vec![
            C::Strong(m.id.clone()),
            C::Text(a.attitude.law.map_or("—", |l| l.as_str()).to_owned()),
            state(&a.rw),
            state(&a.unloading),
            state(&a.sensor_fov),
        ]);
    }
    t.print();
}
