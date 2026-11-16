//! 配分とマージン方針の検査（配分先の参照・次元・合計 ≤ 上限 × (1 − システムマージン)）。

use super::Ctx;
use super::values::check_quantity_dimension;
use crate::design::{Allocation, BudgetKind, EntityKey, Maturity, MaturityFactors};
use crate::diagnostic::Severity;
use crate::path::SettingPath;
use crate::target::Target;

/// `budget.<…>` の設定の対象。
fn budget_target(segments: &[&str]) -> Option<Target> {
    Some(SettingPath::budget(segments).into())
}

pub(super) fn check_budget(ctx: &mut Ctx) {
    let design = ctx.design;
    for kind in BudgetKind::ALL {
        // 設定の無い種類（バジェットを持たないプロジェクトなど）は知らせない。設定があって上限の無いときだけ
        if design
            .budget
            .allocation
            .get(*kind)
            .is_some_and(|a| a.limit.is_none())
        {
            ctx.warning(
                "budget.limit_unset",
                &EntityKey::Allocation,
                &[kind.as_str(), "limit"],
                budget_target(&[kind.as_str(), "limit"]),
                format!("The limit of {} is not set", kind.as_str()),
            );
        }
        if let Some(alloc) = design.budget.allocation.get(*kind) {
            check_allocation(ctx, *kind, alloc);
        }
    }
    let policy = &design.budget.policy;
    check_factors(ctx, &policy.growth, "growth");
    check_factors(ctx, &policy.uncertainty, "uncertainty");
    if !(0.0..=1.0).contains(&policy.harness.allowance) {
        ctx.report(
            Severity::Error,
            "budget.policy",
            &EntityKey::Policy,
            &["harness", "allowance"],
            budget_target(&["policy", "harness", "allowance"]),
            "The harness allowance fraction must be from 0 to 1",
        );
    }
}

fn check_factors(ctx: &mut Ctx, factors: &MaturityFactors, key: &str) {
    for maturity in Maturity::ALL {
        let name = maturity.as_str();
        if factors
            .get(*maturity)
            .is_some_and(|v| !(0.0..=1.0).contains(&v))
        {
            ctx.report(
                Severity::Error,
                "budget.policy",
                &EntityKey::Policy,
                &[key, name],
                budget_target(&["policy", key, name]),
                "The factor must be from 0 to 1",
            );
        }
    }
}

/// 1 種類の配分（上限と配分先の次元・配分先の参照・合計 ≤ 上限 × (1 − マージン)）。
fn check_allocation(ctx: &mut Ctx, kind: BudgetKind, alloc: &Allocation) {
    let design = ctx.design;
    let entity = EntityKey::Allocation;
    let name = kind.as_str();
    let dim = kind.dimension();
    if let Some(limit) = &alloc.limit
        && let Some((sev, code, msg)) = check_quantity_dimension(limit, dim)
    {
        let target = budget_target(&[name, "limit"]);
        ctx.report(sev, code, &entity, &[name, "limit"], target, msg);
        return;
    }
    if !(0.0..1.0).contains(&alloc.system_margin) {
        ctx.error(
            "budget.margin",
            &entity,
            &[name, "system_margin"],
            budget_target(&[name, "system_margin"]),
            "The system margin must be at least 0 and less than 1",
        );
    }
    let mut total = 0.0;
    for (node, q) in &alloc.alloc {
        let keys = [name, "alloc", node.as_str()];
        let path = budget_target(&[name, "alloc", node.as_str()]);
        if node.parse::<crate::design::AnalysisSystem>().is_err()
            && !design.parts.contains_key(node.as_str())
        {
            ctx.error(
                "ref.node",
                &entity,
                &keys,
                path.clone(),
                format!("Allocation target `{node}` is neither a subsystem ID nor a part"),
            );
        }
        match check_quantity_dimension(q, dim) {
            Some((sev, code, msg)) => ctx.report(sev, code, &entity, &keys, path, msg),
            None => total += q.si().unwrap_or(0.0),
        }
    }
    let Some(limit) = alloc.limit.as_ref().and_then(|q| q.si().ok()) else {
        return;
    };
    let available = limit * (1.0 - alloc.system_margin);
    if total > available + 1e-12 {
        ctx.warning(
            "budget.over_allocated",
            &entity,
            &[name, "alloc"],
            budget_target(&[name, "alloc"]),
            format!("The allocation total {total} exceeds limit × (1 − margin) = {available} (SI)"),
        );
    }
}
