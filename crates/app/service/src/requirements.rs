//! 要求適合表（`GET /requirements/compliance`・`fune requirement compliance`）。要求ごとに、確かめる先（解析の判定・バジェット）を
//! 量化したケースで実行し、判定を集める。**判定を出すのは解析とバジェットで、ここは選んで集めるだけ**（制約の値と比べ直さない）。
//!
//! 判定のキーは解析が実行のたびに作る文字列（`dod`・`margin:s_down@TOKYO_GS`）で、解析の出力を見るまで実在が分からない。
//! だから `fune check` では解析の ID だけを見て、キーの実在はここで確かめる（出なければ [`NotVerifiedReason::KeyNotProduced`]）。
//! 同じキーの判定が複数あれば（モードごとなど）最も悪いものを使い、同じ悪さの中では**余裕（`Check::margin`）が最も小さい**もの
//! （余裕を持たない判定は持つ判定より後。持たない同士は先に出たもの）。ケースごとの結果を集めた行の判定は、
//! 違反 > 確かめられない > 警告 > 良の順（1 つでも確かめられないケースがあれば「良」と言わない）で、同じ順位の中では余裕が最も小さいケース
//! （同じなら先のケース）。行は、文を決めた判定（`check`）とケース（`decisive_case`）を名指す。
//! 余裕の定義と比べ方は `sat_analysis::spec`（`Check::with_margin`・`margin_order`）だけにあり、ここは比べるだけ（値や文から作り直さない）。
//! 設計は 1 回だけ読み（[`Session`]）、解析とバジェットはケースごとに 1 回だけ実行する（同じ解析・ケースは要求をまたいで使い回す。
//! 要求を外した設計は下書きとして読み込み済みの設計から作り、同じ外し方の要求は同じ下書きを使う）。適合表の中で設計が編集されても、1 つの表に違う revision は混ざらない。
//!
//! 判定の基準は要求が持つ（`fune_model::design::criteria`）。同じ基準に効く要求が複数あれば、解析とバジェットは最も厳しいものを使うので、
//! **各要求の行は、その要求の制約だけを基準にして判定し直す**（同じ基準に効くほかの要求を外した設計で実行する。保存はしない）。
//! だから緩い要求は、厳しい要求が満たされていれば満たされ、厳しい要求が違反でもその要求自身の制約を満たしていれば良になる。
//! 判定のキーは、要求の `check` が接頭辞として当たるもの（`margin` は `margin:<回線>@<局>` のすべて。`key_covers`）を集める。

use crate::analysis::run_cached;
use crate::budget::budget_cached;
use crate::dto::{
    AnalysisRunResult, AnalysisRunStatus, BudgetKind, BudgetPropellant, BudgetView, CheckView,
    ComplianceCase, ComplianceReport, ComplianceRow, ComplianceSummary, DecisiveCheck,
    NotVerifiedReason, QuantityView,
};
use crate::error::Result;
use crate::query::requirements::constraint_view;
use crate::session::Session;
use crate::workspace::Workspace;
use fune_model::design::{Design, Requirement, key_covers};
use sat_analysis::spec::{Verdict, margin_order};
use std::rc::Rc;

/// 出た判定のキーを `detail` に並べる最大の数。
const KEYS_SHOWN: usize = 6;

/// 要求適合表を作る（保存済みの設計。ID 順）。
pub fn compliance(ws: &Workspace) -> Result<ComplianceReport> {
    let session = Session::open(ws, &[])?;
    let rows: Vec<ComplianceRow> = session
        .snapshot()
        .design
        .requirements
        .values()
        .map(|r| row(&session, r))
        .collect();
    let summary = summarize(&rows);
    Ok(ComplianceReport { rows, summary })
}

/// 適合表が「失敗」か（`fune requirement compliance` の終了コード）: 違反の行がある。`deny_unverified` なら確かめられない行も。
/// 警告は失敗にしない。`--json` でも表でも同じ判定を使う。
pub fn fails(report: &ComplianceReport, deny_unverified: bool) -> bool {
    report.summary.violation > 0 || (deny_unverified && report.summary.not_verified > 0)
}

fn row(session: &Rc<Session>, r: &Requirement) -> ComplianceRow {
    let design = &session.snapshot().design;
    let mut cases: Vec<ComplianceCase> = if let (true, Some(input)) = (r.verify.is_empty(), r.input)
    {
        let detail = Some(format!("Gives the analysis input `{input}`"));
        vec![unverified(None, NotVerifiedReason::DesignInput, detail)]
    } else if r.verify.is_empty() {
        vec![unverified(None, NotVerifiedReason::NoLink, None)]
    } else {
        quantified_cases(design, r)
            .into_iter()
            .map(|case| judge(session, r, case))
            .collect()
    };
    let decided = decisive_index(&cases).map(|i| {
        cases[i].decisive = true;
        cases[i].clone()
    });
    let decided = decided.unwrap_or_else(|| unverified(None, NotVerifiedReason::NoLink, None));
    let id = r.id.as_str();
    ComplianceRow {
        id: id.to_owned(),
        label: fune_model::display_name::name_or_id(r.label.as_deref(), id),
        statement: r.statement.clone(),
        statement_en: r.statement_en.clone(),
        quantity: r.quantity.clone(),
        constraint: constraint_view(r),
        source: r.source.as_ref().map(ToString::to_string),
        verify: r.verify.clone(),
        input: r.input,
        verdict: decided.verdict,
        reason: decided.reason,
        detail: decided.detail,
        message: decided.message,
        message_en: decided.message_en,
        value: decided.value,
        check: decided.check,
        decisive_case: decided.case,
        margin: decided.margin,
        cases,
    }
}

/// 行の文を決めるケース: 順位が最も悪いもの、同じ順位では余裕が最も小さいもの、それも同じなら先のケース（`min_by` は先を残す）。
fn decisive_index(cases: &[ComplianceCase]) -> Option<usize> {
    cases
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| rank(b).cmp(&rank(a)).then(margin_order(a.margin, b.margin)))
        .map(|(i, _)| i)
}

/// 行の判定を決める順位（高いほど悪い）。
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
        check: None,
        margin: None,
        decisive: false,
    }
}

fn judge(session: &Rc<Session>, r: &Requirement, case: Option<String>) -> ComplianceCase {
    let design = &session.snapshot().design;
    if let Some(c) = case.as_deref()
        && !design.cases.contains_key(c)
    {
        let detail = Some(format!("Analysis case `{c}` is not defined"));
        return unverified(case, NotVerifiedReason::UnknownTarget, detail);
    }
    // 同じ基準に効くほかの要求を外して、この要求の制約だけで判定する（外した設計は読み込み済みの設計から作る）
    let draft = session.draft(&remove_ops(&design.criterion_siblings(r)));
    if let Some(kind) = r.verify.budget {
        return judge_budget(draft, kind, case);
    }
    let Some(analysis) = r.verify.analysis.as_deref() else {
        return unverified(case, NotVerifiedReason::NoLink, None);
    };
    match draft.and_then(|d| run_cached(&d, analysis, case.as_deref())) {
        Err(e) => unverified(case, NotVerifiedReason::UnknownTarget, Some(e.to_string())),
        Ok(result) => judge_analysis(&result, r.verify.check.as_deref(), case),
    }
}

/// 要求を外す編集（保存しない。解析とバジェットの `ops`）。
fn remove_ops(ids: &[String]) -> Vec<fune_ops::Op> {
    ids.iter()
        .map(|id| fune_ops::Op::RemoveRequirement { id: id.clone() })
        .collect()
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
        .filter(|c| key.is_none_or(|k| key_covers(k, &c.key)))
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
    // 最も悪い判定の中で余裕が最も小さいもの（余裕が同じ・無ければ先に出たもの）
    let decisive = hits
        .iter()
        .filter(|c| c.verdict == verdict)
        .min_by(|a, b| margin_order(a.margin, b.margin));
    ComplianceCase {
        case: evaluated,
        verdict: Some(verdict),
        reason: None,
        detail: None,
        message: decisive.map(|c| c.message.clone()),
        message_en: decisive.and_then(|c| c.message_en.clone()),
        value: None,
        check: decisive.map(|c| DecisiveCheck {
            key: c.key.clone(),
            label: c.label.clone(),
            label_en: c.label_en.clone(),
            scope: c.scope.clone(),
        }),
        margin: decisive.and_then(|c| c.margin),
        decisive: false,
    }
}

fn judge_budget(
    draft: Result<Rc<Session>>,
    kind: BudgetKind,
    case: Option<String>,
) -> ComplianceCase {
    let b = match draft.and_then(|d| budget_cached(&d, kind, case.as_deref())) {
        Err(e) => return unverified(case, NotVerifiedReason::Failed, Some(e.to_string())),
        Ok(b) => b,
    };
    let b: &BudgetView = &b;
    if !b.missing.is_empty() {
        let paths: Vec<&str> = b.missing.iter().map(|m| m.path.as_str()).collect();
        let detail = Some(paths.join(", "));
        return unverified(case, NotVerifiedReason::MissingInputs, detail);
    }
    let candidates = budget_judgements(b);
    let Some(verdict) = Verdict::worst(candidates.iter().map(|c| c.0)) else {
        let detail = Some(format!("No requirement sets the {kind} limit"));
        return unverified(case, NotVerifiedReason::MissingInputs, detail);
    };
    let decisive = candidates
        .iter()
        .filter(|c| c.0 == verdict)
        .min_by(|a, b| margin_order(a.1.map(|m| m.si), b.1.map(|m| m.si)));
    let total: Option<&QuantityView> = decisive.and_then(|c| c.2);
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
        check: None,
        margin: decisive.and_then(|c| c.1.map(|m| m.si)),
        decisive: false,
    }
}

/// バジェットの乾燥の合計と推薬込みの合計の判定（推薬込みを先に並べる: 同じなら推薬込みの値を出す）。余裕はバジェット自身の余裕（上限までの量）。
fn budget_judgements(b: &BudgetView) -> Vec<BudgetJudged<'_>> {
    let wet = match &b.propellant {
        Some(BudgetPropellant::Counted { wet, .. }) => Some(wet),
        _ => None,
    };
    [
        wet.and_then(|w| Some((w.verdict?, w.margin.as_ref(), Some(&w.total)))),
        b.verdict.map(|v| (v, b.margin.as_ref(), b.total.as_ref())),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// バジェットの判定 1 つ（判定・余裕・合計）。
type BudgetJudged<'a> = (Verdict, Option<&'a QuantityView>, Option<&'a QuantityView>);

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
mod tests;
