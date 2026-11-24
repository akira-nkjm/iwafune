//! `analysis/thermal.toml` の検査（環境条件のモード・割合の範囲・節点の部品・表面材と面積割合・発熱の場所・温度差の単位・
//! 明示の結合の節点と値・外殻の放射率）。

use super::Ctx;
use super::settings::{Scope, scope};
use crate::design::{
    AnalysisSystem, Design, ThermalNodes, ThermalSettings, ThermalSubdivideSettings,
};
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
    let nodes = t.node_map(design.parts.keys().map(|id| id.as_str()));
    check_nodes(&mut s, design, t, &nodes);
    check_surfaces(&mut s, design, t);
    check_heat_nodes(&mut s, design, t, &nodes);
    check_couplings(&mut s, t, &nodes);
    check_subdivide(&mut s, design, t, &nodes);
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

/// 節点の部品の参照と、1 つの部品が 2 つの節点に入っていないか、`[nodes]` のキーが `[nodes]` に載っていない部品の ID と
/// 同じ名前でないか（部品は既定で自分の ID の節点になるので、名前がぶつかる）。
fn check_nodes(s: &mut Scope, design: &Design, t: &ThermalSettings, nodes: &ThermalNodes) {
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
    for part in nodes.clashes() {
        s.error(
            &["nodes", part],
            "thermal.node_name_clash",
            format!(
                "Node `{part}` has the name of part `{part}`, which is not one of its parts. Every part not listed in [nodes] is a node named after the part, \
                 so rename the node or list part `{part}` in a node"
            ),
        );
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
fn check_heat_nodes(s: &mut Scope, design: &Design, t: &ThermalSettings, nodes: &ThermalNodes) {
    for (part, node) in &t.heat_node {
        let exists = design.parts.contains_key(part.as_str());
        s.reference(
            &["heat_node", part],
            exists,
            "ref.part",
            format!("Part `{part}`"),
        );
        let exists = nodes.contains(node);
        s.reference(
            &["heat_node", part],
            exists,
            "ref.thermal_node",
            format!("Thermal node `{node}` (a part ID or a [nodes] key)"),
        );
    }
}

/// 板の分け方（`[subdivide.<部品>]`）。部品があること、`[nodes]` でまとめた部品でないこと（升ごとの節点になる部品は
/// 1 部品 1 節点のものだけ）、升の数が 1 以上で上限以下であること。`u`・`v` の片方だけ書いた途中の状態は警告（解析は不足入力にする）。
/// 板の形かどうか（箱の寸法）は部品の値を解決してから決まるので、解析の入力の検査が見る。
fn check_subdivide(s: &mut Scope, design: &Design, t: &ThermalSettings, nodes: &ThermalNodes) {
    for (part, cells) in &t.subdivide {
        let exists = design.parts.contains_key(part.as_str());
        s.reference(
            &["subdivide", part],
            exists,
            "ref.part",
            format!("Part `{part}`"),
        );
        if exists && !nodes.is_per_part(part) {
            s.error(
                &["subdivide", part],
                "thermal.subdivide_lumped",
                format!(
                    "Part `{part}` is lumped into a node by [nodes], so it cannot be subdivided. \
                     Remove it from [nodes] to give it its own node"
                ),
            );
        }
        for (axis, n) in [("u", cells.u), ("v", cells.v)] {
            match n {
                Some(n) if !(1..=ThermalSubdivideSettings::MAX_CELLS).contains(&n) => s.error(
                    &["subdivide", part, axis],
                    "thermal.subdivide_cells",
                    format!(
                        "A plate is split into 1 to {max} cells along each axis (`{n}` is written for `{part}.{axis}`). \
                         More cells make the explicit transient solver take much smaller steps",
                        max = ThermalSubdivideSettings::MAX_CELLS
                    ),
                ),
                Some(_) => {}
                None => s.warning(
                    &["subdivide", part],
                    "thermal.subdivide_incomplete",
                    format!("`[subdivide.{part}]` needs both `u` and `v` (missing `{axis}`)"),
                ),
            }
        }
    }
}

/// 明示の結合（`[conductive.<節点>]`・`[radiative.<節点>]`）と外殻の放射率。
/// 結合は両端が節点であること、同じ節点どうしでないこと、同じ対を向きを変えて 2 度書かないこと、0 以上であること
/// （次元は設定の次元の表が見る）。
fn check_couplings(s: &mut Scope, t: &ThermalSettings, nodes: &ThermalNodes) {
    for (kind, table) in [("conductive", &t.conductive), ("radiative", &t.radiative)] {
        let mut seen: std::collections::BTreeSet<(&str, &str)> = std::collections::BTreeSet::new();
        for (a, row) in table {
            for (b, q) in row {
                let keys = [kind, a.as_str(), b.as_str()];
                for end in [a, b] {
                    let exists = nodes.contains(end);
                    s.reference(
                        &keys,
                        exists,
                        "ref.thermal_node",
                        format!("Thermal node `{end}` (a part ID or a [nodes] key)"),
                    );
                }
                if a == b {
                    s.error(
                        &keys,
                        "thermal.coupling_self",
                        format!("A coupling must join two different nodes (`{a}` is both ends)"),
                    );
                }
                let pair = if a <= b {
                    (a.as_str(), b.as_str())
                } else {
                    (b.as_str(), a.as_str())
                };
                if !seen.insert(pair) {
                    s.error(
                        &keys,
                        "thermal.coupling_duplicate",
                        format!(
                            "The pair `{}` and `{}` is written twice ({kind})",
                            pair.0, pair.1
                        ),
                    );
                }
                if q.value.is_nan() || q.value < 0.0 {
                    s.error(
                        &keys,
                        "setting.range",
                        "Must be a value of at least 0".into(),
                    );
                }
            }
        }
    }
    if let Some(e) = &t.enclosure {
        match e.emissivity {
            Some(v) if !(v > 0.0 && v <= 1.0) => s.error(
                &["enclosure", "emissivity"],
                "setting.range",
                "Must be a value above 0 and at most 1".into(),
            ),
            None => s.warning(
                &["enclosure"],
                "thermal.enclosure_emissivity",
                "The enclosure needs an emissivity (`emissivity`); without it the enclosure radiation is not used"
                    .into(),
            ),
            _ => {}
        }
    }
}
