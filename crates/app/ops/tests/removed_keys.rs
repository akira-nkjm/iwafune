//! 置き換える Op（`SetCase` など）が既存のレコードから落としたキーの一覧（`Plan::removed`）と、
//! 1 つの欄だけを書く Op（`SetCaseField`）が何も落とさないこと（fixtures/demo-6u に対して計画だけを立てる）。

use fune_model::design::{Case, CaseField};
use fune_model::value::Quantity;
use fune_ops::{Op, OpError, plan, to_cli};
use fune_store::{Snapshot, load};
use std::path::PathBuf;

fn snapshot() -> Snapshot {
    load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")).unwrap()
}

/// 既存のケースを、`beta`・`orbits`・`thermal_case`・`side` を付けずに置き換えると、それらが一覧に載る（書いてあった欄だけ）。
/// 付け足すだけの編集と、新しく作るレコードは何も載せない。
#[test]
fn a_replaced_case_lists_the_keys_it_dropped() {
    let snap = snapshot();
    let old = snap.design.cases["eclipse_worst"].clone();
    assert!(old.beta.is_some() && old.orbits.is_some() && old.thermal_case.is_some());
    let thin = Case {
        beta: None,
        orbits: None,
        thermal_case: None,
        side: None,
        ..old.clone()
    };
    let p = plan(&snap, &[Op::SetCase { case: thin }]).unwrap();
    assert_eq!(p.removed.len(), 1);
    assert_eq!(p.removed[0].path, "cases/eclipse_worst.toml");
    // ファイルの順（side → beta → orbits → thermal_case）
    assert_eq!(
        p.removed[0].keys,
        ["side", "beta", "orbits", "thermal_case"]
    );
    // 同じ内容で置き換えても、値を替えても、何も消えない
    let same = plan(&snap, &[Op::SetCase { case: old.clone() }]).unwrap();
    assert!(same.removed.is_empty());
    let changed = Case {
        orbits: Some(99),
        ..old.clone()
    };
    let p = plan(&snap, &[Op::SetCase { case: changed }]).unwrap();
    assert!(p.removed.is_empty());
    // 新しいケースは何も置き換えない
    let fresh = Case {
        id: "brand_new".into(),
        label: None,
        description: None,
        mode: None,
        phase: None,
        side: None,
        thermal_case: None,
        beta: None,
        orbits: None,
        sun_distance: None,
        epoch: None,
    };
    let p = plan(&snap, &[Op::SetCase { case: fresh }]).unwrap();
    assert!(p.removed.is_empty());
}

/// 入れ子の表の中のキー（マヌーバの `[target]`）も消えたキーとして載る。表ごと無くなれば表のキー。
#[test]
fn nested_keys_of_a_replaced_maneuver_are_listed() {
    let snap = snapshot();
    let (id, old) = snap
        .design
        .maneuvers
        .iter()
        .find(|(_, m)| m.target.altitude.is_some() && m.delta_v.is_none())
        .map(|(id, m)| (id.to_string(), m.clone()))
        .expect("a maneuver with a target altitude");
    let mut thin = old.clone();
    thin.target.altitude = None;
    let p = plan(&snap, &[Op::SetManeuver { maneuver: thin }]).unwrap();
    let removed = &p.removed[0];
    assert_eq!(removed.path, format!("maneuvers/{id}.toml"));
    assert!(
        removed.keys.iter().all(|k| k.starts_with("target")),
        "{removed:?}"
    );
    assert!(
        removed.keys.contains(&"target.altitude".to_owned()) || removed.keys == ["target"],
        "{removed:?}"
    );
}

/// 同じ Op の列の中で消して書き戻したキーは、消えたと言わない。
#[test]
fn a_key_written_back_by_a_later_op_is_not_reported() {
    let snap = snapshot();
    let old = snap.design.cases["eclipse_worst"].clone();
    let thin = Case {
        orbits: None,
        ..old.clone()
    };
    let p = plan(
        &snap,
        &[Op::SetCase { case: thin }, Op::SetCase { case: old }],
    )
    .unwrap();
    assert!(p.removed.is_empty(), "{:?}", p.removed);
}

/// `SetCaseField` は書いた欄だけを変える（ほかの欄は残り、何も消えない）。外す欄は消えるが、置き換えではないので載せない。
#[test]
fn a_case_field_edit_keeps_every_other_field() {
    let snap = snapshot();
    let before = snap.design.cases["eclipse_worst"].clone();
    let set = Op::SetCaseField {
        case: "eclipse_worst".into(),
        field: CaseField::Beta(Some(Quantity::new(30.0, "deg"))),
    };
    let p = plan(&snap, std::slice::from_ref(&set)).unwrap();
    let after = &p.design.cases["eclipse_worst"];
    assert_eq!(after.beta, Some(Quantity::new(30.0, "deg")));
    assert_eq!(after.orbits, before.orbits);
    assert_eq!(after.thermal_case, before.thermal_case);
    assert_eq!(after.label, before.label);
    assert!(p.removed.is_empty());
    assert_eq!(
        to_cli(&set),
        ["fune set cases.eclipse_worst.beta \"30 deg\""]
    );
    let unset = Op::SetCaseField {
        case: "eclipse_worst".into(),
        field: CaseField::Orbits(None),
    };
    let p = plan(&snap, std::slice::from_ref(&unset)).unwrap();
    assert!(p.design.cases["eclipse_worst"].orbits.is_none());
    assert_eq!(p.design.cases["eclipse_worst"].beta, before.beta);
    assert_eq!(to_cli(&unset), ["fune unset cases.eclipse_worst.orbits"]);
    let missing = Op::SetCaseField {
        case: "NO_SUCH_CASE".into(),
        field: CaseField::Orbits(None),
    };
    assert!(matches!(
        plan(&snap, &[missing]),
        Err(OpError::NotFound { .. })
    ));
}
