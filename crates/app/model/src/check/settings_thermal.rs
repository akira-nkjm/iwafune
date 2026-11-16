//! `analysis/thermal.toml` の検査（環境条件のモード・割合の範囲・節点の部品・表面材と面積割合・発熱の場所・温度差の単位）。

use super::Ctx;
use super::settings::{Scope, scope};
use crate::design::{AnalysisSystem, Design, ThermalSettings};
use crate::value::Quantity;

/// 温度差（解析マージン）を書く単位。°C は 273.15 K のずれを足してしまうので使わない。
const TEMPERATURE_DIFFERENCE_UNIT: &str = "K";

pub(super) fn check_thermal(ctx: &mut Ctx) {
    let design = ctx.design;
    let Some(t) = &design.analysis.thermal else {
        return;
    };
    let mut s = scope(ctx, AnalysisSystem::Thermal.setting_path());
    for (name, case) in &t.cases {
        let n = name.as_str();
        if let Some(mode) = &case.mode {
            let exists = design.modes.contains_key(mode.as_str());
            s.reference(
                &["case", n, "mode"],
                exists,
                "ref.mode",
                format!("Mode `{mode}`"),
            );
        }
        s.fraction(&["case", n, "albedo"], case.albedo);
    }
    if let Some(m) = &t.margin {
        for (key, q) in [
            ("uncertainty", &m.uncertainty),
            ("warning_band", &m.warning_band),
        ] {
            check_difference(&mut s, key, q.as_ref());
        }
    }
    check_nodes(&mut s, design, t);
    check_surfaces(&mut s, design, t);
    check_heat_nodes(&mut s, design, t);
}

/// 温度差は K で書き、0 以上にする。
fn check_difference(s: &mut Scope, key: &str, q: Option<&Quantity>) {
    let Some(q) = q else { return };
    if q.unit != TEMPERATURE_DIFFERENCE_UNIT {
        s.error(
            &["margin", key],
            "thermal.difference_unit",
            format!(
                "Temperature differences must be written in {TEMPERATURE_DIFFERENCE_UNIT} (`{}` cannot be used)",
                q.unit
            ),
        );
    } else if q.value.is_nan() || q.value < 0.0 {
        s.error(
            &["margin", key],
            "setting.range",
            "Must be a value of at least 0".into(),
        );
    }
}

/// 節点の部品の参照と、1 つの部品が 2 つの節点に入っていないか。
fn check_nodes(s: &mut Scope, design: &Design, t: &ThermalSettings) {
    let mut owner: std::collections::BTreeMap<&str, &str> = std::collections::BTreeMap::new();
    for (node, parts) in &t.nodes {
        for part in parts {
            let exists = design.parts.contains_key(part.as_str());
            s.reference(
                &["nodes", node],
                exists,
                "ref.part",
                format!("Part `{part}`"),
            );
            if let Some(other) = owner.insert(part.as_str(), node.as_str()) {
                s.error(
                    &["nodes", node],
                    "thermal.node_overlap",
                    format!("Part `{part}` is in both node `{other}` and node `{node}`"),
                );
            }
        }
    }
}

/// 表面材の α/ε の範囲、外面の部品と表面材の参照、面積割合の範囲と合計。
/// α/ε の欠けと割合の合計は 1 つずつ書いていく途中でも起きるので警告にする（解析は入力の誤りで止まる）。
fn check_surfaces(s: &mut Scope, design: &Design, t: &ThermalSettings) {
    for (id, c) in &t.coating {
        s.fraction(&["coating", id, "absorptivity"], c.absorptivity);
        s.fraction(&["coating", id, "emissivity"], c.emissivity);
        if c.absorptivity.is_none() || c.emissivity.is_none() {
            s.warning(
                &["coating", id],
                "thermal.coating_optics",
                format!("Coating `{id}` must have both absorptivity and emissivity"),
            );
        }
    }
    for (part, coatings) in &t.surface {
        let exists = design.parts.contains_key(part.as_str());
        s.reference(
            &["surface", part],
            exists,
            "ref.part",
            format!("Part `{part}`"),
        );
        for (coating, f) in coatings {
            let exists = t.coating.contains_key(coating);
            s.reference(
                &["surface", part, coating],
                exists,
                "ref.coating",
                format!("Coating `{coating}` ([coating.<name>])"),
            );
            s.fraction(&["surface", part, coating], Some(*f));
        }
        let sum: f64 = coatings.values().sum();
        if (sum - 1.0).abs() > super::SHARE_TOLERANCE {
            s.warning(
                &["surface", part],
                "thermal.coating_share",
                format!(
                    "The coating area shares of part `{part}` must add up to 1 (currently {sum})"
                ),
            );
        }
    }
}

/// 発熱の場所の部品と節点の参照。
fn check_heat_nodes(s: &mut Scope, design: &Design, t: &ThermalSettings) {
    for (part, node) in &t.heat_node {
        let exists = design.parts.contains_key(part.as_str());
        s.reference(
            &["heat_node", part],
            exists,
            "ref.part",
            format!("Part `{part}`"),
        );
        let exists = t.nodes.contains_key(node);
        s.reference(
            &["heat_node", part],
            exists,
            "ref.thermal_node",
            format!("Thermal node `{node}` ([nodes])"),
        );
    }
}
