//! 実行の共通の入口・表や図の共通の部品（脚注・判定と状態の表示名・数えられなかった項目の表）。

use super::{Ctx, Out};
use crate::analysis::{TypedRun, run_cached, typed_run};
use crate::dto::{
    AnalysisRunResult, AnalysisRunStatus, QuantityView, ReportItem, ReportLang, ReportUnavailable,
};
use crate::error::{Result, ServiceError};
use crate::units::number_text;
use fune_projection::Projects;
use fune_report::labels::{Text, t, tr};
use fune_report::model::{Table, ValueEntry};
use sat_analysis::spec::{Analysis, Verdict};
use sat_units::format::unit_label;
use std::rc::Rc;

/// 解析 1 回分。DTO の結果（状態・KPI・判定・選んだケース。`result`）と、同じ設計・同じケースの型付きの入出力を持つ。
/// 表と図の値は型付きの出力から読む（JSON の欄の名前では読まない。欄の名前が変わればコンパイルが通らない）。
pub(super) struct Run<A: Analysis> {
    pub(super) result: Rc<AnalysisRunResult>,
    typed: Rc<TypedRun<A>>,
}

/// [`Run::counted`] の結果。
pub(super) enum Counted<'a, A: Analysis> {
    /// 数えられた（入力と出力）
    Yes {
        input: &'a A::Input,
        output: &'a A::Output,
    },
    /// 数えられなかった（理由）。表や図は「算出できません」の注記で書く
    No(String),
}

/// 解析を設計から実行する（書き出す設計の記憶から。同じ解析・ケースは 1 回だけ計算する。`case` が None なら解析の既定のケース）。
pub(super) fn run<A>(ctx: &Ctx, case: Option<&str>) -> Result<Run<A>>
where
    A: Analysis + Projects + 'static,
    A::Input: serde::de::DeserializeOwned + 'static,
    A::Output: serde::de::DeserializeOwned + 'static,
{
    Ok(Run {
        result: run_cached(&ctx.session, A::ID, case)?,
        typed: typed_run::<A>(&ctx.session, case)?,
    })
}

impl<A: Analysis> Run<A> {
    /// 数えられたか。数えられたはずなのに型付きの出力が無いときは、空の表にせずエラーにする（2 つの実行が食い違っている）。
    pub(super) fn counted(&self) -> Result<Counted<'_, A>> {
        if let Some(reason) = why_not(&self.result) {
            return Ok(Counted::No(reason));
        }
        match (&self.typed.input, &self.typed.output) {
            (Some(input), Some(output)) => Ok(Counted::Yes { input, output }),
            _ => Err(ServiceError::Internal(format!(
                "The typed run of {} has no output although the analysis result has one",
                A::ID
            ))),
        }
    }
}

/// 解析が数えられなかった理由（数えられたなら None）。
fn why_not(r: &AnalysisRunResult) -> Option<String> {
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
pub(super) fn footer(ctx: &Ctx) -> Vec<String> {
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
pub(super) fn revision(ctx: &Ctx) -> String {
    ctx.snapshot().design.revision.0.clone()
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
    ctx: &Ctx,
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
pub(super) fn case_label(ctx: &Ctx, case: Option<&str>) -> String {
    match case {
        None => t(ctx.lang, Text::CaseDefault).to_owned(),
        Some(id) => ctx
            .snapshot()
            .design
            .cases
            .get(id)
            .and_then(|c| c.label.clone())
            .unwrap_or_else(|| id.to_owned()),
    }
}

/// 部品の表示名（`<名前> (<ID>)`。名前が無ければ ID）。
pub(super) fn part_name(ctx: &Ctx, id: &str) -> String {
    match ctx
        .snapshot()
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
