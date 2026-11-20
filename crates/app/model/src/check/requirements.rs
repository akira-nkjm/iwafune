//! 要求（`requirements/<ID>.toml`）の検査: 文・量の次元・制約の形と値の次元、量化のケースの参照、確かめる先（解析・バジェット）。
//! 出典の参照は `sources` が見る。解析の判定のキーは解析が実行のたびに作る文字列なので、ここでは見ず、適合表を作るときに確かめる。

use super::Ctx;
use super::values::check_quantity_dimension;
use crate::design::{
    ANALYSIS_IDS, Constraint, ConstraintKind, EntityKey, Requirement, Verification,
};
use crate::target::Target;
use crate::value::units::parse_dimension_name;

pub(super) fn check_requirements(ctx: &mut Ctx) {
    let design = ctx.design;
    for r in design.requirements.values() {
        let entity = EntityKey::Requirement(r.id.to_string());
        if r.statement.trim().is_empty() {
            report(
                ctx,
                r,
                &entity,
                "statement",
                "requirement.statement",
                "The statement is empty",
            );
        }
        if parse_dimension_name(&r.quantity).is_err() {
            report(
                ctx,
                r,
                &entity,
                "quantity",
                "requirement.quantity",
                format!(
                    "`{}` is not a dimension name (e.g. Mass, Power, Temperature)",
                    r.quantity
                ),
            );
        }
        if r.source.is_none() {
            let target = Some(Target::requirement(r.id.as_str(), Some("source")));
            ctx.warning(
                "requirement.source_missing",
                &entity,
                &["source"],
                target,
                "A requirement should name its source (a source ID in sources/)",
            );
        }
        check_constraint(ctx, r, &entity);
        check_quantify(ctx, r, &entity);
        check_verify(ctx, r, &entity);
    }
}

/// 要求の欄の誤り（error）を位置と対象付きで足す。
fn report(
    ctx: &mut Ctx,
    r: &Requirement,
    entity: &EntityKey,
    field: &str,
    code: &str,
    message: impl Into<String>,
) {
    let keys: Vec<&str> = field.split('.').collect();
    let target = Some(Target::requirement(r.id.as_str(), Some(field)));
    ctx.error(code, entity, &keys, target, message);
}

/// 制約: 向きに合う欄だけが書かれていること、値が `quantity` の次元であること、幅が 0 以上、範囲の下限が上限以下。
fn check_constraint(ctx: &mut Ctx, r: &Requirement, entity: &EntityKey) {
    let c = &r.constraint;
    let written: Vec<&str> = c.quantities().iter().map(|(k, _)| *k).collect();
    for key in Constraint::required_keys(c.kind) {
        if !written.contains(key) {
            let field = format!("constraint.{key}");
            let msg = format!("A `{}` constraint needs `{key}`", c.kind);
            report(
                ctx,
                r,
                entity,
                &field,
                "requirement.constraint_missing",
                msg,
            );
        }
    }
    for key in &written {
        if !Constraint::allowed_keys(c.kind).contains(key) {
            let field = format!("constraint.{key}");
            let msg = format!("`{key}` is not used by a `{}` constraint", c.kind);
            report(ctx, r, entity, &field, "requirement.constraint_extra", msg);
        }
    }
    if parse_dimension_name(&r.quantity).is_err() {
        return;
    }
    for (key, q) in c.quantities() {
        if let Some((sev, code, msg)) = check_quantity_dimension(q, &r.quantity) {
            let field = format!("constraint.{key}");
            let keys = ["constraint", key];
            let target = Some(Target::requirement(r.id.as_str(), Some(&field)));
            ctx.report(sev, code, entity, &keys, target, msg);
        }
    }
    check_bounds(ctx, r, entity);
}

/// 値が読めて次元が合っているときだけ、幅の符号と範囲の向きを見る。
fn check_bounds(ctx: &mut Ctx, r: &Requirement, entity: &EntityKey) {
    let c = &r.constraint;
    let si = |q: &crate::value::Quantity| q.si_as(&r.quantity).ok();
    if c.kind == ConstraintKind::Equal
        && let Some(t) = c.tolerance.as_ref().and_then(si)
        && t < 0.0
    {
        let msg = "The tolerance must be 0 or greater";
        report(
            ctx,
            r,
            entity,
            "constraint.tolerance",
            "requirement.tolerance",
            msg,
        );
    }
    if c.kind == ConstraintKind::Range
        && let (Some(lo), Some(hi)) = (c.min.as_ref().and_then(si), c.max.as_ref().and_then(si))
        && lo > hi
    {
        let msg = "The range minimum is greater than its maximum";
        report(ctx, r, entity, "constraint.min", "requirement.range", msg);
    }
}

/// 量化: `all_cases` とケースの列挙は一緒に書かない。書いたケースは実在する。
fn check_quantify(ctx: &mut Ctx, r: &Requirement, entity: &EntityKey) {
    let q = &r.quantify;
    if q.all_cases && !q.cases.is_empty() {
        let msg = "`all_cases` and `cases` cannot be combined";
        report(
            ctx,
            r,
            entity,
            "quantify.cases",
            "requirement.quantify_conflict",
            msg,
        );
    }
    for case in &q.cases {
        if !ctx.design.cases.contains_key(case.as_str()) {
            let target = Some(Target::requirement(r.id.as_str(), Some("quantify.cases")));
            let msg = format!("Analysis case `{case}` is not defined");
            ctx.error("ref.case", entity, &["quantify", "cases"], target, msg);
        }
    }
}

/// 確かめる先: 解析かバジェットのどちらか 1 つ。解析は実在し、判定のキーは解析がなければ書けない。
/// バジェットの次元は要求の量と合う。
fn check_verify(ctx: &mut Ctx, r: &Requirement, entity: &EntityKey) {
    let v: &Verification = &r.verify;
    if v.budget.is_some() && (v.analysis.is_some() || v.check.is_some()) {
        let msg = "`budget` cannot be combined with `analysis`/`check` (a requirement is verified by one of them)";
        report(
            ctx,
            r,
            entity,
            "verify.budget",
            "requirement.verify_conflict",
            msg,
        );
    }
    if v.check.is_some() && v.analysis.is_none() {
        let msg = "`check` needs the `analysis` that produces it";
        report(
            ctx,
            r,
            entity,
            "verify.check",
            "requirement.verify_incomplete",
            msg,
        );
    }
    if let Some(a) = &v.analysis
        && !ANALYSIS_IDS.contains(&a.as_str())
    {
        let msg = format!("Analysis `{a}` does not exist (see `fune analyze ls`)");
        report(ctx, r, entity, "verify.analysis", "ref.analysis", msg);
    }
    if let Some(kind) = v.budget
        && let (Ok(want), Ok(have)) = (
            parse_dimension_name(&r.quantity),
            parse_dimension_name(kind.dimension()),
        )
        && !want.is_compatible_with(have)
    {
        let msg = format!(
            "The {kind} budget is {}, but the requirement constrains {}",
            kind.dimension(),
            r.quantity
        );
        report(
            ctx,
            r,
            entity,
            "verify.budget",
            "requirement.verify_dimension",
            msg,
        );
    }
}
