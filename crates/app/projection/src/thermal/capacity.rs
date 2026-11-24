//! 熱容量が無い節点の警告（`thermal.zero_capacity`）。質量が 0（か員数 0）の部品は、既定の 1 部品 1 節点では熱容量 0 の節点になり、
//! 熱の解析は入力の誤りとして断る。解析を回さなくても `fune check` で分かるように、同じ節点の組み立て（[`super::network::nodes`]）と
//! 同じ判定（`ThermalNode::lacks_heat_capacity`。解析の入力検査もこれを使う）で、設計に熱の設定があるときだけ調べる。

use super::network::nodes;
use super::{thermal, thermal_nodes};
use crate::context::Context;
use crate::missing::MissingInputs;
use fune_model::design::EntityKey;
use fune_model::resolve::Resolved;
use fune_model::{Design, Diagnostic, Target};

/// 熱容量が無い節点の警告（熱の設定が無ければ空）。節点ごとに 1 件。
/// 質量が無い部品は解析では不足入力（`<部品>.mass`）になり、節点の熱容量にも数えない（その節点は熱容量が無い）。
pub fn check(design: &Design, resolved: &Resolved) -> Vec<Diagnostic> {
    let Some(settings) = design.analysis.thermal.as_ref() else {
        return Vec::new();
    };
    let Ok(ctx) = Context::new(design, resolved, None) else {
        return Vec::new();
    };
    let map = thermal_nodes(&ctx, settings);
    // 不足入力は解析が数える。ここでは節点の熱容量だけを見る（比熱が無い部品の節点は、原因が質量ではなく比熱なので飛ばす）
    let mut missing = MissingInputs::new();
    let found = nodes(&ctx, &map, "thermal.steady", &mut missing);
    let no_specific_heat: Vec<String> = missing
        .into_vec()
        .into_iter()
        .filter(|m| m.path.ends_with(".specific_heat"))
        .filter_map(|m| m.part)
        .collect();
    found
        .into_iter()
        .filter(|n| n.lacks_heat_capacity())
        .filter(|n| {
            map.iter()
                .find(|(id, _)| *id == n.id)
                .is_some_and(|(_, def)| def.parts.iter().all(|p| !no_specific_heat.contains(p)))
        })
        .filter_map(|n| diagnostic(design, &map, &n.id))
        .collect()
}

fn diagnostic(
    design: &Design,
    map: &fune_model::design::ThermalNodes,
    node: &str,
) -> Option<Diagnostic> {
    let (_, def) = map.iter().find(|(id, _)| *id == node)?;
    if def.lumped {
        let parts = def.parts.join("`, `");
        let message = format!(
            "Thermal node `{node}` (parts `{parts}`) has no heat capacity: mass × count × specific heat is zero or missing for every part, \
             and the thermal analyses refuse such a node. Give one of the parts a mass, or list a part that has one in `analysis.thermal.nodes.{node}`"
        );
        return Some(
            Diagnostic::warning("thermal.zero_capacity", message)
                .with_target(thermal(&["nodes", node]))
                .at(design
                    .source_map
                    .locate(&EntityKey::Setting(thermal(&[])), &["nodes", node])),
        );
    }
    let message = format!(
        "Part `{node}` has zero or missing mass, so its own thermal node has no heat capacity (mass × count × specific heat) and the thermal analyses refuse it. \
         Give the part a mass, or lump it into the node of the part it is mounted on in `analysis.thermal.nodes` \
         (for example `fune set analysis.thermal.nodes.<NODE> \"<PART>, {node}\"`)"
    );
    Some(
        Diagnostic::warning("thermal.zero_capacity", message)
            .with_target(Target::attr(node, "mass"))
            .at(design
                .source_map
                .locate(&EntityKey::Part(node.to_owned()), &["attr", "mass"])),
    )
}
