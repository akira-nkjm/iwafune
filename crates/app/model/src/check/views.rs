//! 図の配置（`view/<名前>.toml`）の検査。ノードの参照は見た目だけなので warning、手で引いた線の経路の形
//! （直交・端子の位置。規則は `ViewEdge::is_orthogonal`・`offsets_are_valid`）は読めない図になるので error
//! （GUI・CLI の編集も、この検査で新しい error が出れば計画が拒まれる）。

use super::Ctx;
use crate::design::EntityKey;
use crate::target::Target;

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
                        "Node `{id}` of diagram `{}` is not a part, group, or net",
                        view.id
                    ),
                );
            }
        }
        for (key, edge) in &view.edge {
            let keys = ["edge", key.as_str()];
            let target = || Some(Target::view(view.id.as_str(), None));
            if !edge.is_orthogonal() {
                ctx.error(
                    "view.route",
                    &entity,
                    &keys,
                    target(),
                    format!("The route of line `{key}` must be an orthogonal line with at least 2 points"),
                );
            }
            if !edge.offsets_are_valid() {
                ctx.error(
                    "view.port_offset",
                    &entity,
                    &keys,
                    target(),
                    format!(
                        "The terminal position of line `{key}` must be at least 0 (distance from the left or top corner of the side)"
                    ),
                );
            }
        }
    }
}
