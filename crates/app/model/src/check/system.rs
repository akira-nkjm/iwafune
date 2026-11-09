//! モード・ネット・ハーネス・ケース・出典の検査。

use super::Ctx;
use super::values::check_quantity_dimension;
use crate::design::{EntityKey, Mode, NetKind, StateAssign};
use crate::diagnostic::Severity;
use crate::id::validate_entity_id;
use crate::path::{AttrPath, ParamPath, PortRef};
use crate::target::Target;
use crate::value::{Provenance, Quantity, Value};

/// モード: 部品の参照・状態の語彙・時間割合・表に無い部品。
pub(super) fn check_modes(ctx: &mut Ctx) {
    let design = ctx.design;
    for mode in design.modes.values() {
        let entity = EntityKey::Mode(mode.id.to_string());
        if let Err(e) = validate_entity_id(mode.id.as_str()) {
            ctx.error(
                "id.invalid",
                &entity,
                &[],
                Some(Target::mode(mode.id.as_str(), None)),
                e.to_string(),
            );
        }
        for (part_id, assign) in &mode.states {
            check_mode_row(ctx, mode, &entity, part_id.as_str(), assign);
        }
        for part in design.parts.values() {
            if !design.part_states(part).is_empty() && !mode.states.contains_key(part.id.as_str()) {
                ctx.warning(
                    "mode.unassigned",
                    &entity,
                    &["states"],
                    Some(Target::mode(mode.id.as_str(), Some(part.id.as_str()))),
                    format!(
                        "モード `{}` に部品 `{}` の状態がありません（0 W とはみなしません）",
                        mode.id, part.id
                    ),
                );
            }
        }
    }
}

/// モードの表の 1 行（部品の参照・状態の語彙・時間割合）。
fn check_mode_row(
    ctx: &mut Ctx,
    mode: &Mode,
    entity: &EntityKey,
    part_id: &str,
    assign: &StateAssign,
) {
    let design = ctx.design;
    let keys = ["states", part_id];
    let path = Some(Target::mode(mode.id.as_str(), Some(part_id)));
    let Some(part) = design.parts.get(part_id) else {
        let message = format!("モード `{}` の部品 `{part_id}` がありません", mode.id);
        ctx.error("ref.part", entity, &keys, path, message);
        return;
    };
    let states = design.part_states(part);
    for state in std::iter::once(&assign.state).chain(assign.duty.keys()) {
        if !states.iter().any(|s| s == state) {
            let message = format!(
                "`{part_id}` の状態 `{state}` は種別にありません（{}）",
                states.join(" / ")
            );
            ctx.error("state.unknown", entity, &keys, path.clone(), message);
        }
    }
    let total: f64 = assign.duty.values().sum();
    if assign.duty.values().any(|d| !(0.0..=1.0).contains(d))
        || total > 1.0 + super::SHARE_TOLERANCE
    {
        let message =
            format!("`{part_id}` の時間割合は 0〜1 で、合計 1 以下にします（合計 {total}）");
        ctx.error("mode.duty", entity, &keys, path, message);
    }
}

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
            format!("部品 `{}` がありません", port.part),
        );
        return;
    };
    match (part.ports.get(&port.port), net) {
        (None, _) => ctx.warning(
            "ref.port",
            entity,
            &[key],
            path,
            format!(
                "部品 `{}` にポート `{}` が書かれていません",
                port.part, port.port
            ),
        ),
        (Some(binding), Some(net)) if binding.net.as_ref().is_some_and(|n| n.as_str() != net) => {
            ctx.warning(
                "net.port_mismatch",
                entity,
                &[key],
                path,
                format!("ポート `{port}` はネット `{net}` に属していません"),
            );
        }
        _ => {}
    }
}

fn check_dim(
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
        if let Err(e) = validate_entity_id(id) {
            ctx.error(
                "id.invalid",
                &entity,
                &[],
                Some(Target::net(id, None)),
                e.to_string(),
            );
        }
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
                "電源でないネットに rail / source が書かれています",
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
                    format!("ネット `{net}` がありません"),
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
                format!("線材のカタログ品 `{wire}` がありません"),
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

/// ケースの参照と、すべての出典（`from.source`）の参照。
pub(super) fn check_cases_and_sources(ctx: &mut Ctx) {
    check_cases(ctx);
    check_source_refs(ctx);
}

fn check_cases(ctx: &mut Ctx) {
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
                format!("モード `{mode}` がありません"),
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
                format!("熱の環境条件 `{tc}` が analysis/thermal.toml にありません"),
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

/// 出典を調べる値 1 つ（位置を引くエンティティ・属性・対象・出典）。
type FromRef<'a> = (EntityKey, String, Target, Option<&'a Provenance>);

/// 値の出典（`from.source`）が `sources/` にあるか。
fn check_source_refs(ctx: &mut Ctx) {
    let design = ctx.design;
    let mut refs: Vec<FromRef> = Vec::new();
    for part in design.parts.values() {
        let id = part.id.as_str();
        for (attr, v) in &part.attr {
            let target = |state: Option<&str>| {
                Target::Param(ParamPath::Attr {
                    part: id.to_owned(),
                    attr: attr.to_string(),
                    state: state.map(str::to_owned),
                })
            };
            collect_from(
                &mut refs,
                EntityKey::Part(id.to_owned()),
                attr.as_str(),
                v,
                target,
            );
        }
    }
    for item in design.catalog.values() {
        for (attr, v) in &item.attr {
            let target = |state: Option<&str>| Target::Catalog {
                id: item.id.clone(),
                attr: Some(AttrPath {
                    attr: attr.to_string(),
                    state: state.map(str::to_owned),
                }),
            };
            let entity = EntityKey::Catalog(item.id.clone());
            collect_from(&mut refs, entity, attr.as_str(), v, target);
        }
    }
    for (entity, attr, target, from) in refs {
        if let Some(source) = from.and_then(|f| f.source.as_deref())
            && !design.sources.contains_key(source)
        {
            ctx.report(
                Severity::Error,
                "ref.source",
                &entity,
                &["attr", &attr],
                Some(target),
                format!("出典 `{source}` が sources/ にありません"),
            );
        }
    }
}

/// 値（と状態ごとのセル）の出典を集める。`target` は状態（セルなら Some）→ 対象。
fn collect_from<'a>(
    out: &mut Vec<FromRef<'a>>,
    entity: EntityKey,
    attr: &str,
    value: &'a crate::value::Authored<Value>,
    target: impl Fn(Option<&str>) -> Target,
) {
    out.push((
        entity.clone(),
        attr.to_string(),
        target(None),
        value.from.as_ref(),
    ));
    if let Value::ByState(cells) = &value.value {
        for (state, cell) in cells {
            let t = target(Some(state.as_str()));
            out.push((entity.clone(), attr.to_string(), t, cell.from.as_ref()));
        }
    }
}

/// 図の配置のノードが部品・グループ・ネットを指すか（見た目だけなので warning）。
pub(super) fn check_views(ctx: &mut Ctx) {
    let design = ctx.design;
    for view in design.views.values() {
        let entity = EntityKey::View(view.id.clone());
        for id in view.nodes.keys() {
            if !design.node_exists(id) && !design.nets.contains_key(id.as_str()) {
                ctx.warning(
                    "view.unknown_node",
                    &entity,
                    &["nodes", id.as_str()],
                    Some(Target::view(view.id.as_str(), Some(id.as_str()))),
                    format!(
                        "図 `{}` のノード `{id}` は部品・グループ・ネットのどれでもありません",
                        view.id
                    ),
                );
            }
        }
    }
}
