//! 不足入力の対象（`MissingInput.part`）が、設計にある部品だけを指すこと。
//! 「どれかの部品の属性が足りない」・解析ケース・設定の不足に、部品でない名前（`<部品>`・`cases`）を入れない。

use fune_model::design::Case;
use fune_model::id::Id;
use fune_model::{Design, Resolved, Target};
use fune_projection::{Context, MissingInput, ProjectionError, entries, project_json};
use std::path::Path;

fn demo() -> Design {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/demo-6u");
    fune_store::load(&root).expect("デモを読めること").design
}

/// すべての解析の不足入力（組み立てられた解析は飛ばす）。
fn missing_of(design: &Design, resolved: &Resolved, case: Option<&str>) -> Vec<MissingInput> {
    let ctx = match Context::new(design, resolved, case) {
        Ok(ctx) => ctx,
        Err(m) => return m.into_vec(),
    };
    let mut out = Vec::new();
    for e in entries() {
        if let Err(ProjectionError::Missing(m)) = project_json(e.id, &ctx) {
            out.extend(m.into_vec());
        }
    }
    out
}

fn assert_parts_exist(design: &Design, missing: &[MissingInput]) {
    assert!(!missing.is_empty());
    for m in missing {
        assert_eq!(m.part.as_deref(), m.target.as_ref().and_then(Target::part));
        if let Some(part) = &m.part {
            assert!(
                design.parts.contains_key(part.as_str()),
                "不足入力 `{}` の part `{part}` は設計にありません",
                m.path
            );
        }
    }
}

/// 部品の無い設計: 「消費電力を持つ部品が無い」などは、部品 ID を持たない。
#[test]
fn missing_inputs_of_an_empty_design_name_no_parts() {
    let design = Design::default();
    let resolved = fune_model::resolve(&design);
    let missing = missing_of(&design, &resolved, None);
    assert_parts_exist(&design, &missing);
    let power = missing.iter().find(|m| m.path == "<部品>.power");
    assert_eq!(
        power.and_then(|m| m.target.clone()),
        Some(Target::any_part("power"))
    );
}

/// 解析ケースの不足（無いケース・次元の違う β）は、部品 ID を持たない。
#[test]
fn missing_case_inputs_name_no_parts() {
    let mut design = demo();
    let mut case: Case =
        serde_json::from_value(serde_json::json!({ "beta": { "value": 1.0, "unit": "kg" } }))
            .unwrap();
    case.id = Id::new("BAD_BETA");
    design.cases.insert(Id::new("BAD_BETA"), case);
    let resolved = fune_model::resolve(&design);
    let bad_beta = missing_of(&design, &resolved, Some("BAD_BETA"));
    assert_parts_exist(&design, &bad_beta);
    let beta = Some(Target::case("BAD_BETA", Some("beta")));
    assert!(
        bad_beta
            .iter()
            .any(|m| m.target == beta && m.path == "cases.BAD_BETA.beta")
    );
    let unknown = missing_of(&design, &resolved, Some("NO_SUCH_CASE"));
    assert_parts_exist(&design, &unknown);
    let case = Some(Target::case("NO_SUCH_CASE", None));
    assert!(
        unknown
            .iter()
            .any(|m| m.target == case && m.path == "cases.NO_SUCH_CASE")
    );
}
