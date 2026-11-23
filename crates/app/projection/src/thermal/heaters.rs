//! サーモスタット付きのヒータの射影: `setpoint`（`{ min, max }` の範囲）を持つ部品は、平均電力の発熱（`loads`）ではなく
//! `ThermalHeater` として解析に渡す（二重に数えない）。オンの電力は `power` の `on` の状態の値、熱を入れる節点は発熱と同じ規則
//! （`[heat_node]` に書いた節点、無ければ部品の属する節点）。制御則・モードでの有効/無効・判定は sat-analysis が行う（既定値も解析）。

use super::node_of;
use crate::context::Context;
use crate::missing::MissingInputs;
use crate::parts::state_values;
use fune_model::Target;
use fune_model::design::ThermalSettings;
use fune_model::resolve::{ResolvedPart, SiValue};
use fune_model::role::STATE_MAIN;
use sat_analysis::thermal::ThermalHeater;
use sat_units::typed::{Power, Qty};

/// 部品がサーモスタット付きのヒータか（`setpoint` が範囲で書かれている）。
pub(super) fn is_thermostat(ctx: &Context<'_>, p: &ResolvedPart) -> bool {
    matches!(ctx.attr(p, "setpoint"), Some(SiValue::Range { .. }))
}

/// 熱を入れる節点の対応（部品 → 節点。`[heat_node]` が節点への所属より優先）。
pub(super) fn heat_nodes(s: &ThermalSettings) -> std::collections::BTreeMap<&str, &str> {
    let mut nodes = node_of(s);
    for (pid, node) in &s.heat_node {
        nodes.insert(pid.as_str(), node.as_str());
    }
    nodes
}

/// サーモスタット付きのヒータ（節点が決まる部品だけ）。オンの電力が無ければ不足入力。
pub fn heaters(
    ctx: &Context<'_>,
    s: &ThermalSettings,
    m: &mut MissingInputs,
) -> Vec<ThermalHeater> {
    let nodes = heat_nodes(s);
    let mut out = Vec::new();
    for (pid, node) in nodes {
        let Some(p) = ctx.resolved.parts.get(pid) else {
            continue;
        };
        let Some(SiValue::Range { min, max }) = ctx.attr(p, "setpoint") else {
            continue;
        };
        let power = state_values::<Power>(ctx, p, "power").and_then(|v| v.get(STATE_MAIN));
        let Some(power) = m.need(
            power,
            Target::attr(pid, "power"),
            "Power of the heater in the on state (the thermostat heat)",
        ) else {
            continue;
        };
        out.push(ThermalHeater {
            id: p.id.clone(),
            node: node.to_owned(),
            on_below: Qty::from_raw(*min),
            off_above: Qty::from_raw(*max),
            power,
            count: p.count_or_one(),
            redundancy: crate::parts::redundancy(p),
            states: p.states.clone(),
        });
    }
    out
}
