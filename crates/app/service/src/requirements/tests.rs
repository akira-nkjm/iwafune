//! 適合表の選び方（最も悪い判定の中で余裕の最も小さいもの・ケース）のテスト。

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
        check: None,
        margin: None,
        decisive: false,
    }
}

/// 違反 > 確かめられない > 警告 > 良。同じ順位なら先のケース。
#[test]
fn the_worst_case_decides_and_unverified_outranks_a_warning() {
    let pick = |v: &[Option<Verdict>]| {
        let cases: Vec<ComplianceCase> = v.iter().map(|x| case(*x)).collect();
        decisive_index(&cases).map(|i| cases[i].verdict)
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

fn check_view(key: &str, verdict: Verdict, margin: Option<f64>) -> CheckView {
    CheckView {
        key: key.to_owned(),
        label: key.to_owned(),
        label_en: None,
        verdict,
        message: format!("message of {key}"),
        message_en: None,
        at: format!("/checks/{key}"),
        scope: None,
        margin,
    }
}

fn run(checks: Vec<CheckView>) -> AnalysisRunResult {
    AnalysisRunResult {
        id: "x.y".into(),
        case: None,
        case_source: None,
        configuration: None,
        configuration_source: None,
        side: None,
        status: AnalysisRunStatus::Ok,
        checks,
        verdict: None,
        kpis: vec![],
        input: None,
        output: None,
        dims: Default::default(),
        labels: Default::default(),
        missing: vec![],
        error: None,
        overview: Default::default(),
        part_heat: None,
        part_findings: None,
        space_weather: None,
        detail: None,
    }
}

/// 同じキーの族に当たる判定が全部良なら、余裕の最も小さいものが行を決める（並びの順によらない）。
#[test]
fn among_equal_verdicts_the_smallest_margin_decides() {
    let first_loose = run(vec![
        check_view("margin:a@GS", Verdict::Ok, Some(0.9)),
        check_view("margin:b@GS", Verdict::Ok, Some(0.2)),
    ]);
    let c = judge_analysis(&first_loose, Some("margin"), None);
    assert_eq!(
        c.check.as_ref().map(|k| k.key.as_str()),
        Some("margin:b@GS")
    );
    assert_eq!(c.message.as_deref(), Some("message of margin:b@GS"));
    assert_eq!(c.margin, Some(0.2));
    let first_tight = run(vec![
        check_view("margin:b@GS", Verdict::Ok, Some(0.2)),
        check_view("margin:a@GS", Verdict::Ok, Some(0.9)),
    ]);
    let c = judge_analysis(&first_tight, Some("margin"), None);
    assert_eq!(
        c.check.as_ref().map(|k| k.key.as_str()),
        Some("margin:b@GS")
    );
}

/// 判定の順位が先: 余裕の大きい警告は、余裕の小さい良より先に行を決める。
#[test]
fn the_verdict_decides_before_the_margin() {
    let r = run(vec![
        check_view("margin:a@GS", Verdict::Ok, Some(0.01)),
        check_view("margin:b@GS", Verdict::Warning, Some(5.0)),
    ]);
    let c = judge_analysis(&r, Some("margin"), None);
    assert_eq!(c.verdict, Some(Verdict::Warning));
    assert_eq!(
        c.check.as_ref().map(|k| k.key.as_str()),
        Some("margin:b@GS")
    );
}

/// 余裕を持たない判定どうしは先に出たもの（今までどおり）。持つ判定は持たない判定より先。
#[test]
fn checks_without_margins_keep_their_order() {
    let r = run(vec![
        check_view("dod", Verdict::Ok, None),
        check_view("dod", Verdict::Ok, None),
    ]);
    let c = judge_analysis(&r, Some("dod"), None);
    assert_eq!(c.check.as_ref().map(|k| k.key.as_str()), Some("dod"));
    assert_eq!(c.margin, None);
    let mixed = run(vec![
        check_view("margin:a@GS", Verdict::Ok, None),
        check_view("margin:b@GS", Verdict::Ok, Some(3.0)),
    ]);
    let c = judge_analysis(&mixed, Some("margin"), None);
    assert_eq!(
        c.check.as_ref().map(|k| k.key.as_str()),
        Some("margin:b@GS")
    );
}

fn with_margin(verdict: Verdict, margin: Option<f64>, name: &str) -> ComplianceCase {
    ComplianceCase {
        case: Some(name.to_owned()),
        margin,
        ..case(Some(verdict))
    }
}

/// ケースをまたぐ: 同じ順位なら余裕の小さいケース、違反は余裕によらず決める。同じなら先のケース。
#[test]
fn across_cases_the_tighter_case_decides_and_a_violation_always_does() {
    let (ok, bad) = (Verdict::Ok, Verdict::Violation);
    let loose_first = [
        with_margin(ok, Some(0.8), "a"),
        with_margin(ok, Some(0.1), "b"),
    ];
    assert_eq!(decisive_index(&loose_first), Some(1));
    let tight_first = [
        with_margin(ok, Some(0.1), "a"),
        with_margin(ok, Some(0.8), "b"),
    ];
    assert_eq!(decisive_index(&tight_first), Some(0));
    let violated = [
        with_margin(ok, Some(0.0), "a"),
        with_margin(bad, Some(-0.5), "b"),
        with_margin(ok, Some(0.3), "c"),
    ];
    assert_eq!(decisive_index(&violated), Some(1));
    let none = [with_margin(ok, None, "a"), with_margin(ok, None, "b")];
    assert_eq!(decisive_index(&none), Some(0));
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
