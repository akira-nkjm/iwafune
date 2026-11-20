//! 系の概要の解析の表の行（`dto::AnalysisOverview`）のうち、どの系でも同じ組み立て。**行に出す主な結果の選び方
//! （KPI の定義の順に [`MAIN_KPIS`] 個まで）はここだけ**。判定の文は持ち主
//! （`CheckView.scope`）ごとにまとめる。小さな棒グラフと「データを見る」で隠す表は、系ごとの組み立て（電源は
//! `power`）が [`add_system_parts`] で足す。

use crate::dto::{AnalysisOverview, AnalysisRunResult, CheckView, KpiView, OverviewCheckGroup};
use crate::dto::{OverviewChart, QuantityView};
use crate::workspace::Snapshot;

mod power;

/// 行に出す主な結果の数の上限。
pub(crate) const MAIN_KPIS: usize = 3;

/// 主な結果: KPI を定義の順に [`MAIN_KPIS`] 個まで。
pub(crate) fn main_kpis(kpis: &[KpiView]) -> &[KpiView] {
    &kpis[..kpis.len().min(MAIN_KPIS)]
}

/// 判定の文を持ち主ごとにまとめる（まとまりは最初に出た順、中は元の順）。
pub(crate) fn check_groups(checks: &[CheckView]) -> Vec<OverviewCheckGroup> {
    let mut groups: Vec<OverviewCheckGroup> = Vec::new();
    for c in checks {
        match groups.iter_mut().find(|g| g.scope == c.scope) {
            Some(g) => g.checks.push(c.clone()),
            None => groups.push(OverviewCheckGroup {
                scope: c.scope.clone(),
                checks: vec![c.clone()],
            }),
        }
    }
    groups
}

/// 解析 1 つの行の概要。`chart` と `hidden_tables` は系ごとの組み立てが選んだもの。
pub(crate) fn overview(
    result: &AnalysisRunResult,
    chart: Option<OverviewChart>,
    hidden_tables: Vec<Vec<String>>,
) -> AnalysisOverview {
    AnalysisOverview {
        main: main_kpis(&result.kpis).to_vec(),
        checks: check_groups(&result.checks),
        chart,
        hidden_tables,
    }
}

/// 系ごとの組み立て（今は電源の棒グラフと隠す表）を結果の概要の行に足す。`snapshot` と `case` は実行に使った設計とケース
/// （入力を丸ごと与えた実行は `snapshot` が無い）。
pub(crate) fn add_system_parts(
    result: &mut AnalysisRunResult,
    snapshot: Option<&Snapshot>,
    case: Option<&str>,
) {
    power::add(result, snapshot, case);
}

/// 棒の塗りの長さ: いちばん大きい値に対する割合（0〜1。最大が 0 以下か値が無ければ 0）。
pub(crate) fn share_of_largest(value: Option<&QuantityView>, largest: f64) -> f64 {
    match value {
        Some(v) if largest > 0.0 && v.si.is_finite() => (v.si / largest).clamp(0.0, 1.0),
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sat_analysis::spec::Verdict;

    fn check(at: &str, scope: Option<&str>) -> CheckView {
        CheckView {
            key: "dod".into(),
            label: "DoD".into(),
            label_en: None,
            verdict: Verdict::Ok,
            message: String::new(),
            message_en: None,
            at: at.into(),
            scope: scope.map(str::to_owned),
        }
    }

    #[test]
    fn checks_are_grouped_by_owner_in_first_seen_order() {
        let groups = check_groups(&[
            check("/cases/0/checks/0", Some("定常")),
            check("/cases/1/checks/0", Some("撮像")),
            check("/cases/0/checks/1", Some("定常")),
            check("/checks/0", None),
        ]);
        let shape: Vec<_> = groups
            .iter()
            .map(|g| (g.scope.as_deref(), g.checks.len()))
            .collect();
        assert_eq!(shape, [(Some("定常"), 2), (Some("撮像"), 1), (None, 1)]);
        assert_eq!(groups[0].checks[1].at, "/cases/0/checks/1");
    }

    #[test]
    fn share_is_clamped_and_zero_without_scale() {
        let q = |si: f64| crate::units::quantity_auto(si, "Power");
        assert_eq!(share_of_largest(Some(&q(5.0)), 10.0), 0.5);
        assert_eq!(share_of_largest(Some(&q(15.0)), 10.0), 1.0);
        assert_eq!(share_of_largest(Some(&q(5.0)), 0.0), 0.0);
        assert_eq!(share_of_largest(None, 10.0), 0.0);
    }
}
