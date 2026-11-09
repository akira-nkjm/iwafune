//! 診断の表示（miette）。service が付けた該当行の本文（excerpt）を使い、TOML のファイルと行を指して示す。

use fune_service::dto::{DiagnosticView, Severity as DiagnosticSeverity};
use miette::{
    GraphicalReportHandler, GraphicalTheme, LabeledSpan, MietteDiagnostic, MietteError,
    MietteSpanContents, Report, Severity, SourceCode, SourceSpan, SpanContents,
};

/// 診断を描画した文字列。
pub fn render(diags: &[DiagnosticView], color: bool) -> String {
    let theme = if color {
        GraphicalTheme::unicode()
    } else {
        GraphicalTheme::unicode_nocolor()
    };
    let handler = GraphicalReportHandler::new_themed(theme)
        .with_links(false)
        .with_context_lines(1);
    let mut out = String::new();
    for d in diags {
        let report = to_report(d);
        if handler.render_report(&mut out, report.as_ref()).is_err() {
            out.push_str(&format!(
                "{}: {}\n",
                d.diagnostic.code, d.diagnostic.message
            ));
        }
    }
    out
}

fn to_report(view: &DiagnosticView) -> Report {
    let d = &view.diagnostic;
    let mut diag = MietteDiagnostic::new(d.message.clone())
        .with_code(d.code.clone())
        .with_severity(match d.severity {
            DiagnosticSeverity::Error => Severity::Error,
            DiagnosticSeverity::Warning => Severity::Warning,
            DiagnosticSeverity::Info => Severity::Advice,
        });
    let location = match (&d.file, d.line) {
        (Some(f), Some(l)) => Some(format!("{f}:{l}")),
        (Some(f), None) => Some(f.clone()),
        _ => None,
    };
    let help = [
        view.path.as_ref().map(|p| format!("パラメーター: {p}")),
        location,
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("  ");
    if !help.is_empty() {
        diag = diag.with_help(help);
    }
    let Some((source, span)) = excerpt_source(view) else {
        return Report::new(diag);
    };
    let label = view.path.clone().unwrap_or_else(|| "ここ".to_owned());
    diag = diag.with_label(LabeledSpan::new_with_span(Some(label), span));
    Report::new(diag).with_source_code(source)
}

/// 該当行の本文から、miette に渡す本文と範囲を作る。
fn excerpt_source(view: &DiagnosticView) -> Option<(Excerpt, SourceSpan)> {
    let ex = view.excerpt.as_ref()?;
    let d = &view.diagnostic;
    let line = d.line?;
    let index = usize::try_from(line.checked_sub(ex.first_line)?).ok()?;
    let mut offset = 0usize;
    let mut target = None;
    for (i, text) in ex.text.split_inclusive('\n').enumerate() {
        if i == index {
            target = Some((offset, text.trim_end_matches(['\n', '\r'])));
            break;
        }
        offset += text.len();
    }
    let (start, text) = target?;
    let column = d.column.map_or(0, |c| c.saturating_sub(1) as usize);
    let column = column.min(text.len());
    let trimmed = text.len() - text.trim_start().len();
    let begin = column.max(trimmed);
    let len = text.len().saturating_sub(begin).max(1);
    let name = d.file.clone().unwrap_or_default();
    let source = Excerpt {
        name,
        text: ex.text.clone(),
        first_line: ex.first_line as usize,
    };
    Some((source, SourceSpan::new((start + begin).into(), len)))
}

/// ファイルの一部。行番号をファイルの行に合わせて見せる。
#[derive(Debug)]
struct Excerpt {
    name: String,
    text: String,
    first_line: usize,
}

impl SourceCode for Excerpt {
    fn read_span<'a>(
        &'a self,
        span: &SourceSpan,
        context_lines_before: usize,
        context_lines_after: usize,
    ) -> Result<Box<dyn SpanContents<'a> + 'a>, MietteError> {
        let inner = self
            .text
            .read_span(span, context_lines_before, context_lines_after)?;
        Ok(Box::new(MietteSpanContents::new_named(
            self.name.clone(),
            inner.data(),
            *inner.span(),
            inner.line() + self.first_line.saturating_sub(1),
            inner.column(),
            inner.line_count(),
        )))
    }
}
