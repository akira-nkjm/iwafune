use super::*;
use crate::Workspace;
use crate::dto::{ModeSummaryScope, Op, SystemModesRequest};
use sat_analysis::spec::System;
use std::path::Path;

fn demo() -> Workspace {
    Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
        .unwrap()
}

fn analyzed(case: Option<&str>, ops: Vec<Op>) -> crate::dto::SystemModesView {
    let req = SystemModesRequest {
        case: case.map(str::to_owned),
        ops,
        analyzed: true,
        ..Default::default()
    };
    crate::system_modes::system_modes(&demo(), System::Power, &req).unwrap()
}

#[test]
fn analyzed_view_adds_budgets_and_charts_in_design_order() {
    let view = analyzed(None, Vec::new());
    let ids: Vec<_> = view.modes.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ["NOMINAL", "MISSION", "DOWNLINK", "SAFE"]);
    for mode in &view.modes {
        let power = mode.power.as_ref().unwrap();
        assert_eq!(power.id, mode.id);
        assert!(power.sunlit.is_some() && power.eclipse.is_some());
        assert_eq!(power.charts.len(), 3);
        // 部品別の値の表は変わらない
        assert!(mode.sunlit.total.is_some());
    }
}

#[test]
fn plain_view_does_not_run_the_analyses() {
    let req = SystemModesRequest::default();
    let view = crate::system_modes::system_modes(&demo(), System::Power, &req).unwrap();
    assert!(view.modes.iter().all(|m| m.power.is_none()));
}

/// ケースを選ぶと、そのケースのモードだけが電力収支を持ち、ほかは軌道発電のグラフで代わる。
#[test]
fn case_limits_the_balance_to_its_mode() {
    let view = analyzed(Some("hot_imaging"), Vec::new());
    let power = |id: &str| {
        view.modes
            .iter()
            .find(|m| m.id == id)
            .unwrap()
            .power
            .as_ref()
            .unwrap()
    };
    assert_eq!(power("MISSION").charts.len(), 3);
    assert!(power("MISSION").sunlit.is_some());
    assert!(power("NOMINAL").sunlit.is_none());
    assert_eq!(power("NOMINAL").charts[0].id, "generation");
}

/// 保存していない編集は書き込まずに当たる（DoD の上限の要求を極端に下げると判定が変わり、保存済みは変わらない）。
#[test]
fn draft_ops_apply_without_writing() {
    let worst = |v: &crate::dto::SystemModesView| {
        sat_analysis::spec::Verdict::worst(
            v.modes
                .iter()
                .filter_map(|m| m.power.as_ref().and_then(|p| p.verdict)),
        )
    };
    // DoD の上限は要求が持つので、要求の値を書き換える編集を当てる
    let mut dod = demo()
        .snapshot()
        .unwrap()
        .design
        .requirements
        .values()
        .find(|r| r.id.as_str() == "R_DOD")
        .cloned()
        .unwrap();
    dod.constraint.value = Some(fune_model::value::Quantity::new(0.1, "%"));
    let draft = vec![Op::SetRequirement { requirement: dod }];
    assert_eq!(
        worst(&analyzed(None, draft)),
        Some(sat_analysis::spec::Verdict::Violation)
    );
    assert_eq!(
        worst(&analyzed(None, Vec::new())),
        Some(sat_analysis::spec::Verdict::Ok)
    );
}

#[test]
fn totals_scope_returns_only_the_total_row() {
    let ws = demo();
    let rows = |scope| {
        let req = SystemModesRequest {
            summary: scope,
            ..Default::default()
        };
        crate::system_modes::system_modes(&ws, System::Power, &req)
            .unwrap()
            .summary
    };
    assert_eq!(rows(ModeSummaryScope::All).len(), 3);
    let totals = rows(ModeSummaryScope::Totals);
    assert_eq!(totals.len(), 1);
    assert!(!totals[0].verdict);
}

#[test]
fn missing_analyses_leave_the_view_without_power_parts() {
    let ws = demo();
    let session = Session::open(&ws, &[]).unwrap();
    let runs = Runs {
        load: typed_run(&session, None).unwrap(),
        orbit: typed_run(&session, None).unwrap(),
        balance: typed_run(&session, None).unwrap(),
        sizing: typed_run(&session, None).unwrap(),
    };
    assert!(!assemble(&runs).is_empty());
    // どの解析も出力が無い（不足入力・失敗）ときは、表示は空
    fn none<A: sat_analysis::spec::Analysis>() -> Rc<TypedRun<A>> {
        Rc::new(TypedRun {
            input: None,
            output: None,
        })
    }
    let runs = Runs {
        load: none(),
        orbit: none(),
        balance: none(),
        sizing: none(),
    };
    assert!(assemble(&runs).is_empty());
}
