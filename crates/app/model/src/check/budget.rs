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
    for (kind, alloc) in design.budget.allocation.iter() {
        check_allocation(ctx, kind, alloc);
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
            "ハーネスの見込み割合は 0〜1 にします",
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
                "係数は 0〜1 にします",
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
    if let Some((sev, code, msg)) = check_quantity_dimension(&alloc.limit, dim) {
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
            "システムマージンは 0 以上 1 未満にします",
        );
    }
    let mut total = 0.0;
    for (node, q) in &alloc.alloc {
        let keys = [name, "alloc", node.as_str()];
        let path = budget_target(&[name, "alloc", node.as_str()]);
        if !design.node_exists(node) {
            ctx.error(
                "ref.node",
                &entity,
                &keys,
                path.clone(),
                format!("配分先 `{node}` はグループにも部品にもありません"),
            );
        }
        match check_quantity_dimension(q, dim) {
            Some((sev, code, msg)) => ctx.report(sev, code, &entity, &keys, path, msg),
            None => total += q.si().unwrap_or(0.0),
        }
    }
    let limit = alloc.limit.si().unwrap_or(0.0);
    let available = limit * (1.0 - alloc.system_margin);
    if total > available + 1e-12 {
        ctx.warning(
            "budget.over_allocated",
            &entity,
            &[name, "alloc"],
            budget_target(&[name, "alloc"]),
            format!("配分の合計 {total} が上限 × (1 − マージン) = {available}（SI）を超えています"),
        );
    }
}
