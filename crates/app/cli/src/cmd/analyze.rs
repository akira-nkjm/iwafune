//! `fune analyze`・`fune budget`

use crate::args::AnalyzeArgs;
use crate::output::{C, Out};
use fune_service::dto::{
    AnalysisKind, AnalysisRunStatus, BudgetRequest, BudgetView, RunAnalysisRequest, Verdict,
};
use fune_service::{Workspace, analysis, budget};

/// `fune analyze <id> [--case X] [--input in.json] [--ops ops.json]`
pub fn run(ws: &Workspace, a: &AnalyzeArgs, out: &Out) -> anyhow::Result<u8> {
    let input = match &a.input {
        Some(path) => Some(serde_json::from_str(&std::fs::read_to_string(path)?)?),
        None => None,
    };
    let ops = match &a.ops {
        Some(path) => super::ops::ops_from_file(path)?,
        None => Vec::new(),
    };
    let req = RunAnalysisRequest {
        case: a.case.clone(),
        input,
        input_patch: None,
        ops,
    };
    let r = analysis::run_analysis(ws, &a.target, &req)?;
    if out.json {
        out.json(&r)?;
        return Ok(u8::from(r.status != AnalysisRunStatus::Ok));
    }
    match r.status {
        AnalysisRunStatus::Ok => out.ok(format!("{} を実行しました", r.id)),
        AnalysisRunStatus::MissingInputs => out.fail(format!(
            "{} の入力が足りません（{} 件）",
            r.id,
            r.missing.len()
        )),
        AnalysisRunStatus::Failed => out.fail(format!(
            "{} が失敗しました: {}",
            r.id,
            r.error.clone().unwrap_or_default()
        )),
    }
    if !r.missing.is_empty() {
        let mut t = out.table(&["足りない入力", "内容"]);
        for m in &r.missing {
            t.row(vec![C::Strong(m.path.clone()), C::Text(m.reason.clone())]);
        }
        t.print();
    }
    print_checks(&r.checks, out);
    if r.status == AnalysisRunStatus::Ok {
        println!("{}", out.dim("全体の結果は --json で出せます"));
    }
    Ok(u8::from(r.status != AnalysisRunStatus::Ok))
}

/// 出力の `checks`（どの解析も同じ形）を表にする。モードごとなど同じ項目が並ぶので、対象の列を付ける。
fn print_checks(checks: &[fune_service::dto::CheckView], out: &Out) {
    if checks.is_empty() {
        return;
    }
    let mut t = out.table(&["判定", "対象", "項目", "内容"]);
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
            C::Text(c.label.clone()),
            C::Dim(c.message.clone()),
        ]);
    }
    t.print();
}

/// `fune analyze ls`（プロジェクトが無くても使える）。
pub fn ls(out: &Out) -> anyhow::Result<u8> {
    let entries = analysis::analysis_catalog()?;
    if out.json {
        out.json(&entries)?;
        return Ok(0);
    }
    let mut t = out.table(&["ID", "名前", "種類", "入力"]);
    for e in &entries {
        let kind = match e.info.kind {
            AnalysisKind::Steady => "定常",
            AnalysisKind::TimeSeries => "時系列",
        };
        t.row(vec![
            C::Strong(e.info.id.clone()),
            C::Text(e.info.title.clone()),
            C::Text(kind.to_owned()),
            C::Verdict(
                if e.projectable {
                    "設計から組立"
                } else {
                    "未対応"
                }
                .to_owned(),
                e.projectable,
            ),
        ]);
    }
    t.print();
    Ok(0)
}

/// `fune budget mass|power|data [--ops ops.json]`
pub fn budget(
    ws: &Workspace,
    kind: &str,
    ops: Option<&std::path::Path>,
    out: &Out,
) -> anyhow::Result<u8> {
    let req = BudgetRequest {
        ops: match ops {
            Some(path) => super::ops::ops_from_file(path)?,
            None => Vec::new(),
        },
    };
    let b = budget::budget(ws, budget::parse_kind(kind)?, &req)?;
    if out.json {
        out.json(&b)?;
        return Ok(0);
    }
    print_budget(&b, out);
    Ok(u8::from(b.verdict == Verdict::Violation))
}

fn verdict_cell(v: Verdict) -> C {
    match v {
        Verdict::Ok => C::Verdict("✔".into(), true),
        Verdict::Warning => C::Text("⚠".into()),
        Verdict::Violation => C::Verdict("✖".into(), false),
    }
}

fn print_budget(b: &BudgetView, out: &Out) {
    let mode = b
        .mode
        .as_ref()
        .map(|m| format!("  モード {m}"))
        .unwrap_or_default();
    out.heading(&format!("{}のバジェット", b.label));
    println!("{}", out.dim(format!("{}{mode}", b.analysis)));
    let mut t = out
        .table(&["", "行", "現状", "成長余裕込み", "配分", "余裕"])
        .numeric(&[2, 3, 4, 5]);
    let opt = |q: &Option<fune_service::dto::QuantityView>| {
        q.as_ref()
            .map_or(C::Dim("—".into()), |q| C::Num(q.display.clone()))
    };
    for r in &b.rows {
        t.row(vec![
            verdict_cell(r.verdict),
            C::Strong(r.label.clone()),
            C::Num(r.current.display.clone()),
            C::Num(r.with_growth.display.clone()),
            opt(&r.allocated),
            opt(&r.margin),
        ]);
    }
    t.print();
    let limit = b
        .limit
        .as_ref()
        .map_or("—".to_owned(), |q| q.display.clone());
    let margin = b
        .margin
        .as_ref()
        .map_or("—".to_owned(), |q| q.display.clone());
    let line = format!(
        "合計 {}  上限 {limit}（システムマージン {}）  余裕 {margin}",
        b.total.display, b.system_margin_display.display
    );
    match b.verdict {
        Verdict::Ok => out.ok(line),
        Verdict::Warning => out.warn(line),
        Verdict::Violation => out.fail(line),
    }
    for n in &b.notes {
        out.warn(n);
    }
}
