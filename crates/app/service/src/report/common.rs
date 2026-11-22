//! 実行の共通の入口・表や図の共通の部品（脚注・判定と状態の表示名・数えられなかった項目の表）。

use super::{Ctx, Out};
use crate::analysis::run_analysis;
use crate::dto::{
    AnalysisRunResult, AnalysisRunStatus, QuantityView, ReportItem, ReportLang, ReportUnavailable,
    RunAnalysisRequest,
};
use crate::error::Result;
use crate::units::number_text;
use fune_report::labels::{Text, t, tr};
use fune_report::model::{Table, ValueEntry};
use sat_analysis::spec::Verdict;
use sat_units::format::unit_label;

/// 解析を設計から実行する（保存済みの設計。`case` が None なら解析の既定のケース）。
pub(super) fn run(ctx: &Ctx<'_>, id: &str, case: Option<&str>) -> Result<AnalysisRunResult> {
    run_analysis(
        ctx.ws,
        id,
        &RunAnalysisRequest {
            case: case.map(str::to_owned),
            ..RunAnalysisRequest::default()
        },
    )
}

/// 解析が数えられなかった理由（数えられたなら None）。
pub(super) fn why_not(r: &AnalysisRunResult) -> Option<String> {
    match r.status {
        AnalysisRunStatus::Ok if r.output.is_some() => None,
        AnalysisRunStatus::Ok => Some("the analysis returned no output".to_owned()),
        AnalysisRunStatus::MissingInputs => {
            let paths: Vec<&str> = r.missing.iter().map(|m| m.path.as_str()).collect();
            Some(format!("missing inputs ({})", paths.join(", ")))
        }
        AnalysisRunStatus::Failed => Some(
            r.error
                .clone()
                .unwrap_or_else(|| "the analysis failed".to_owned()),
        ),
    }
}

/// 表・図の下の注記の共通部分（有効数字・モデルの revision）。
pub(super) fn footer(ctx: &Ctx<'_>) -> Vec<String> {
    vec![
        tr(
            ctx.lang,
            Text::SignificantFigures,
            &[&sat_units::format::SIGNIFICANT_DIGITS.to_string()],
        ),
        tr(ctx.lang, Text::ModelRevision, &[&revision(ctx)]),
    ]
}

/// 設計の revision。
pub(super) fn revision(ctx: &Ctx<'_>) -> String {
    ctx.snapshot.design.revision.0.clone()
}

/// 数えられなかった項目を並べる（表や図は呼び出し側が「算出できません」の注記で書く）。
pub(super) fn note_unavailable(out: &mut Out, item: ReportItem, case: Option<&str>, reason: &str) {
    out.unavailable.push(ReportUnavailable {
        item,
        case: case.map(str::to_owned),
        reason: reason.to_owned(),
    });
}

/// 数えられなかったときの空の表。
pub(super) fn empty_table(
    ctx: &Ctx<'_>,
    item: ReportItem,
    case: Option<&str>,
    caption: String,
    reason: &str,
) -> Table {
    Table {
        item,
        case: case.map(str::to_owned),
        subject: None,
        variant: None,
        caption,
        columns: Vec::new(),
        rows: Vec::new(),
        notes: footer(ctx),
        unavailable: Some(tr(ctx.lang, Text::NotAvailable, &[reason])),
    }
}

/// 判定の表示名。
pub(super) fn verdict_text(lang: ReportLang, v: Verdict) -> &'static str {
    t(
        lang,
        match v {
            Verdict::Ok => Text::VerdictOk,
            Verdict::Warning => Text::VerdictWarning,
            Verdict::Violation => Text::VerdictViolation,
        },
    )
}

/// 日英の表示名の組から言語に合う側（英語が無ければ作者の表示名）。
pub(super) fn pick_label(lang: ReportLang, label: &str, label_en: Option<&str>) -> String {
    match lang {
        ReportLang::Ja => label.to_owned(),
        ReportLang::En => crate::query::english(label, label_en).to_owned(),
    }
}

/// 解析ケースの表示名（ケースの `label`、無ければ ID、ケースが無ければ「既定のケース」）。
pub(super) fn case_label(ctx: &Ctx<'_>, case: Option<&str>) -> String {
    match case {
        None => t(ctx.lang, Text::CaseDefault).to_owned(),
        Some(id) => ctx
            .snapshot
            .design
            .cases
            .get(id)
            .and_then(|c| c.label.clone())
            .unwrap_or_else(|| id.to_owned()),
    }
}

/// 部品の表示名（`<名前> (<ID>)`。名前が無ければ ID）。
pub(super) fn part_name(ctx: &Ctx<'_>, id: &str) -> String {
    match ctx
        .snapshot
        .resolved
        .parts
        .get(id)
        .and_then(|p| p.label.as_deref())
    {
        Some(label) => format!("{label} ({id})"),
        None => id.to_owned(),
    }
}

/// 量の表示から文章用の値の項目を作る。
pub(super) fn value_entry(key: String, label: String, q: &QuantityView) -> ValueEntry {
    ValueEntry {
        key,
        label,
        text: q.display.clone(),
        number: number_text(q.value),
        unit: unit_label(&q.unit),
        si: q.si,
    }
}

/// 表示用のケース（ケースによる項目の `case` 欄。既定のケースのときは解析が選んだケース）。
pub(super) fn effective(r: &AnalysisRunResult, requested: Option<&str>) -> Option<String> {
    r.case.clone().or_else(|| requested.map(str::to_owned))
}

/// 書き出したケースとして覚える（重複なし）。
pub(super) fn remember_case(out: &mut Out, case: &Option<String>) {
    if let Some(id) = case
        && !out.cases.contains(id)
    {
        out.cases.push(id.clone());
    }
}
