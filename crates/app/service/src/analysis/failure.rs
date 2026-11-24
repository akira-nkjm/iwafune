//! 解析の結果が「失敗」かどうかの判定（`fune analyze` の終了コード）。HTTP は結果を `AnalysisRunResult` のまま返すので、この関数は CLI が使う。

use crate::dto::{AnalysisRunResult, AnalysisRunStatus};
use sat_analysis::spec::Verdict;

/// 解析の結果が失敗か: 実行が成功していない（入力が足りない・失敗）、またはどれかの判定が違反。
/// 警告は失敗にしない。`--json` でも表でも同じ判定を使う。
pub fn fails(r: &AnalysisRunResult) -> bool {
    r.status != AnalysisRunStatus::Ok || r.checks.iter().any(|c| c.verdict == Verdict::Violation)
}

#[cfg(test)]
mod tests {
    use super::fails;
    use crate::dto::{AnalysisRunResult, AnalysisRunStatus, CheckView};
    use sat_analysis::spec::Verdict;

    fn check(verdict: Verdict) -> CheckView {
        CheckView {
            key: "k".into(),
            label: "l".into(),
            label_en: None,
            verdict,
            message: "m".into(),
            message_en: None,
            at: "/checks/0".into(),
            scope: None,
        }
    }

    fn result(status: AnalysisRunStatus, checks: Vec<CheckView>) -> AnalysisRunResult {
        AnalysisRunResult {
            id: "x.y".into(),
            case: None,
            case_source: None,
            configuration: None,
            configuration_source: None,
            side: None,
            status,
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
            space_weather: None,
            detail: None,
        }
    }

    #[test]
    fn ok_with_ok_or_warning_checks_passes() {
        let r = result(
            AnalysisRunStatus::Ok,
            vec![check(Verdict::Ok), check(Verdict::Warning)],
        );
        assert!(!fails(&r));
    }

    #[test]
    fn a_violation_check_fails() {
        let r = result(
            AnalysisRunStatus::Ok,
            vec![check(Verdict::Ok), check(Verdict::Violation)],
        );
        assert!(fails(&r));
    }

    #[test]
    fn missing_inputs_or_failed_run_fails() {
        assert!(fails(&result(AnalysisRunStatus::MissingInputs, vec![])));
        assert!(fails(&result(AnalysisRunStatus::Failed, vec![])));
    }
}
