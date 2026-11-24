//! 形態（`configurations/<ID>.toml`）の検査。
//!
//! `[pose.<部品>]` のキーは部品であること（`ref.part`）、姿勢は部品の姿勢と同じ検査（`parts::check_pose`。対象は
//! `configurations.<ID>.pose.<部品>`）。取付の無い部品（根）の上書きは機体座標での姿勢で、取付のある部品と同じ検査。
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
        if !design.parts.contains_key(part) {
            ctx.error(
                "ref.part",
                &entity,
                &["pose", part],
                target,
                format!("Part `{part}` is not defined"),
            );
            continue;
        }
        check_pose(ctx, &entity, &["pose", part], target, pose);
    }
}
