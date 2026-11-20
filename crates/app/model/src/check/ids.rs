//! ID の規則と一意性の検査。一意性は大文字小文字を無視して比べる（規則は `id::same_id`・`id::duplicates`）。
//!
//! | 名前空間 | ID の規則 |
//! |---|---|
//! | 部品とグループ（同じ名前空間） | `validate_node_id`（パスの予約語も禁止） |
//! | モード・ネット・ハーネス・解析ケース・出典 | `validate_entity_id` |
//! | 共有カタログの出典（カタログごと。`<カタログ名>/<ID>` の ID の部分） | `validate_entity_id` |
//! | カタログ品（出所ごと。`<出所>/<ID>` の ID の部分） | `validate_entity_id` |

use super::Ctx;
use crate::design::{EntityKey, split_catalog_ref};
use crate::id::{IdError, duplicates, validate_entity_id, validate_node_id};
use crate::target::Target;

/// 1 つの名前空間の ID（検査する ID, 位置を引くエンティティ, 対象）。
type Entry = (String, EntityKey, Target);

pub(super) fn check_ids(ctx: &mut Ctx) {
    let design = ctx.design;
    let parts = design.parts.keys().map(|id| {
        let id = id.to_string();
        (id.clone(), EntityKey::Part(id.clone()), Target::Part { id })
    });
    let groups = design.groups.keys().map(|id| {
        let id = id.to_string();
        (
            id.clone(),
            EntityKey::Part(id.clone()),
            Target::Group { id },
        )
    });
    namespace(ctx, parts.chain(groups).collect(), validate_node_id);
    let modes = design.modes.keys().map(|id| {
        let id = id.to_string();
        let target = Target::mode(&id, None);
        (id.clone(), EntityKey::Mode(id), target)
    });
    namespace(ctx, modes.collect(), validate_entity_id);
    let nets = design.nets.keys().map(|id| {
        let id = id.to_string();
        let target = Target::net(&id, None);
        (id.clone(), EntityKey::Net(id), target)
    });
    namespace(ctx, nets.collect(), validate_entity_id);
    let harnesses = design.harnesses.keys().map(|id| {
        let id = id.to_string();
        let target = Target::harness(&id, None);
        (id.clone(), EntityKey::Harness(id), target)
    });
    namespace(ctx, harnesses.collect(), validate_entity_id);
    let cases = design.cases.keys().map(|id| {
        let id = id.to_string();
        let target = Target::case(&id, None);
        (id.clone(), EntityKey::Case(id), target)
    });
    namespace(ctx, cases.collect(), validate_entity_id);
    check_source_ids(ctx);
    let maneuvers = design.maneuvers.keys().map(|id| {
        let id = id.to_string();
        let target = Target::maneuver(&id, None);
        (id.clone(), EntityKey::Maneuver(id), target)
    });
    namespace(ctx, maneuvers.collect(), validate_entity_id);
    let requirements = design.requirements.keys().map(|id| {
        let id = id.to_string();
        let target = Target::requirement(&id, None);
        (id.clone(), EntityKey::Requirement(id), target)
    });
    namespace(ctx, requirements.collect(), validate_entity_id);
    check_catalog_ids(ctx);
}

/// 出典の ID。プロジェクトの出典と、共有カタログの出典（`<カタログ名>/<ID>`。カタログごとの名前空間で、ID の部分を検査する）。
fn check_source_ids(ctx: &mut Ctx) {
    let design = ctx.design;
    let mut spaces: std::collections::BTreeMap<&str, Vec<Entry>> = Default::default();
    for id in design.sources.keys() {
        let full = id.to_string();
        let (space, local) = match design.shared_catalog_of(&full) {
            Some(c) => (
                c.name.as_str(),
                split_catalog_ref(&full).map_or(full.clone(), |(_, i)| i.to_owned()),
            ),
            None => ("", full.clone()),
        };
        spaces.entry(space).or_default().push((
            local,
            EntityKey::Source(full.clone()),
            Target::Source { id: full },
        ));
    }
    for entries in spaces.into_values() {
        namespace(ctx, entries, validate_entity_id);
    }
}

/// カタログ品の ID（出所ごとの名前空間）。
fn check_catalog_ids(ctx: &mut Ctx) {
    let design = ctx.design;
    for source in &design.project.catalogs {
        let items: Vec<Entry> = design
            .catalog
            .keys()
            .filter_map(|reference| {
                let (s, id) = split_catalog_ref(reference)?;
                (s == source.name).then(|| {
                    let target = Target::Catalog {
                        id: reference.clone(),
                        attr: None,
                    };
                    (id.to_owned(), EntityKey::Catalog(reference.clone()), target)
                })
            })
            .collect();
        namespace(ctx, items, validate_entity_id);
    }
}

/// 1 つの名前空間の ID の規則と一意性。
fn namespace(ctx: &mut Ctx, entries: Vec<Entry>, rule: fn(&str) -> Result<(), IdError>) {
    for (id, entity, target) in &entries {
        if let Err(e) = rule(id) {
            ctx.error(
                "id.invalid",
                entity,
                &[],
                Some(target.clone()),
                e.to_string(),
            );
        }
    }
    for (later, first) in duplicates(entries.iter().map(|(id, _, _)| id.as_str())) {
        let (id, entity, target) = &entries[later];
        let other = &entries[first].0;
        ctx.error(
            "id.duplicate",
            entity,
            &[],
            Some(target.clone()),
            format!("ID `{id}` duplicates `{other}` (compared ignoring case)"),
        );
    }
}
