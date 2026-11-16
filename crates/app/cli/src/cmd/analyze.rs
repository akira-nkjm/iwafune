//! `fune analyze`・`fune budget`

use crate::args::AnalyzeArgs;
use crate::output::{C, Out};
use fune_service::dto::{
    AnalysisInputSource, AnalysisKind, AnalysisRunStatus, BudgetPropellant, BudgetRequest,
    BudgetRow, BudgetUncounted, BudgetView, KpiView, MissingInput, QuantityView,
    RunAnalysisRequest, Verdict,
};
use fune_service::{Workspace, analysis, budget, query};

/// `fune analyze <id> [--case X] [--input in.json] [--ops ops.json]`
pub fn run(ws: &Workspace, a: &AnalyzeArgs, out: &Out) -> anyhow::Result<u8> {
    let source = match &a.input {
        Some(path) => AnalysisInputSource::Given {
            input: serde_json::from_str(&std::fs::read_to_string(path)?)?,
        },
        None => AnalysisInputSource::Design,
    };
    let ops = match &a.ops {
        Some(path) => super::ops::ops_from_file(path)?,
        None => Vec::new(),
    };
    let req = RunAnalysisRequest {
        case: a.case.clone(),
        source,
        ops,
    };
    let r = analysis::run_analysis(ws, &a.target, &req)?;
    if out.json {
        out.json(&r)?;
        return Ok(u8::from(r.status != AnalysisRunStatus::Ok));
    }
    match r.status {
        AnalysisRunStatus::Ok => out.ok(format!("{} ran", r.id)),
        AnalysisRunStatus::MissingInputs => {
            out.fail(format!("{}: missing inputs ({})", r.id, r.missing.len()))
        }
        AnalysisRunStatus::Failed => out.fail(format!(
            "{} failed: {}",
            r.id,
            r.error.clone().unwrap_or_default()
        )),
    }
    print_missing(&r.missing, out);
    print_kpis(&r.kpis, out);
    print_checks(&r.checks, out);
    if let Some(v) = &r.space_weather {
        let fetched = v.fetched_at.as_deref().unwrap_or("no fetch time recorded");
        println!(
            "{}",
            out.dim(format!(
                "Space weather table: {} to {} (fetched {fetched}, SHA-256 {})",
                v.first_date, v.last_date, v.sha256
            ))
        );
    }
    super::output_tables::print(&r, out);
    if r.status == AnalysisRunStatus::Ok {
        println!("{}", out.dim("Use --json for the full result"));
    }
    Ok(u8::from(r.status != AnalysisRunStatus::Ok))
}

/// 足りない入力の表（解析とバジェットで同じ形）。
fn print_missing(missing: &[MissingInput], out: &Out) {
    if missing.is_empty() {
        return;
    }
    let mut t = out.table(&["Missing input", "Reason"]);
    for m in missing {
        t.row(vec![C::Strong(m.path.clone()), C::Text(m.reason.clone())]);
    }
    t.print();
}

/// KPI（代表値・判定。選び方は service の `analysis::kpi`）を表にする。判定を数える KPI は数を出す。
fn print_kpis(kpis: &[KpiView], out: &Out) {
    if kpis.is_empty() {
        return;
    }
    let mut t = out.table(&["", "KPI", "Value"]).numeric(&[2]);
    for k in kpis {
        let value = match (&k.value, k.tally) {
            (Some(v), _) => C::Num(v.display.clone()),
            (None, Some(t)) if t.flagged == 0 => C::Text(format!("All OK ({})", t.checked)),
            (None, Some(t)) => C::Text(format!("{} / {} need review", t.flagged, t.checked)),
            (None, None) => C::Dim("—".into()),
        };
        let mark = k.verdict.map_or(C::Dim(String::new()), verdict_cell);
        t.row(vec![
            mark,
            C::Strong(query::english(&k.label, Some(&k.label_en)).to_owned()),
            value,
        ]);
    }
    t.print();
}

/// 出力の `checks`（どの解析も同じ形）を表にする。モードごとなど同じ項目が並ぶので、対象の列を付ける。
fn print_checks(checks: &[fune_service::dto::CheckView], out: &Out) {
    if checks.is_empty() {
        return;
    }
    let mut t = out.table(&["Verdict", "Target", "Item", "Detail"]);
    for c in checks {
        let verdict = c.verdict;
        let mark = match verdict {
            Verdict::Ok => C::Verdict("✔ ok".into(), true),
            Verdict::Warning => C::Text("⚠ warning".into()),
            _ => C::Verdict("✖ violation".into(), false),
        };
        t.row(vec![
            mark,
            C::Text(c.scope.clone().unwrap_or_default()),
            C::Text(query::english(&c.label, c.label_en.as_deref()).to_owned()),
            C::Dim(query::english(&c.message, c.message_en.as_deref()).to_owned()),
        ]);
    }
    t.print();
}

/// `fune analyze ls`（プロジェクトが無くても使える）。
pub fn ls(out: &Out) -> anyhow::Result<u8> {
    let entries = analysis::analysis_catalog()?;
    // 並びは解析の一覧（catalog）のまま: 系の決まった順、そのあとに前提条件の話題
    if out.json {
        out.json(&entries)?;
        return Ok(0);
    }
    let mut t = out.table(&["Area", "ID", "Name", "Kind", "Input"]);
    for e in &entries {
        let kind = kind_text(e.info.kind);
        t.row(vec![
            C::Text(fune_service::analysis::area_key(e.info.area).to_owned()),
            C::Strong(e.info.id.clone()),
            C::Text(query::english(&e.info.title, Some(&e.info.title_en)).to_owned()),
            C::Text(kind.to_owned()),
            C::Verdict(
                if e.projectable {
                    "built from design"
                } else {
                    "unsupported"
                }
                .to_owned(),
                e.projectable,
            ),
        ]);
    }
    t.print();
    Ok(0)
}

/// `fune budget mass|power|data [--case HOT] [--ops ops.json]`。入力が足りないか判定（質量は乾燥と推薬込み）が違反なら 1 で終わる。
pub fn budget(
    ws: &Workspace,
    kind: &str,
    case: Option<&str>,
    ops: Option<&std::path::Path>,
    out: &Out,
) -> anyhow::Result<u8> {
    let req = BudgetRequest {
        case: case.map(str::to_owned),
        ops: match ops {
            Some(path) => super::ops::ops_from_file(path)?,
            None => Vec::new(),
        },
    };
    let b = budget::budget(ws, budget::parse_kind(kind)?, &req)?;
    let failed = budget::failure::fails(&b);
    if out.json {
        out.json(&b)?;
        return Ok(u8::from(failed));
    }
    print_budget(&b, out);
    Ok(u8::from(failed))
}

/// 配分の行の名前（「その他」の行は service が null で返すので、ここで文言を付ける）。
fn row_name(r: &BudgetRow) -> String {
    r.label
        .as_deref()
        .map(|l| query::english(l, r.label_en.as_deref()).to_owned())
        .or_else(|| r.node.clone())
        .unwrap_or_else(|| "(other)".to_owned())
}

/// 解析の種類の言い方（`analyze ls` と `flow`）。
pub(super) fn kind_text(kind: AnalysisKind) -> &'static str {
    match kind {
        AnalysisKind::Steady => "steady",
        AnalysisKind::TimeSeries => "time series",
    }
}

/// 判定の印のセル（KPI・バジェット・フロー図）。
pub(super) fn verdict_cell(v: Verdict) -> C {
    match v {
        Verdict::Ok => C::Verdict("✔".into(), true),
        Verdict::Warning => C::Text("⚠".into()),
        Verdict::Violation => C::Verdict("✖".into(), false),
    }
}

fn opt_cell(q: &Option<QuantityView>) -> C {
    q.as_ref()
        .map_or(C::Dim("—".into()), |q| C::Num(q.display.clone()))
}

fn opt_text(q: &Option<QuantityView>) -> String {
    q.as_ref().map_or("—".to_owned(), |q| q.display.clone())
}

/// 判定に合わせて 1 行を出す（判定が無ければ OK の色）。
fn verdict_line(v: Option<Verdict>, line: String, out: &Out) {
    match v.unwrap_or(Verdict::Ok) {
        Verdict::Ok => out.ok(line),
        Verdict::Warning => out.warn(line),
        Verdict::Violation => out.fail(line),
    }
}

fn print_budget(b: &BudgetView, out: &Out) {
    let case = b
        .case
        .as_ref()
        .map(|c| format!("  case {c}"))
        .unwrap_or_default();
    let mode = b
        .mode
        .as_ref()
        .map(|m| format!("  mode {m}"))
        .unwrap_or_default();
    out.heading(&format!(
        "{} budget",
        query::english(&b.label, b.label_en.as_deref())
    ));
    println!("{}", out.dim(format!("{}{case}{mode}", b.analysis)));
    let Some(total) = &b.total else {
        out.fail(format!("Missing inputs ({})", b.missing.len()));
        print_missing(&b.missing, out);
        return;
    };
    let mut t = out
        .table(&["", "Row", "Current", "With growth", "Allocated", "Margin"])
        .numeric(&[2, 3, 4, 5]);
    for r in &b.rows {
        t.row(vec![
            verdict_cell(r.verdict),
            C::Strong(row_name(r)),
            C::Num(r.current.display.clone()),
            C::Num(r.with_growth.display.clone()),
            opt_cell(&r.allocated),
            opt_cell(&r.margin),
        ]);
    }
    if let Some(BudgetPropellant::Counted { loaded, .. }) = &b.propellant {
        // 推薬は成長余裕を掛けず、配分の行でもない
        t.row(vec![
            C::Text(String::new()),
            C::Strong("Propellant".into()),
            C::Num(loaded.display.clone()),
            C::Num(loaded.display.clone()),
            C::Dim("—".into()),
            C::Dim("—".into()),
        ]);
    }
    t.print();
    let name = if b.propellant.is_some() {
        "Dry total"
    } else {
        "Total"
    };
    let line = format!(
        "{name} {}  limit {} (system margin {})  margin {}",
        total.display,
        opt_text(&b.limit),
        b.system_margin.display,
        opt_text(&b.margin)
    );
    verdict_line(b.verdict, line, out);
    if let Some(p) = &b.propellant {
        print_propellant(p, out);
    }
    print_uncounted(&b.uncounted, out);
    for n in &b.notes {
        out.warn(n);
    }
}

/// 値が無くて合計に数えなかった部品（0 とはみなしていない）。電力は数えなかったモードも添える。
fn print_uncounted(list: &[BudgetUncounted], out: &Out) {
    if list.is_empty() {
        return;
    }
    let names: Vec<String> = list
        .iter()
        .map(|u| {
            let name = if u.label == u.part {
                u.part.clone()
            } else {
                format!("{} ({})", u.part, u.label)
            };
            if u.modes.is_empty() {
                name
            } else {
                format!("{name} [{}]", u.modes.join(", "))
            }
        })
        .collect();
    out.warn(format!(
        "Parts not counted in the total because they have no value ({}; not treated as 0): {}",
        list.len(),
        names.join(", ")
    ));
}

/// 推薬込みの合計か、推薬を数えなかった理由（文言はここで付ける）。
fn print_propellant(p: &BudgetPropellant, out: &Out) {
    match p {
        BudgetPropellant::Counted { analysis, wet, .. } => verdict_line(
            wet.verdict,
            format!(
                "Total with propellant {}  margin {}  (propellant from {analysis})",
                wet.total.display,
                opt_text(&wet.margin)
            ),
            out,
        ),
        BudgetPropellant::NoPropulsion => {
            println!(
                "{}",
                out.dim("Propellant not counted: no propulsion (maneuvers or propulsion parts)")
            );
        }
        BudgetPropellant::Missing { missing } => {
            out.warn(format!(
                "Propellant not counted: missing propulsion inputs ({})",
                missing.len()
            ));
            print_missing(missing, out);
        }
        BudgetPropellant::Failed { message } => {
            out.warn(format!(
                "Propellant not counted: the propulsion analysis failed: {message}"
            ));
        }
    }
}
