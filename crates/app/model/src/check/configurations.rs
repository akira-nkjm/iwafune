//! 形態（`configurations/<ID>.toml`）の検査。
//!
//! `[pose.<部品>]` のキーは部品であること（`ref.part`）、姿勢は部品の姿勢と同じ検査（`parts::check_pose`。対象は
//! `configurations.<ID>.pose.<部品>`）。取付の無い部品（取付の根）への上書きは解決が使わないので警告する。
//! ID の規則と重複は `ids`。

use super::Ctx;
use super::parts::check_pose;
use crate::design::{Configuration, EntityKey};
use crate::target::Target;

pub(super) fn check_configurations(ctx: &mut Ctx) {
    let design = ctx.design;
    for configuration in design.configurations.values() {
        check_pose_rows(ctx, configuration);
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
