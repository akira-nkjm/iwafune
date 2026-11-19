//! モード・ネット・ハーネス・ケースの検査（参照・状態・次元）。出典は `super::sources`。ID の規則と一意性は `super::ids`、図は `super::views`。

use super::Ctx;
use super::values::check_quantity_dimension;
use crate::design::{Design, EntityKey, NetKind, UncertaintySide};
use crate::path::PortRef;
use crate::target::Target;
use crate::value::{Quantity, Scalar, Value};

/// ポートの参照（部品が存在するか。ポートの所属が違えば warning）。対象は参照を書いたキー（`target`）。
/// `部品.ポート` の形でない書き方は読み込み（`fune_model::path::port_text`）が誤りにする。
fn check_port_ref(
    ctx: &mut Ctx,
    entity: &EntityKey,
    key: &str,
    target: Target,
    port: &PortRef,
    net: Option<&str>,
) {
    let design = ctx.design;
    let path = Some(target);
    let Some(part) = design.parts.get(port.part.as_str()) else {
        ctx.error(
            "ref.part",
            entity,
            &[key],
            path,
            format!("Part `{}` is not defined", port.part),
        );
        return;
    };
    match (part.ports.get(&port.port), net) {
        (None, _) => ctx.warning(
            "ref.port",
            entity,
            &[key],
            path,
            format!("Part `{}` does not declare port `{}`", port.part, port.port),
        ),
        (Some(binding), Some(net)) if binding.net.as_ref().is_some_and(|n| n.as_str() != net) => {
            ctx.warning(
                "net.port_mismatch",
                entity,
                &[key],
                path,
                format!("Port `{port}` does not belong to net `{net}`"),
            );
        }
        _ => {}
    }
}

pub(super) fn check_dim(
    ctx: &mut Ctx,
    entity: &EntityKey,
    key: &str,
    target: Target,
    q: Option<&Quantity>,
    dim: &str,
) {
    if let Some((sev, code, msg)) = q.and_then(|q| check_quantity_dimension(q, dim)) {
        ctx.report(sev, code, entity, &[key], Some(target), msg);
    }
}

/// ネット: 供給元・マスターのポート、量の次元。
pub(super) fn check_nets(ctx: &mut Ctx) {
    let design = ctx.design;
    for net in design.nets.values() {
        let entity = EntityKey::Net(net.id.to_string());
        let id = net.id.as_str();
        for (key, port) in [("source", &net.source), ("master", &net.master)] {
            if let Some(port) = port {
                let target = Target::net(id, Some(key));
                check_port_ref(ctx, &entity, key, target, port, Some(id));
            }
        }
        for (key, q, dim) in [
            ("rail", net.rail.as_ref(), "Voltage"),
            ("limit", net.limit.as_ref(), "ElectricCurrent"),
            ("bitrate", net.bitrate.as_ref(), "DataRate"),
        ] {
            check_dim(ctx, &entity, key, Target::net(id, Some(key)), q, dim);
        }
        if net.kind != NetKind::Power && (net.rail.is_some() || net.source.is_some()) {
            ctx.warning(
                "net.kind",
                &entity,
                &["kind"],
                Some(Target::net(id, None)),
                "rail / source is set on a net that is not a power net",
            );
        }
    }
}

/// ハーネス: 両端のポート・ネット・線材・長さ。
pub(super) fn check_harnesses(ctx: &mut Ctx) {
    let design = ctx.design;
    for h in design.harnesses.values() {
        let entity = EntityKey::Harness(h.id.to_string());
        let id = h.id.as_str();
        for (key, port) in [("from", &h.from), ("to", &h.to)] {
            let target = Target::harness(id, Some(key));
            check_port_ref(ctx, &entity, key, target, port, None);
        }
        for net in &h.nets {
            if !design.nets.contains_key(net.as_str()) {
                ctx.error(
                    "ref.net",
                    &entity,
                    &["nets"],
                    Some(Target::harness(id, None)),
                    format!("Net `{net}` is not defined"),
                );
            }
        }
        if let Some(wire) = &h.wire
            && !design.catalog.contains_key(wire)
        {
            ctx.error(
                "ref.catalog",
                &entity,
                &["wire"],
                Some(Target::harness(id, None)),
                format!("Wire catalog item `{wire}` is not defined"),
            );
        }
        check_dim(
            ctx,
            &entity,
            "length",
            Target::harness(id, Some("length")),
            h.length.as_ref(),
            "Length",
        );
    }
}

/// ケースの参照（モード・熱の環境条件）と β の次元。
pub(super) fn check_cases(ctx: &mut Ctx) {
    let design = ctx.design;
    for case in design.cases.values() {
        let entity = EntityKey::Case(case.id.to_string());
        if let Some(mode) = &case.mode
            && !design.modes.contains_key(mode.as_str())
        {
            ctx.error(
                "ref.mode",
                &entity,
                &["mode"],
                Some(Target::case(case.id.as_str(), None)),
                format!("Mode `{mode}` is not defined"),
            );
        }
        let thermal = design.analysis.thermal.as_ref();
        if let Some(tc) = &case.thermal_case
            && !thermal.is_some_and(|t| t.cases.contains_key(tc))
        {
            ctx.error(
                "ref.thermal_case",
                &entity,
                &["thermal_case"],
                Some(Target::case(case.id.as_str(), None)),
                format!("Thermal environment `{tc}` is not in analysis/thermal.toml"),
            );
        }
        if let Some(side @ (UncertaintySide::Min | UncertaintySide::Max)) = case.side
            && !any_value_has_bound(design, side)
        {
            ctx.info(
                "case.side_without_bounds",
                &entity,
                &["side"],
                Some(Target::case(case.id.as_str(), Some("side"))),
                format!(
                    "side = {side} has no effect: no value in the design has a `{side}` bound, so the analyses read the nominal values"
                ),
            );
        }
        check_dim(
            ctx,
            &entity,
            "beta",
            Target::case(case.id.as_str(), Some("beta")),
            case.beta.as_ref(),
            "Angle",
        );
    }
}

/// 設計のどこかの値（部品・カタログの属性。状態ごとの値を含む）が `side` の側の値（`min` / `max`）を持つか。
fn any_value_has_bound(design: &Design, side: UncertaintySide) -> bool {
    let has = |s: &Scalar| match side {
        UncertaintySide::Min => s.min.is_some(),
        UncertaintySide::Max => s.max.is_some(),
        UncertaintySide::Nominal => false,
    };
    let in_value = |v: &Value| match v {
        Value::Scalar(s) => has(s),
        Value::ByState(m) => m.values().any(|a| has(&a.value)),
        _ => false,
    };
    let parts = design.parts.values().flat_map(|p| p.attr.values());
    let items = design.catalog.values().flat_map(|c| c.attr.values());
    parts.chain(items).any(|a| in_value(&a.value))
}
