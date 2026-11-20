//! 要求の Op のコマンド（`fune requirement add|set|rm`）。

use super::text::Cmd;
use crate::op::Op;
use fune_model::design::Requirement;
use fune_model::value::{Quantity, quantity_to_text};

/// 要求の Op のコマンド。該当しなければ None。
pub(super) fn requirement_op(op: &Op) -> Option<Vec<String>> {
    Some(vec![match op {
        Op::CreateRequirement { requirement } => requirement_cmd("add", requirement),
        Op::SetRequirement { requirement } => requirement_cmd("set", requirement),
        Op::RemoveRequirement { id } => Cmd::new(&["fune", "requirement", "rm", id]).done(),
        _ => return None,
    }])
}

/// `fune requirement add|set <ID> --statement … --quantity Mass --kind at_most --value "12 kg" …`
/// （旗は `fune_service` の `RequirementText` と 1 対 1）。
fn requirement_cmd(verb: &str, r: &Requirement) -> String {
    let q = |x: &Option<Quantity>| x.as_ref().map(quantity_to_text);
    let c = &r.constraint;
    let cases = (!r.quantify.cases.is_empty()).then(|| {
        r.quantify
            .cases
            .iter()
            .map(|c| c.as_str())
            .collect::<Vec<_>>()
            .join(",")
    });
    let all = r.quantify.all_cases.then_some("true");
    Cmd::new(&["fune", "requirement", verb, r.id.as_str()])
        .flag("statement", Some(&r.statement))
        .flag("statement-en", r.statement_en.as_deref())
        .flag("label", r.label.as_deref())
        .flag("quantity", Some(&r.quantity))
        .flag("source", r.source.as_ref().map(|s| s.as_str()))
        .flag("kind", Some(c.kind.as_str()))
        .flag("value", q(&c.value).as_deref())
        .flag("tolerance", q(&c.tolerance).as_deref())
        .flag("min", q(&c.min).as_deref())
        .flag("max", q(&c.max).as_deref())
        .flag("cases", cases.as_deref())
        .flag("analysis", r.verify.analysis.as_deref())
        .flag("check", r.verify.check.as_deref())
        .flag("budget", r.verify.budget.map(|b| b.as_str()))
        .flag("all-cases", all)
        .done()
}
