use super::*;
use crate::analysis::{run_cached, typed_run};
use crate::budget::budget_cached;
use crate::dto::BudgetKind;
use sat_analysis::power::OrbitGeneration;
use sat_analysis::spec::Analysis;
use std::cell::Cell;
use std::path::Path;

fn demo() -> Workspace {
    Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
        .unwrap()
}

/// 同じ鍵は 1 回だけ計算し、鍵が違う・値の型が違えば別の値。
#[test]
fn cached_computes_once_per_key_and_type() {
    let session = Session::open(&demo(), &[]).unwrap();
    let calls = Cell::new(0);
    let count = |n: u32| {
        calls.set(calls.get() + 1);
        n
    };
    let a = session.cached(&("a", Some("x")), || count(1));
    let again = session.cached(&("a", Some("x")), || count(2));
    assert_eq!((*a, *again, calls.get()), (1, 1, 1));
    assert!(Rc::ptr_eq(&a, &again));
    // 鍵が違う
    assert_eq!(*session.cached(&("a", None::<&str>), || count(3)), 3);
    // 同じ鍵でも値の型が違えば別
    assert_eq!(*session.cached(&("a", Some("x")), || "text"), "text");
    assert_eq!(calls.get(), 2);
}

/// 失敗も記憶し、エラーは複製して返す。
#[test]
fn cached_result_remembers_failures() {
    let session = Session::open(&demo(), &[]).unwrap();
    let calls = Cell::new(0);
    let run = || {
        calls.set(calls.get() + 1);
        Err::<u32, _>(ServiceError::InvalidInput("no".to_owned()))
    };
    assert!(session.cached_result(&"k", run).is_err());
    assert!(session.cached_result(&"k", run).is_err());
    assert_eq!(calls.get(), 1);
}

/// 計算の中でこの記憶をもう一度使ってよい（入れ子の解析）。
#[test]
fn cached_allows_nested_use() {
    let session = Session::open(&demo(), &[]).unwrap();
    let outer = session.cached(&"outer", || *session.cached(&"inner", || 2_u32) + 1);
    assert_eq!(*outer, 3);
}

/// 下書き: 編集が無ければ自分自身、同じ編集は同じ記憶、編集は設計に効く（ディスクは変わらない）。
#[test]
fn draft_is_shared_and_leaves_the_base_alone() {
    let session = Session::open(&demo(), &[]).unwrap();
    assert!(Rc::ptr_eq(&session, &session.draft(&[]).unwrap()));
    let ops = vec![fune_ops::Op::RemoveRequirement {
        id: "R_MASS".to_owned(),
    }];
    let draft = session.draft(&ops).unwrap();
    assert!(Rc::ptr_eq(&draft, &session.draft(&ops).unwrap()));
    assert!(
        session
            .snapshot()
            .design
            .requirements
            .contains_key("R_MASS")
    );
    assert!(!draft.snapshot().design.requirements.contains_key("R_MASS"));
    assert_ne!(
        session.snapshot().design.revision,
        draft.snapshot().design.revision
    );
}

/// 解析の実行は同じ解析・依頼のケースで 1 回だけ（失敗でない結果は同じ値）。ケースの指定と省略は別の依頼。
#[test]
fn analysis_runs_are_reused() {
    let session = Session::open(&demo(), &[]).unwrap();
    let a = run_cached(&session, "power.mode_load", None).unwrap();
    let b = run_cached(&session, "power.mode_load", None).unwrap();
    assert!(Rc::ptr_eq(&a, &b));
    let c = run_cached(&session, "power.mode_load", Some("hot_imaging")).unwrap();
    assert!(!Rc::ptr_eq(&a, &c));
    assert!(run_cached(&session, "no.such", None).is_err());
}

/// 型付きの実行は実効のケースで記憶する（省略と、既定のケースを明示したものは同じ）。
#[test]
fn typed_runs_are_keyed_by_the_effective_case() {
    let session = Session::open(&demo(), &[]).unwrap();
    let a = typed_run::<OrbitGeneration>(&session, None).unwrap();
    let b = typed_run::<OrbitGeneration>(&session, None).unwrap();
    assert!(Rc::ptr_eq(&a, &b));
    let (default, _) = fune_projection::case_policy::effective_case(
        Some(&session.snapshot().design),
        OrbitGeneration::ID,
        None,
    );
    assert_eq!(default, Some("hot_imaging"));
    let explicit = typed_run::<OrbitGeneration>(&session, default).unwrap();
    assert!(Rc::ptr_eq(&a, &explicit));
    let other = typed_run::<OrbitGeneration>(&session, Some("cold_safe")).unwrap();
    assert!(!Rc::ptr_eq(&a, &other));
    assert!(a.output.is_some());
}

/// バジェットも同じ種類・ケースで 1 回だけ数える。
#[test]
fn budgets_are_reused() {
    let session = Session::open(&demo(), &[]).unwrap();
    let a = budget_cached(&session, BudgetKind::Mass, None).unwrap();
    let b = budget_cached(&session, BudgetKind::Mass, None).unwrap();
    assert!(Rc::ptr_eq(&a, &b));
}
