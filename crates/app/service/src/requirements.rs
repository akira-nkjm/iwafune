//! 要求適合表（`GET /requirements/compliance`・`fune requirement compliance`）。要求ごとに、確かめる先（解析の判定・バジェット）を
//! 量化したケースで実行し、判定を集める。**判定を出すのは解析とバジェットで、ここは選んで集めるだけ**（制約の値と比べ直さない）。
//!
//! 判定のキーは解析が実行のたびに作る文字列（`dod`・`margin:s_down@TOKYO_GS`）で、解析の出力を見るまで実在が分からない。
//! だから `fune check` では解析の ID だけを見て、キーの実在はここで確かめる（出なければ [`NotVerifiedReason::KeyNotProduced`]）。
//! 同じキーの判定が複数あれば（モードごとなど）最も悪いものを使う。ケースごとの結果を集めた行の判定は、
//! 違反 > 確かめられない > 警告 > 良の順（1 つでも確かめられないケースがあれば「良」と言わない）。
//! 解析とバジェットはケースごとに 1 回だけ実行する（同じ解析・ケースは要求をまたいで使い回す）。

use crate::analysis::run_analysis;
use crate::budget::budget;
use crate::dto::{
    AnalysisRunResult, AnalysisRunStatus, BudgetKind, BudgetPropellant, BudgetRequest, BudgetView,
    CheckView, ComplianceCase, ComplianceReport, ComplianceRow, ComplianceSummary,
    NotVerifiedReason, QuantityView, RunAnalysisRequest,
};
use crate::error::Result;
use crate::query::requirements::constraint_view;
use crate::workspace::Workspace;
use fune_model::design::{Design, Requirement};
use sat_analysis::spec::Verdict;
use std::collections::BTreeMap;

/// 出た判定のキーを `detail` に並べる最大の数。
const KEYS_SHOWN: usize = 6;

/// 要求適合表を作る（保存済みの設計。ID 順）。
pub fn compliance(ws: &Workspace) -> Result<ComplianceReport> {
    let design = ws.snapshot()?.design;
    let mut runs = Runs::default();
    let rows: Vec<ComplianceRow> = design
        .requirements
        .values()
        .map(|r| row(ws, &design, r, &mut runs))
        .collect();
    let summary = summarize(&rows);
    Ok(ComplianceReport { rows, summary })
}

/// 適合表が「失敗」か（`fune requirement compliance` の終了コード）: 違反の行がある。`deny_unverified` なら確かめられない行も。
/// 警告は失敗にしない。`--json` でも表でも同じ判定を使う。
pub fn fails(report: &ComplianceReport, deny_unverified: bool) -> bool {
    report.summary.violation > 0 || (deny_unverified && report.summary.not_verified > 0)
}

/// 同じ解析・ケースの実行を使い回す入れ物。
#[derive(Default)]
struct Runs {
    analyses: BTreeMap<(String, Option<String>), Result<AnalysisRunResult>>,
    budgets: BTreeMap<(BudgetKind, Option<String>), Result<BudgetView>>,
}

fn row(ws: &Workspace, design: &Design, r: &Requirement, runs: &mut Runs) -> ComplianceRow {
    let cases: Vec<ComplianceCase> = if r.verify.is_empty() {
        vec![unverified(None, NotVerifiedReason::NoLink, None)]
    } else {
        quantified_cases(design, r)
            .into_iter()
            .map(|case| judge(ws, design, r, case, runs))
            .collect()
    };
    let decided = cases
        .iter()
        .max_by_key(|c| rank(c))
        .cloned()
        .unwrap_or_else(|| unverified(None, NotVerifiedReason::NoLink, None));
    let id = r.id.as_str();
    ComplianceRow {
        id: id.to_owned(),
        label: r.label.clone().unwrap_or_else(|| id.to_owned()),
        statement: r.statement.clone(),
        statement_en: r.statement_en.clone(),
        quantity: r.quantity.clone(),
        constraint: constraint_view(r),
        source: r.source.as_ref().map(ToString::to_string),
        verify: r.verify.clone(),
        verdict: decided.verdict,
        reason: decided.reason,
        detail: decided.detail,
        message: decided.message,
        message_en: decided.message_en,
        value: decided.value,
        cases,
    }
}

/// 行の判定を決める順位（高いほど悪い。同じなら先のケース）。
fn rank(c: &ComplianceCase) -> u8 {
    match (c.verdict, c.reason) {
        (Some(Verdict::Violation), _) => 3,
        (None, _) => 2,
        (Some(Verdict::Warning), _) => 1,
        (Some(Verdict::Ok), _) => 0,
    }
}

/// 量化したケース（`None` はケースを指定せず、解析の既定に任せる）。`cases/` が無ければ `all_cases` も既定の 1 つだけ。
fn quantified_cases(design: &Design, r: &Requirement) -> Vec<Option<String>> {
    let q = &r.quantify;
    if q.all_cases && !design.cases.is_empty() {
        return design.cases.keys().map(|c| Some(c.to_string())).collect();
    }
    if !q.all_cases && !q.cases.is_empty() {
        return q.cases.iter().map(|c| Some(c.to_string())).collect();
    }
    vec![None]
}

fn unverified(
    case: Option<String>,
    reason: NotVerifiedReason,
    detail: Option<String>,
) -> ComplianceCase {
    ComplianceCase {
        case,
        verdict: None,
        reason: Some(reason),
        detail,
        message: None,
        message_en: None,
        value: None,
    }
}

fn judge(
    ws: &Workspace,
    design: &Design,
    r: &Requirement,
    case: Option<String>,
    runs: &mut Runs,
) -> ComplianceCase {
    if let Some(c) = case.as_deref()
        && !design.cases.contains_key(c)
    {
        let detail = Some(format!("Analysis case `{c}` is not defined"));
        return unverified(case, NotVerifiedReason::UnknownTarget, detail);
    }
    if let Some(kind) = r.verify.budget {
        return judge_budget(ws, kind, case, runs);
    }
    let Some(analysis) = r.verify.analysis.as_deref() else {
        return unverified(case, NotVerifiedReason::NoLink, None);
    };
    let key = (analysis.to_owned(), case.clone());
    let run = runs.analyses.entry(key).or_insert_with(|| {
        let req = RunAnalysisRequest {
            case: case.clone(),
            ..RunAnalysisRequest::default()
        };
        run_analysis(ws, analysis, &req)
    });
    match run {
        Err(e) => unverified(case, NotVerifiedReason::UnknownTarget, Some(e.to_string())),
        Ok(result) => judge_analysis(result, r.verify.check.as_deref(), case),
    }
}

/// 解析の結果から、`check` のキーの判定（無ければ解析の判定全体）の最も悪いものを選ぶ。
fn judge_analysis(
    result: &AnalysisRunResult,
    key: Option<&str>,
    case: Option<String>,
) -> ComplianceCase {
    let evaluated = result.case.clone().or(case);
    match result.status {
        AnalysisRunStatus::MissingInputs => {
            let paths: Vec<&str> = result.missing.iter().map(|m| m.path.as_str()).collect();
            let detail = Some(paths.join(", "));
            return unverified(evaluated, NotVerifiedReason::MissingInputs, detail);
        }
        AnalysisRunStatus::Failed => {
            let detail = result.error.clone();
            return unverified(evaluated, NotVerifiedReason::Failed, detail);
        }
        AnalysisRunStatus::Ok => {}
    }
    let hits: Vec<&CheckView> = result
        .checks
        .iter()
        .filter(|c| key.is_none_or(|k| c.key == k))
        .collect();
    let Some(verdict) = Verdict::worst(hits.iter().map(|c| c.verdict)) else {
        let mut keys: Vec<&str> = result.checks.iter().map(|c| c.key.as_str()).collect();
        keys.dedup();
        let shown: Vec<&str> = keys.iter().copied().take(KEYS_SHOWN).collect();
        let more = if keys.len() > KEYS_SHOWN { ", …" } else { "" };
        let detail = Some(if keys.is_empty() {
            "The analysis produced no checks".to_owned()
        } else {
            format!("Checks produced: {}{more}", shown.join(", "))
        });
        return unverified(evaluated, NotVerifiedReason::KeyNotProduced, detail);
    };
    let worst = hits.iter().find(|c| c.verdict == verdict);
    ComplianceCase {
        case: evaluated,
        verdict: Some(verdict),
        reason: None,
        detail: None,
        message: worst.map(|c| c.message.clone()),
        message_en: worst.and_then(|c| c.message_en.clone()),
        value: None,
    }
}

fn judge_budget(
    ws: &Workspace,
    kind: BudgetKind,
    case: Option<String>,
    runs: &mut Runs,
) -> ComplianceCase {
    let run = runs.budgets.entry((kind, case.clone())).or_insert_with(|| {
        let req = BudgetRequest {
            case: case.clone(),
            ..BudgetRequest::default()
        };
        budget(ws, kind, &req)
    });
    let b = match run {
        Err(e) => return unverified(case, NotVerifiedReason::Failed, Some(e.to_string())),
        Ok(b) => b,
    };
    if !b.missing.is_empty() {
        let paths: Vec<&str> = b.missing.iter().map(|m| m.path.as_str()).collect();
        let detail = Some(paths.join(", "));
        return unverified(case, NotVerifiedReason::MissingInputs, detail);
    }
    let wet = match &b.propellant {
        Some(BudgetPropellant::Counted { wet, .. }) => Some(wet),
        _ => None,
    };
    let Some(verdict) = Verdict::worst(b.verdict.into_iter().chain(wet.and_then(|w| w.verdict)))
    else {
        let detail = Some(format!("no {kind} limit in budget/allocation.toml"));
        return unverified(case, NotVerifiedReason::MissingInputs, detail);
    };
    let total: Option<&QuantityView> = wet.map(|w| &w.total).or(b.total.as_ref());
    let message = match (total, &b.limit) {
        (Some(t), Some(l)) => Some(format!("Total {} against limit {}", t.display, l.display)),
        _ => None,
    };
    ComplianceCase {
        case,
        verdict: Some(verdict),
        reason: None,
        detail: None,
        message,
        message_en: None,
        value: total.cloned(),
    }
}

/// 判定の数え方（行の判定ごと）。
fn summarize(rows: &[ComplianceRow]) -> ComplianceSummary {
    let mut s = ComplianceSummary {
        total: u32::try_from(rows.len()).unwrap_or(u32::MAX),
        ..ComplianceSummary::default()
    };
    for r in rows {
        match r.verdict {
            Some(Verdict::Ok) => s.ok += 1,
            Some(Verdict::Warning) => s.warning += 1,
            Some(Verdict::Violation) => s.violation += 1,
            None => s.not_verified += 1,
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case(verdict: Option<Verdict>) -> ComplianceCase {
        ComplianceCase {
            case: None,
            verdict,
            reason: verdict.is_none().then_some(NotVerifiedReason::Failed),
            detail: None,
            message: None,
            message_en: None,
            value: None,
        }
    }

    /// 違反 > 確かめられない > 警告 > 良。同じ順位なら先のケース。
    #[test]
    fn the_worst_case_decides_and_unverified_outranks_a_warning() {
        let pick = |v: &[Option<Verdict>]| {
            let cases: Vec<ComplianceCase> = v.iter().map(|x| case(*x)).collect();
            cases.iter().max_by_key(|c| rank(c)).map(|c| c.verdict)
        };
        let (ok, warn, bad) = (
            Some(Verdict::Ok),
            Some(Verdict::Warning),
            Some(Verdict::Violation),
        );
        assert_eq!(pick(&[ok, warn]), Some(warn));
        assert_eq!(pick(&[warn, None]), Some(None));
        assert_eq!(pick(&[ok, None, bad]), Some(bad));
        assert_eq!(pick(&[ok, ok]), Some(ok));
    }

    #[test]
    fn failure_counts_violations_and_optionally_unverified_rows() {
        let report = |violation, not_verified| ComplianceReport {
            rows: Vec::new(),
            summary: ComplianceSummary {
                violation,
                not_verified,
                ..ComplianceSummary::default()
            },
        };
        assert!(fails(&report(1, 0), false));
        assert!(!fails(&report(0, 2), false));
        assert!(fails(&report(0, 2), true));
        assert!(!fails(&report(0, 0), true));
    }
}
