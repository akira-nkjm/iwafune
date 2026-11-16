//! 部品・グループ・カタログ品の検査（ID・定義の参照・属性・取付・ポート）。

use super::Ctx;
use super::values::check_value_against;
use crate::chain::chain;
use crate::design::{EntityKey, Part, Pose, position_unit};
use crate::path::{AttrPath, PortRef};
use crate::target::Target;
use crate::value::list_from_text;
use crate::value::units::expect_dimension;

/// 部品とグループ。
pub(super) fn check_parts(ctx: &mut Ctx) {
    check_system_dirs(ctx);
    let design = ctx.design;
    for part in design.parts.values() {
        let entity = EntityKey::Part(part.id.to_string());
        check_definition(ctx, part, &entity);
        check_part_attrs(ctx, part, &entity);
        check_mount(ctx, part, &entity);
        check_ports(ctx, part, &entity);
    }
}

/// 最上位ディレクトリは系の ID（parts/ 直下の部品は許す）。
fn check_system_dirs(ctx: &mut Ctx) {
    use crate::design::{AnalysisSystem, NodeRef};
    let design = ctx.design;
    let roots: std::collections::BTreeSet<NodeRef> = design
        .parts
        .values()
        .filter_map(|p| {
            design
                .ancestors(&NodeRef::Part(p.id.clone()))
                .last()
                .cloned()
        })
        .collect();
    let allowed = AnalysisSystem::ALL
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    for root in roots {
        let id = root.id();
        if id.parse::<AnalysisSystem>().is_err() {
            ctx.error(
                "part.system_dir",
                &EntityKey::Part(id.to_owned()),
                &[],
                Some(Target::Group { id: id.to_owned() }),
                format!("Top-level directory `{id}` of parts/ must be a subsystem ID ({allowed})"),
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
            format!("Catalog item `{reference}` is not defined"),
        );
    }
    if let Some(kind) = &part.kind {
        if design.kind(kind.as_str()).is_none() {
            ctx.error(
                "ref.kind",
                entity,
                &["kind"],
                Some(Target::field(&id, "kind")),
                format!("Kind `{kind}` is not defined"),
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
                    "Kind `{kind}` differs from the catalog item's kind `{}`",
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
            "The part has neither `catalog` nor `kind`",
        );
    }
    for candidate in &part.candidates {
        if !design.catalog.contains_key(candidate) {
            ctx.error(
                "ref.catalog",
                entity,
                &["candidates"],
                Some(Target::field(&id, "candidates")),
                format!("Candidate catalog item `{candidate}` is not defined"),
            );
        }
    }
    if part.count == 0 {
        ctx.error(
            "part.count",
            entity,
            &["count"],
            Some(Target::field(&id, "count")),
            "The count is 0",
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
                format!("Attribute `{attr}` has no definition (add one in defs/attribute/)"),
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
                    format!("Required attribute `{attr}` is not set (kind `{}`)", k.id),
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
                format!("Mount target part `{}` is not defined", mount.on),
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
                        format!("Part `{}` has no mount point `{point}`", mount.on),
                    );
                }
                if mount_cycle(design, part) {
                    ctx.error(
                        "mount.cycle",
                        entity,
                        &["mount"],
                        target(),
                        "The mount chain is circular",
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
    let mut parts = chain(
        Some(part),
        |p: &&Part| p.id.as_str(),
        |p| {
            p.mount
                .as_ref()
                .and_then(|m| design.parts.get(m.on.as_str()))
        },
    );
    parts.by_ref().for_each(drop);
    parts.cycled()
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
        && let Err(e) = expect_dimension(position_unit(p), "Length")
    {
        let code = match e {
            crate::value::ValueError::Dimension { .. } => "unit.dimension",
            _ => "unit.parse",
        };
        ctx.error(code, entity, keys, target(), e.to_string());
    }
    if let Some(q) = &pose.orientation {
        let norm = (q.w * q.w + q.x * q.x + q.y * q.y + q.z * q.z).sqrt();
        if (norm - 1.0).abs() > super::UNIT_NORM_TOLERANCE {
            ctx.error(
                "pose.quaternion_norm",
                entity,
                keys,
                target(),
                format!("The attitude quaternion does not have norm 1 ({norm:.4})"),
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
                format!("Net `{net}` is not defined"),
            );
        }
    }
}

/// カタログ品のタグの検査。タグは `fune catalog set <品> tags <値>` の並びの書式でそのまま読み直せる文字列
/// （空でない・前後に空白が無い・区切りの `,` を含まない）で、同じ品の中で重複しない。
fn check_tags(ctx: &mut Ctx, entity: &EntityKey, tags: &[String]) {
    for (i, tag) in tags.iter().enumerate() {
        if list_from_text(tag).as_slice() != std::slice::from_ref(tag) {
            ctx.error(
                "catalog.tag_text",
                entity,
                &["tags"],
                None,
                format!(
                    "Tag `{tag}` cannot be used. It must be non-empty, have no leading or trailing whitespace, and not contain `,`"
                ),
            );
        } else if tags[..i].contains(tag) {
            ctx.error(
                "catalog.tag_duplicate",
                entity,
                &["tags"],
                None,
                format!("Tag `{tag}` is duplicated. Keep only one"),
            );
        }
    }
}

/// カタログ品（種別の参照と値）。属性 ID が部品のフィールド名と重なっていないかも見る。
pub(super) fn check_catalog(ctx: &mut Ctx) {
    let design = ctx.design;
    for item in design.catalog.values() {
        let entity = EntityKey::Catalog(item.id.clone());
        check_tags(ctx, &entity, &item.tags);
        if design.kind(item.kind.as_str()).is_none() {
            ctx.error(
                "ref.kind",
                &entity,
                &["kind"],
                Some(Target::Catalog {
                    id: item.id.clone(),
                    attr: None,
                }),
                format!("Kind `{}` is not defined", item.kind),
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
                    format!("Attribute `{attr}` has no definition"),
                ),
            }
        }
    }
}
