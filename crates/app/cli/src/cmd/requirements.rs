//! `fune requirement …`: 引数 → Op（`add|set|rm`。打った値を読むのは service の `edit::requirement_op`）と、
//! 読み取り（`ls`・`compliance`。HTTP の `GET /requirements`・`GET /requirements/compliance` と同じ関数）。

use super::common::{Built, Handled, verdict_cell};
use crate::args::{RequirementArgs, RequirementCmd};
use crate::output::{C, Out};
use fune_service::dto::{ComplianceCase, ComplianceRow, NotVerifiedReason, Op, RequirementText};
use fune_service::{Workspace, edit, query, requirements};

/// `fune requirement add|set|rm`
pub fn requirement(c: &RequirementCmd) -> Built<'_> {
    Ok(match c {
        RequirementCmd::Add(a) => Some((edit::requirement_op(&text(a), true)?, &a.edit)),
        RequirementCmd::Set(a) => Some((edit::requirement_op(&text(a), false)?, &a.edit)),
        RequirementCmd::Rm { id, edit } => Some((Op::RemoveRequirement { id: id.clone() }, edit)),
        RequirementCmd::Ls | RequirementCmd::Compliance { .. } => None,
    })
}

fn text(a: &RequirementArgs) -> RequirementText {
    RequirementText {
        id: a.id.clone(),
        statement: a.statement.clone(),
        statement_en: a.statement_en.clone(),
        label: a.label.clone(),
        quantity: a.quantity.clone(),
        source: a.source.clone(),
        kind: a.kind.clone(),
        value: a.value.clone(),
        tolerance: a.tolerance.clone(),
        min: a.min.clone(),
        max: a.max.clone(),
        all_cases: a.all_cases.clone(),
        cases: a.cases.clone(),
        analysis: a.analysis.clone(),
        check: a.check.clone(),
        budget: a.budget.clone(),
    }
}

/// `fune requirement ls|compliance`（読み取り）。
pub fn read(ws: &Workspace, cmd: &RequirementCmd, out: &Out) -> Handled {
    match cmd {
        RequirementCmd::Ls => ls(ws, out),
        RequirementCmd::Compliance { deny_unverified } => compliance(ws, *deny_unverified, out),
        _ => Ok(None),
    }
}

fn ls(ws: &Workspace, out: &Out) -> Handled {
    let list = query::requirements(ws)?;
    if out.json {
        out.json(&list)?;
        return Ok(Some(0));
    }
    let mut t = out.table(&[
        "ID",
        "Name",
        "Quantity",
        "Constraint",
        "Source",
        "Verified by",
    ]);
    for r in &list {
        let by = match (&r.verify.budget, &r.verify.analysis, &r.verify.check) {
            (Some(b), _, _) => format!("budget {b}"),
            (None, Some(a), Some(k)) => format!("{a} / {k}"),
            (None, Some(a), None) => a.clone(),
            _ => String::new(),
        };
        t.row(vec![
            C::Strong(r.id.clone()),
            C::Text(r.label.clone()),
            C::Text(r.quantity.clone()),
            C::Num(r.constraint.text.clone()),
            C::Text(r.source.clone().unwrap_or_default()),
            C::Dim(by),
        ]);
    }
    t.print();
    Ok(Some(0))
}

fn compliance(ws: &Workspace, deny_unverified: bool, out: &Out) -> Handled {
    let report = requirements::compliance(ws)?;
    let failed = requirements::fails(&report, deny_unverified);
    if out.json {
        out.json(&report)?;
        return Ok(Some(u8::from(failed)));
    }
    let mut t = out.table(&[
        "Verdict",
        "ID",
        "Requirement",
        "Constraint",
        "Cases",
        "Detail",
    ]);
    for r in &report.rows {
        t.row(vec![
            row_verdict(r),
            C::Strong(r.id.clone()),
            C::Text(query::english(&r.statement, r.statement_en.as_deref()).to_owned()),
            C::Num(r.constraint.text.clone()),
            C::Text(case_names(&r.cases)),
            C::Dim(detail(r)),
        ]);
    }
    t.print();
    let s = &report.summary;
    println!(
        "{} requirements: {} ok, {} warning, {} violation, {} not verified",
        s.total, s.ok, s.warning, s.violation, s.not_verified
    );
    Ok(Some(u8::from(failed)))
}

fn row_verdict(r: &ComplianceRow) -> C {
    match r.verdict {
        Some(v) => verdict_cell(v),
        None => C::Dim("? not verified".to_owned()),
    }
}

/// 評価したケース（ケースを指定しなかったものは「default」）。
fn case_names(cases: &[ComplianceCase]) -> String {
    let names: Vec<&str> = cases
        .iter()
        .map(|c| c.case.as_deref().unwrap_or("default"))
        .collect();
    names.join(", ")
}

/// 行の補足: 判定した文か、確かめられなかった理由。
fn detail(r: &ComplianceRow) -> String {
    if let Some(reason) = r.reason {
        let why = match reason {
            NotVerifiedReason::NoLink => "no analysis check or budget linked",
            NotVerifiedReason::MissingInputs => "inputs missing",
            NotVerifiedReason::Failed => "analysis failed",
            NotVerifiedReason::KeyNotProduced => "check not produced",
            NotVerifiedReason::UnknownTarget => "unknown analysis, budget or case",
        };
        return match &r.detail {
            Some(d) => format!("{why}: {d}"),
            None => why.to_owned(),
        };
    }
    r.message_en
        .clone()
        .or_else(|| r.message.clone())
        .unwrap_or_default()
}
