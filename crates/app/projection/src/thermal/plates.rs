//! 升に分ける板の射影（`[subdivide.<部品>]`）。升の数・箱・材料の熱伝導率・取付で結ばれた相手の箱を集めて解析
//! （`sat_analysis::thermal::network::subdivide`）へ渡す。板かどうかの判断と升への配り方は解析が行う。

use super::thermal;
use crate::context::Context;
use crate::missing::MissingInputs;
use crate::quantity::setting;
use fune_model::Target;
use fune_model::design::setting_dims as dims;
use fune_model::design::{ThermalNodes, ThermalSettings};
use sat_analysis::structure::interference::InterferenceBox;
use sat_analysis::thermal::network::{ThermalPlate, ThermalPlateAttachment};

/// 升に分ける板（`[subdivide.<部品>]`）。升の数・箱（機体座標）・材料の熱伝導率・取付で結ばれた相手の箱を集めるだけで、
/// 板かどうかの判断・升への配り方は解析（`thermal::network::subdivide`）が行う。部品が無い・節点にまとめられた部品は
/// ここでは飛ばし（`check` の誤り、または解析が「その名前の節点が無い」入力の誤りにする）、`u`・`v` や箱や材料が無ければ不足入力。
pub(super) fn plates(
    ctx: &Context<'_>,
    s: &ThermalSettings,
    map: &ThermalNodes,
    m: &mut MissingInputs,
) -> Vec<ThermalPlate> {
    let all = crate::structure::boxes(ctx).boxes;
    let find = |id: &str| -> Option<InterferenceBox> {
        all.iter().find(|b| b.id == id).cloned().map(Into::into)
    };
    let mut out = Vec::new();
    for (part, cells) in &s.subdivide {
        let Some(p) = ctx.resolved.parts.get(part.as_str()) else {
            continue;
        };
        let at = |axis: &str| thermal(&["subdivide", part, axis]);
        let count = |n: Option<u32>, axis: &str, m: &mut MissingInputs| {
            m.need(n, at(axis), "Number of plate cells along the axis")
        };
        let (u, v) = (count(cells.u, "u", m), count(cells.v, "v", m));
        let frame = m.need(
            find(part),
            Target::attr(part, "size"),
            "Part outline (the plate box)",
        );
        let k = crate::structure::material(ctx, p, m).and_then(|(name, mat)| {
            setting(
                &dims::MATERIAL_THERMAL_CONDUCTIVITY,
                &[&name],
                mat.thermal_conductivity.as_ref(),
                "Thermal conductivity of the plate material",
                m,
            )
        });
        let (Some(u), Some(v), Some(frame), Some(conductivity)) = (u, v, frame, k) else {
            continue;
        };
        if map.node_of(part) != Some(part.as_str()) {
            continue;
        }
        out.push(ThermalPlate {
            part: part.clone(),
            cells: [u, v],
            conductivity,
            attachments: attachments(ctx, map, part, &find),
            frame,
        });
    }
    out
}

/// 板と取付で結ばれた相手（板に取り付く部品と、板の取付先）の節点と箱。
fn attachments(
    ctx: &Context<'_>,
    map: &ThermalNodes,
    part: &str,
    find: &dyn Fn(&str) -> Option<InterferenceBox>,
) -> Vec<ThermalPlateAttachment> {
    let own_parent = ctx
        .resolved
        .parts
        .get(part)
        .and_then(|p| p.mount.as_ref())
        .map(|mount| mount.on.as_str());
    let children = ctx
        .resolved
        .parts
        .values()
        .filter(|q| q.mount.as_ref().is_some_and(|mount| mount.on == part))
        .map(|q| q.id.as_str());
    own_parent
        .into_iter()
        .chain(children)
        .filter_map(|other| {
            Some(ThermalPlateAttachment {
                node: map.node_of(other)?.to_owned(),
                footprint: find(other)?,
            })
        })
        .collect()
}
