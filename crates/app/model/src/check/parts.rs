//! 部品・グループ・カタログ品の検査（ID・定義の参照・属性・取付・ポート）。

use super::Ctx;
use super::values::check_value_against;
use crate::design::{EntityKey, Part, Pose};
use crate::diagnostic::Severity;
use crate::id::{fold, validate_node_id};
use crate::path::{AttrPath, CATALOG_FIELDS, PART_FIELDS, PortRef};
use crate::target::Target;
use crate::value::units::expect_dimension;
use std::collections::BTreeMap;

/// 部品とグループ。
pub(super) fn check_parts(ctx: &mut Ctx) {
    check_node_ids(ctx);
    let design = ctx.design;
    for part in design.parts.values() {
        let entity = EntityKey::Part(part.id.to_string());
        check_definition(ctx, part, &entity);
        check_part_attrs(ctx, part, &entity);
        check_mount(ctx, part, &entity);
        check_ports(ctx, part, &entity);
    }
}

/// ID の規則と、Group と Part を合わせた一意性（大文字小文字を無視）。
fn check_node_ids(ctx: &mut Ctx) {
    let design = ctx.design;
    for id in design.project.groups.keys() {
        if !design.groups.contains_key(id.as_str()) {
            ctx.warning(
                "group.unknown_label",
                &EntityKey::Project,
                &["groups", id.as_str()],
                Some(Target::Group { id: id.clone() }),
                format!(
                    "fune.toml の [groups] の `{id}` に当たるディレクトリが parts/ にありません"
                ),
            );
        }
    }
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    let parts = design.parts.keys().map(|id| {
        let id = id.to_string();
        (id.clone(), Target::Part { id })
    });
    let groups = design.groups.keys().map(|id| {
        let id = id.to_string();
        (id.clone(), Target::Group { id })
    });
    for (id, target) in parts.chain(groups) {
        let entity = EntityKey::Part(id.clone());
        if let Err(e) = validate_node_id(&id) {
            ctx.error(
                "id.invalid",
                &entity,
                &[],
                Some(target.clone()),
                e.to_string(),
            );
        }
        if let Some(other) = seen.insert(fold(&id), id.clone()) {
            ctx.error(
                "id.duplicate",
                &entity,
                &[],
                Some(target),
                format!("ID `{id}` が `{other}` と重複しています（大文字小文字を無視して比較）"),
            );
        }
    }
}

/// カタログ・種別の参照。
fn check_definition(ctx: &mut Ctx, part: &Part, entity: &EntityKey) {
    let design = ctx.design;
    let id = part.id.to_string();
    let item = part.catalog.as_ref().map(|c| (c, design.catalog.get(c)));
    if let Some((reference, None)) = item {
        ctx.error(
            "ref.catalog",
            entity,
            &["catalog"],
            Some(Target::field(&id, "catalog")),
            format!("カタログ品 `{reference}` がありません"),
        );
    }
    if let Some(kind) = &part.kind {
        if design.kind(kind.as_str()).is_none() {
            ctx.error(
                "ref.kind",
                entity,
                &["kind"],
                Some(Target::field(&id, "kind")),
                format!("種別 `{kind}` がありません"),
            );
        } else if let Some((_, Some(item))) = item
            && item.kind != *kind
        {
            ctx.error(
                "kind.catalog_mismatch",
                entity,
                &["kind"],
                Some(Target::field(&id, "kind")),
                format!(
                    "種別 `{kind}` がカタログ品の種別 `{}` と違います",
                    item.kind
                ),
            );
        }
    }
    check_count_and_candidates(ctx, part, entity);
}

/// 定義の有無・トレード候補・員数。
fn check_count_and_candidates(ctx: &mut Ctx, part: &Part, entity: &EntityKey) {
    let design = ctx.design;
    let id = part.id.to_string();
    if part.catalog.is_none() && part.kind.is_none() {
        ctx.error(
            "part.no_definition",
            entity,
            &[],
            Some(Target::Part { id: id.clone() }),
            "部品に `catalog` も `kind` もありません",
        );
    }
    for candidate in &part.candidates {
        if !design.catalog.contains_key(candidate) {
            ctx.error(
                "ref.catalog",
                entity,
                &["candidates"],
                Some(Target::field(&id, "candidates")),
                format!("候補のカタログ品 `{candidate}` がありません"),
            );
        }
    }
    if part.count == 0 {
        ctx.error(
            "part.count",
            entity,
            &["count"],
            Some(Target::field(&id, "count")),
            "員数が 0 です",
        );
    }
}

/// 上書きの値と、必須の属性の欠け。
fn check_part_attrs(ctx: &mut Ctx, part: &Part, entity: &EntityKey) {
    let design = ctx.design;
    let id = part.id.to_string();
    let states = design.part_states(part).to_vec();
    for (attr, authored) in &part.attr {
        let path = Some(Target::attr(&id, attr.as_str()));
        let Some(def) = design.attribute(attr.as_str()) else {
            ctx.error(
                "ref.attribute",
                entity,
                &["attr", attr.as_str()],
                path,
                format!("属性 `{attr}` の定義がありません（defs/attribute/ に足せます）"),
            );
            continue;
        };
        for (severity, code, message) in check_value_against(def, &authored.value, &states) {
            ctx.report(
                severity,
                code,
                entity,
                &["attr", attr.as_str()],
                path.clone(),
                message,
            );
        }
    }
    let Some(kind) = design.part_kind_id(part) else {
        return;
    };
    let catalog = part.catalog.as_ref().and_then(|c| design.catalog.get(c));
    for k in design.kind_chain(kind) {
        for attr in &k.required {
            let present = part.attr.contains_key(attr.as_str())
                || catalog.is_some_and(|c| c.attr.contains_key(attr.as_str()))
                || design
                    .attribute(attr.as_str())
                    .is_some_and(|d| d.default.is_some());
            if !present {
                ctx.warning(
                    "attr.missing",
                    entity,
                    &["attr"],
                    Some(Target::attr(&id, attr.as_str())),
                    format!("必須の属性 `{attr}` が未設定です（種別 `{}`）", k.id),
                );
            }
        }
    }
}

/// 取付先・取付点・循環・姿勢。
fn check_mount(ctx: &mut Ctx, part: &Part, entity: &EntityKey) {
    let design = ctx.design;
    let id = part.id.to_string();
    let target = || Some(Target::Mount { part: id.clone() });
    if let Some(mount) = &part.mount {
        match design.parts.get(mount.on.as_str()) {
            None => ctx.error(
                "ref.mount",
                entity,
                &["mount", "on"],
                target(),
                format!("取付先の部品 `{}` がありません", mount.on),
            ),
            Some(parent) => {
                if let Some(point) = &mount.point
                    && !parent.mount_points.contains_key(point)
                {
                    ctx.error(
                        "ref.mount_point",
                        entity,
                        &["mount", "point"],
                        target(),
                        format!("部品 `{}` に取付点 `{point}` がありません", mount.on),
                    );
                }
                if mount_cycle(design, part) {
                    ctx.error(
                        "mount.cycle",
                        entity,
                        &["mount"],
                        target(),
                        "取付の連鎖が循環しています",
                    );
                }
            }
        }
    }
    if let Some(pose) = &part.pose {
        check_pose(ctx, entity, &id, None, pose);
    }
    for (name, pose) in &part.mount_points {
        check_pose(ctx, entity, &id, Some(name.as_str()), pose);
    }
}

fn mount_cycle(design: &crate::design::Design, part: &Part) -> bool {
    let mut seen = vec![part.id.as_str()];
    let mut current = part.mount.as_ref().map(|m| m.on.as_str());
    while let Some(on) = current {
        if seen.contains(&on) {
            return true;
        }
        seen.push(on);
        current = design
            .parts
            .get(on)
            .and_then(|p| p.mount.as_ref())
            .map(|m| m.on.as_str());
    }
    false
}

/// 姿勢 1 つ（部品の `pose` か、取付点 `mount_point` の姿勢）。
fn check_pose(ctx: &mut Ctx, entity: &EntityKey, id: &str, mount_point: Option<&str>, pose: &Pose) {
    let keys: &[&str] = match mount_point {
        None => &["pose"],
        Some(point) => &["mount_points", point],
    };
    let target = || {
        Some(Target::Pose {
            part: id.to_owned(),
            mount_point: mount_point.map(str::to_owned),
        })
    };
    if let Some(p) = &pose.position
        && let Err(e) = expect_dimension(p.unit.as_deref().unwrap_or("m"), "Length")
    {
        let code = match e {
            crate::value::ValueError::Dimension { .. } => "unit.dimension",
            _ => "unit.parse",
        };
        ctx.error(code, entity, keys, target(), e.to_string());
    }
    if let Some(q) = &pose.orientation {
        let norm = (q.w * q.w + q.x * q.x + q.y * q.y + q.z * q.z).sqrt();
        if (norm - 1.0).abs() > 1e-3 {
            ctx.error(
                "pose.quaternion_norm",
                entity,
                keys,
                target(),
                format!("姿勢のクォータニオンの大きさが 1 ではありません（{norm:.4}）"),
            );
        }
    }
}

/// ポート → ネットの参照。
fn check_ports(ctx: &mut Ctx, part: &Part, entity: &EntityKey) {
    let design = ctx.design;
    for (port, binding) in &part.ports {
        if let Some(net) = &binding.net
            && !design.nets.contains_key(net.as_str())
        {
            ctx.error(
                "ref.net",
                entity,
                &["ports", port.as_str(), "net"],
                Some(Target::Port(PortRef {
                    part: part.id.to_string(),
                    port: port.to_string(),
                })),
                format!("ネット `{net}` がありません"),
            );
        }
    }
}

/// カタログ品（種別の参照と値）。属性 ID が部品のフィールド名と重なっていないかも見る。
pub(super) fn check_catalog(ctx: &mut Ctx) {
    let design = ctx.design;
    for item in design.catalog.values() {
        let entity = EntityKey::Catalog(item.id.clone());
        if design.kind(item.kind.as_str()).is_none() {
            ctx.error(
                "ref.kind",
                &entity,
                &["kind"],
                Some(Target::Catalog {
                    id: item.id.clone(),
                    attr: None,
                }),
                format!("種別 `{}` がありません", item.kind),
            );
            continue;
        }
        let states = design.kind_states(item.kind.as_str()).to_vec();
        for (attr, authored) in &item.attr {
            let keys = ["attr", attr.as_str()];
            let path = Some(Target::Catalog {
                id: item.id.clone(),
                attr: Some(AttrPath {
                    attr: attr.to_string(),
                    state: None,
                }),
            });
            match design.attribute(attr.as_str()) {
                Some(def) => {
                    for (sev, code, msg) in check_value_against(def, &authored.value, &states) {
                        ctx.report(sev, code, &entity, &keys, path.clone(), msg);
                    }
                }
                None => ctx.error(
                    "ref.attribute",
                    &entity,
                    &keys,
                    path,
                    format!("属性 `{attr}` の定義がありません"),
                ),
            }
        }
    }
    for id in design.attributes.keys() {
        if PART_FIELDS.contains(&id.as_str()) || CATALOG_FIELDS.contains(&id.as_str()) {
            ctx.report(
                Severity::Error,
                "def.reserved",
                &EntityKey::Attribute(id.to_string()),
                &[],
                Some(Target::AttributeDef { id: id.to_string() }),
                format!("属性 ID `{id}` は部品・カタログ品のフィールド名と同じです"),
            );
        }
    }
}
