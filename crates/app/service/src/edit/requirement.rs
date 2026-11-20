//! `fune requirement add|set` に打った引数 → `CreateRequirement`・`SetRequirement`（`POST /ops/from_command` の
//! `requirement_add`・`requirement_set`）。**要求の旗の読み方はここだけ**（逆の向き = 編集の欄の初期値は
//! `query::requirements::requirement_text`）。

use super::text::{bool_from_text, given, invalid, list_from_text, quantity_from_text};
use crate::dto::RequirementText;
use crate::error::Result;
use fune_model::design::{Constraint, Quantification, Requirement, Verification};
use fune_model::value::Quantity;
use fune_ops::Op;

/// 打った引数 → `CreateRequirement`（`create`）か `SetRequirement`。
pub fn requirement_op(t: &RequirementText, create: bool) -> Result<Op> {
    let requirement = requirement_of(t)?;
    Ok(if create {
        Op::CreateRequirement { requirement }
    } else {
        Op::SetRequirement { requirement }
    })
}

fn quantity(text: &Option<String>) -> Result<Option<Quantity>> {
    given(text).map(quantity_from_text).transpose()
}

fn requirement_of(t: &RequirementText) -> Result<Requirement> {
    let owned = |x: &Option<String>| given(x).map(str::to_owned);
    let cases = given(&t.cases).map(list_from_text).unwrap_or_default();
    Ok(Requirement {
        id: t.id.trim().into(),
        label: owned(&t.label),
        statement: t.statement.trim().to_owned(),
        statement_en: owned(&t.statement_en),
        quantity: t.quantity.trim().to_owned(),
        source: owned(&t.source).map(Into::into),
        constraint: Constraint {
            kind: t.kind.trim().parse().map_err(invalid)?,
            value: quantity(&t.value)?,
            tolerance: quantity(&t.tolerance)?,
            min: quantity(&t.min)?,
            max: quantity(&t.max)?,
        },
        quantify: Quantification {
            all_cases: bool_from_text(&t.all_cases)?.unwrap_or(false),
            cases: cases.into_iter().map(Into::into).collect(),
        },
        verify: Verification {
            analysis: owned(&t.analysis),
            check: owned(&t.check),
            budget: given(&t.budget)
                .map(|b| b.parse().map_err(invalid))
                .transpose()?,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::requirements::requirement_text;

    fn text() -> RequirementText {
        RequirementText {
            id: "R_T".into(),
            statement: " The temperature shall stay in range. ".into(),
            quantity: "Temperature".into(),
            kind: "range".into(),
            min: Some("0 degC".into()),
            max: Some(" 40 degC".into()),
            source: Some("sys".into()),
            cases: Some("a, b".into()),
            all_cases: Some(String::new()),
            analysis: Some("thermal.steady".into()),
            check: Some("range_low:BAT:operating".into()),
            ..RequirementText::default()
        }
    }

    #[test]
    fn typed_text_becomes_a_requirement_and_back() {
        let Op::SetRequirement { requirement } = requirement_op(&text(), false).unwrap() else {
            panic!("not SetRequirement");
        };
        assert_eq!(
            requirement.statement,
            "The temperature shall stay in range."
        );
        assert_eq!(
            requirement.constraint.max,
            Some(Quantity::new(40.0, "degC"))
        );
        assert_eq!(requirement.quantify.cases.len(), 2);
        assert!(!requirement.quantify.all_cases);
        let back = requirement_op(&requirement_text(&requirement), true).unwrap();
        assert_eq!(back, Op::CreateRequirement { requirement });
    }

    #[test]
    fn bad_text_is_rejected() {
        let mut t = text();
        t.kind = "about".into();
        assert!(requirement_op(&t, false).is_err());
        let mut t = text();
        t.max = Some("forty".into());
        assert!(requirement_op(&t, false).is_err());
        let mut t = text();
        t.budget = Some("fuel".into());
        assert!(requirement_op(&t, false).is_err());
        let mut t = text();
        t.all_cases = Some("yes".into());
        assert!(requirement_op(&t, false).is_err());
    }
}
