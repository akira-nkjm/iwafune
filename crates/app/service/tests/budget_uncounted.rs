//! 値の無い部品を黙って集計から外さない: バジェットの `uncounted`。
//! 合計は数えた部品だけの値のまま（0 とはみなさない）。
use fune_service::dto::{BudgetKind, BudgetRequest, BudgetView};
use fune_service::{Workspace, budget};
use serde_json::{Value, json};
use std::path::Path;

fn demo() -> Workspace {
    Workspace::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u"))
        .expect("demo")
}

fn ops(v: Value) -> Vec<fune_service::dto::Op> {
    serde_json::from_value(v).unwrap()
}

fn view(ws: &Workspace, kind: BudgetKind, edit: Value) -> BudgetView {
    let req = BudgetRequest {
        case: None,
        ops: ops(edit),
    };
    budget::budget(ws, kind, &req).unwrap()
}

#[test]
fn demo_counts_every_part() {
    let ws = demo();
    for kind in [BudgetKind::Mass, BudgetKind::Power, BudgetKind::Data] {
        assert!(view(&ws, kind, json!([])).uncounted.is_empty(), "{kind}");
    }
}

/// 質量を消した部品は、合計から外れたうえで `uncounted` に名前が出る（合計はその分だけ小さい）。
#[test]
fn mass_budget_names_the_part_without_mass() {
    let ws = demo();
    let before = view(&ws, BudgetKind::Mass, json!([]));
    let after = view(
        &ws,
        BudgetKind::Mass,
        json!([{ "op": "unset_value", "part": "HTR_CAM", "attr": "mass" }]),
    );
    let names: Vec<&str> = after.uncounted.iter().map(|u| u.part.as_str()).collect();
    assert_eq!(names, ["HTR_CAM"]);
    assert!(after.uncounted[0].modes.is_empty());
    assert!(after.total.unwrap().si < before.total.unwrap().si);
}

/// モードの割り当てを消した部品は、そのモードで `uncounted` に出る。
#[test]
fn power_budget_names_the_unassigned_part_with_its_mode() {
    let ws = demo();
    let v = view(
        &ws,
        BudgetKind::Power,
        json!([{ "op": "set_mode_states",
                 "cells": [{ "mode": "DOWNLINK", "part": "GPS", "state": null }] }]),
    );
    let gps = v.uncounted.iter().find(|u| u.part == "GPS").unwrap();
    assert_eq!(gps.modes, ["DOWNLINK"]);
}
