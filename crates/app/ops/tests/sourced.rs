//! 出典の付いた値の書き込み（docs/model.md §10.2 の 3）: 1 行 1 要素の形と、要素を変えたときの差分（fixtures/demo-6u に対して計画だけを立てる）。

use fune_model::Id;
use fune_model::value::{Provenance, Scalar, Value};
use fune_ops::{Op, plan};
use fune_store::{FileChange, Snapshot, load};
use std::path::PathBuf;

fn snapshot() -> Snapshot {
    load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u")).unwrap()
}

/// 変わった行（前後の行の多重集合の差）。
fn changed_lines(change: &FileChange) -> (Vec<String>, Vec<String>) {
    let before: Vec<&str> = change.before.as_deref().unwrap_or("").lines().collect();
    let after: Vec<&str> = change.after.as_deref().unwrap_or("").lines().collect();
    let removed = before
        .iter()
        .filter(|l| !after.contains(l))
        .map(|l| l.to_string())
        .collect();
    let added = after
        .iter()
        .filter(|l| !before.contains(l))
        .map(|l| l.to_string())
        .collect();
    (removed, added)
}

/// 書いた本文が正規形（`fune fmt` で変わらない）。
fn assert_canonical(change: &FileChange) {
    let text = change.after.as_deref().unwrap();
    assert_eq!(
        fune_store::format::format_text(&change.path, text).unwrap(),
        text
    );
}

fn set_mass(part: &str, value: f64, unit: &str) -> Op {
    Op::SetValue {
        part: part.into(),
        attr: "mass".into(),
        state: None,
        value: Value::Scalar(Scalar::new(value, unit)),
        from: None,
    }
}

#[test]
fn set_value_changes_exactly_one_line() {
    // 出典の付いた値（`[attr.mass]` の下に 1 行 1 要素）は、出典を保ったまま値を変えると 1 行だけが変わる。
    let snap = snapshot();
    let from = snap.design.parts["RW_X"].attr[&Id::new("mass")]
        .from
        .clone();
    assert!(from.is_some());
    let op = Op::SetValue {
        part: "RW_X".into(),
        attr: "mass".into(),
        state: None,
        value: Value::Scalar(Scalar::new(0.155, "kg")),
        from,
    };
    let p = plan(&snap, &[op]).unwrap();
    assert_eq!(p.changes.len(), 1);
    assert_eq!(p.changes[0].path, "parts/aocs/RW/RW_X.toml");
    let (removed, added) = changed_lines(&p.changes[0]);
    assert_eq!(removed, vec!["value       = 0.152".to_string()]);
    assert_eq!(added, vec!["value       = 0.155".to_string()]);
    let resolved = fune_model::resolve(&p.design);
    assert!((resolved.parts["RW_X"].scalar("mass").unwrap() - 0.155).abs() < 1e-12);
}

#[test]
fn dropping_the_source_writes_one_line_again() {
    let snap = snapshot();
    let p = plan(&snap, &[set_mass("RW_X", 0.155, "kg")]).unwrap();
    let (removed, added) = changed_lines(&p.changes[0]);
    // 出典を付けずに書いた値は、前の値の出典を引き継がない（EM 実測の値ではなくなるため）。
    assert!(removed.contains(&"[attr.mass]".to_string()), "{removed:?}");
    assert_eq!(
        added,
        vec![
            "[attr]".to_string(),
            r#"mass = { value = 0.155, unit = "kg" }"#.to_string()
        ]
    );
    assert_canonical(&p.changes[0]);
}

#[test]
fn adding_a_source_spreads_the_value_over_lines() {
    let snap = snapshot();
    let op = Op::SetValue {
        part: "HTR_BAT".into(),
        attr: "specific_heat".into(),
        state: None,
        value: Value::Scalar(Scalar::new(1100.0, "J/(kg*K)")),
        from: Some(Provenance {
            source: Some("tbd".into()),
            at: None,
            note: Some("仮の値".into()),
        }),
    };
    let p = plan(&snap, &[op]).unwrap();
    let text = p.changes[0].after.as_deref().unwrap();
    assert!(
        text.contains("[attr.specific_heat]\nvalue       = 1100.0\nunit        = \"J/(kg*K)\"\nfrom.source = \"tbd\"\nfrom.note   = \"仮の値\"\n"),
        "{text}"
    );
    assert_canonical(&p.changes[0]);
}

#[test]
fn a_sourced_state_cell_uses_dotted_keys() {
    let snap = snapshot();
    let cell = |from: Option<&str>| Op::SetValue {
        part: "HTR_BAT".into(),
        attr: "power".into(),
        state: Some("on".into()),
        value: Value::Scalar(Scalar::new(2.5, "W")),
        from: from.map(|s| Provenance {
            source: Some(s.into()),
            at: None,
            note: None,
        }),
    };
    // 出典を保てば値の 1 行だけが変わる。
    let p = plan(&snap, &[cell(Some("sys_judgement"))]).unwrap();
    let (removed, added) = changed_lines(&p.changes[0]);
    assert_eq!(removed, vec!["on.value       = 3.0".to_string()]);
    assert_eq!(added, vec!["on.value       = 2.5".to_string()]);
    // 出典を外すとセルは 1 行に戻り、表の `=` も揃え直す（fmt と同じ）。
    let p = plan(&snap, &[cell(None)]).unwrap();
    let text = p.changes[0].after.as_deref().unwrap();
    assert!(
        text.contains("[attr.power]\noff = { value = 0.0, unit = \"W\" }\non  = { value = 2.5, unit = \"W\" }\n"),
        "{text}"
    );
    assert_canonical(&p.changes[0]);
}
