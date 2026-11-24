//! 打ち上げ条件が選ぶ品（`conditions/launch.toml` の `launcher`・`dispenser`）の検査。
//! 参照先が定義されているか・種別が合うか（`ref.launcher`・`ref.dispenser`・`launch.kind`）と、打ち上げ機・ディスペンサーの種別を
//! 部品に使っていないか（`part.launch_kind`。これらは衛星の部品ではなく条件）。

use super::Ctx;
use super::settings::scope;
use crate::design::{EntityKey, LaunchInterface};
use crate::path::SettingPath;
use crate::target::Target;

/// 選んだ品と、部品の種別。
pub(super) fn check_launch(ctx: &mut Ctx) {
    check_selection(ctx);
    check_parts(ctx);
}

fn check_selection(ctx: &mut Ctx) {
    let design = ctx.design;
    for which in LaunchInterface::NEAREST_FIRST {
        let Some(reference) = design.launch_selection(which) else {
            continue;
        };
        let key = which.as_str();
        let mut s = scope(ctx, SettingPath::conditions(&["launch"]));
        let Some(item) = design.catalog.get(reference) else {
            s.error(
                &[key],
                &format!("ref.{key}"),
                format!(
                    "Catalog item `{reference}` ({key} of conditions/launch.toml) is not defined"
                ),
            );
            continue;
        };
        if !design.is_launch_kind_of(item.kind.as_str(), which) {
            s.error(
                &[key],
                "launch.kind",
                format!(
                    "Catalog item `{reference}` has kind `{}`, but the {key} must be an item of kind `{}`",
                    item.kind,
                    which.kind_id()
                ),
            );
        }
    }
}

fn check_parts(ctx: &mut Ctx) {
    let design = ctx.design;
    for part in design.parts.values() {
        let Some(kind) = design.part_kind_id(part) else {
            continue;
        };
        if design.is_launch_kind(kind) {
            ctx.error(
                "part.launch_kind",
                &EntityKey::Part(part.id.to_string()),
                &["kind"],
                Some(Target::Part {
                    id: part.id.to_string(),
                }),
                format!(
                    "Kind `{kind}` is a launch vehicle or dispenser, which is a launch condition and not a part. Select it with conditions.launch.launcher or conditions.launch.dispenser"
                ),
            );
        }
    }
}
