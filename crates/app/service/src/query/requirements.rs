//! 要求 → 表示（ID 順）。制約の文（`≤ 12.0 kg`）の整形もここだけ（適合表も使う）。

use super::values::authored_quantity;
use super::views::file_of;
use crate::dto::{ConstraintView, RequirementText, RequirementView};
use fune_model::Target;
use fune_model::design::{Constraint, ConstraintKind, Design, EntityKey, Requirement};
use fune_model::value::{Quantity, quantity_to_text};
use std::collections::BTreeMap;

/// 編集の欄（`RequirementText` の欄）→ 対象のキー（`Target::requirement` の `field`。検査が付ける対象と同じ）。
const FIELDS: &[(&str, &str)] = &[
    ("statement", "statement"),
    ("quantity", "quantity"),
    ("source", "source"),
    ("kind", "constraint.kind"),
    ("value", "constraint.value"),
    ("tolerance", "constraint.tolerance"),
    ("min", "constraint.min"),
    ("max", "constraint.max"),
    ("cases", "quantify.cases"),
    ("all_cases", "quantify.all_cases"),
    ("analysis", "verify.analysis"),
    ("check", "verify.check"),
    ("budget", "verify.budget"),
    ("input", "input"),
];

/// 要求の一覧（ID 順）。
pub(crate) fn requirements(design: &Design) -> Vec<RequirementView> {
    design
        .requirements
        .values()
        .map(|r| view(design, r))
        .collect()
}

fn view(design: &Design, r: &Requirement) -> RequirementView {
    let id = r.id.as_str();
    RequirementView {
        id: id.to_owned(),
        label: fune_model::display_name::name_or_id(r.label.as_deref(), id),
        statement: r.statement.clone(),
        statement_en: r.statement_en.clone(),
        quantity: r.quantity.clone(),
        source: r.source.as_ref().map(ToString::to_string),
        constraint: constraint_view(r),
        quantify: r.quantify.clone(),
        verify: r.verify.clone(),
        input: r.input,
        text: requirement_text(r),
        field_paths: FIELDS
            .iter()
            .map(|(field, key)| {
                let target = Target::requirement(id, Some(key));
                ((*field).to_owned(), target.to_string())
            })
            .collect::<BTreeMap<_, _>>(),
        file: file_of(design, EntityKey::Requirement(id.to_owned())),
    }
}

/// 制約の向きと、値を書かれた単位のまま整形した文（`≤ 12.0 kg`・`= 20.0 degC ± 2.0 K`・`0.0 degC … 40.0 degC`）。
pub(crate) fn constraint_view(r: &Requirement) -> ConstraintView {
    let c = &r.constraint;
    let show = |q: &Option<Quantity>| {
        q.as_ref()
            .map(|q| authored_quantity(q, Some(&r.quantity)).display)
            .unwrap_or_else(|| "?".to_owned())
    };
    let text = match c.kind {
        ConstraintKind::AtMost | ConstraintKind::AtLeast => {
            format!("{} {}", c.kind.symbol(), show(&c.value))
        }
        ConstraintKind::Equal => match &c.tolerance {
            Some(_) => format!("= {} ± {}", show(&c.value), show(&c.tolerance)),
            None => format!("= {}", show(&c.value)),
        },
        ConstraintKind::Range => format!("{} … {}", show(&c.min), show(&c.max)),
    };
    ConstraintView { kind: c.kind, text }
}

/// 今の要求 → 打った引数（`edit::requirement_op` で読み直すと同じ要求になる。GUI の編集の欄の初期値）。
pub(crate) fn requirement_text(r: &Requirement) -> RequirementText {
    let q = |x: &Option<Quantity>| x.as_ref().map(quantity_to_text);
    let c: &Constraint = &r.constraint;
    let cases = (!r.quantify.cases.is_empty()).then(|| {
        r.quantify
            .cases
            .iter()
            .map(|c| c.as_str())
            .collect::<Vec<_>>()
            .join(",")
    });
    RequirementText {
        id: r.id.to_string(),
        statement: r.statement.clone(),
        statement_en: r.statement_en.clone(),
        label: r.label.clone(),
        quantity: r.quantity.clone(),
        source: r.source.as_ref().map(ToString::to_string),
        kind: c.kind.as_str().to_owned(),
        value: q(&c.value),
        tolerance: q(&c.tolerance),
        min: q(&c.min),
        max: q(&c.max),
        all_cases: r.quantify.all_cases.then(|| "true".to_owned()),
        cases,
        analysis: r.verify.analysis.clone(),
        check: r.verify.check.clone(),
        budget: r.verify.budget.map(|b| b.as_str().to_owned()),
        input: r.input.map(|i| i.as_str().to_owned()),
    }
}
