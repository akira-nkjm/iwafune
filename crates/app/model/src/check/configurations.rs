//! 形態（`configurations/<ID>.toml`）の検査。
//!
//! `[pose.<部品>]` のキーは部品であること（`ref.part`）、姿勢は部品の姿勢と同じ検査（`parts::check_pose`。対象は
//! `configurations.<ID>.pose.<部品>`）。取付の無い部品（取付の根）への上書きは解決が使わないので警告する。
//! ID の規則と重複は `ids`。

use super::Ctx;
use super::parts::check_pose;
use crate::Id;
use crate::design::{Configuration, EntityKey};
use crate::path::SettingPath;
use crate::target::Target;

pub(super) fn check_configurations(ctx: &mut Ctx) {
    let design = ctx.design;
    for configuration in design.configurations.values() {
        check_pose_rows(ctx, configuration);
    }
    check_selections(ctx);
}

/// 形態を選ぶ欄の参照と、打ち上げ形態の省略。
fn check_selections(ctx: &mut Ctx) {
    let launch = SettingPath::conditions(&["launch"]);
    let entity = EntityKey::Setting(launch.clone());
    let target = Target::from(launch.child(&["configuration"]));
    if let Some(id) = ctx
        .design
        .conditions
        .launch
        .as_ref()
        .and_then(|s| s.configuration.as_ref())
    {
        check_selection(ctx, &entity, target, id);
    } else if !ctx.design.configurations.is_empty() {
        ctx.info(
            "launch.configuration_unset",
            &entity,
            &["configuration"],
            Some(target),
            "No launch configuration is selected. Launch analyses use the base placement",
        );
    }
    for phase in ctx.design.phases.values() {
        if let Some(id) = &phase.configuration {
            check_selection(
                ctx,
                &EntityKey::Phase(phase.id.to_string()),
                Target::phase(phase.id.as_str(), Some("configuration")),
                id,
            );
        }
    }
}

/// 参照先の実在を検査する。参照された形態の削除も適用後の同じ検査で拒まれる。
fn check_selection(ctx: &mut Ctx, entity: &EntityKey, target: Target, id: &Id<Configuration>) {
    if !ctx.design.configurations.contains_key(id) {
        ctx.error(
            "ref.configuration",
            entity,
            &["configuration"],
            Some(target),
            format!("Configuration `{id}` is not defined"),
        );
    }
}

/// `[pose.<部品>]` の各行。
fn check_pose_rows(ctx: &mut Ctx, configuration: &Configuration) {
    let design = ctx.design;
    let entity = EntityKey::Configuration(configuration.id.to_string());
    for (part_id, pose) in &configuration.pose {
        let part = part_id.as_str();
        let field = format!("pose.{part}");
        let target = Some(Target::configuration(
            configuration.id.as_str(),
            Some(&field),
        ));
        let Some(found) = design.parts.get(part) else {
            ctx.error(
                "ref.part",
                &entity,
                &["pose", part],
                target,
                format!("Part `{part}` is not defined"),
            );
            continue;
        };
        check_pose(ctx, &entity, &["pose", part], target.clone(), pose);
        if found.mount.is_none() {
            ctx.warning(
                "configuration.pose_without_mount",
                &entity,
                &["pose", part],
                target,
                format!(
                    "Part `{part}` has no mount, so its pose is not used. Override the pose of a mounted part"
                ),
            );
        }
    }
}
